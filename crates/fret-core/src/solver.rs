use serde::{Deserialize, Serialize};

use crate::chord::ChordSpec;
use crate::instrument::InstrumentConfig;
use crate::pitch::MidiNote;
use crate::scoring::score_voicing;

/// Search configuration for the biomechanical fretboard solver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchOptions {
    pub min_fret: u8,
    pub max_fret: u8,
    pub max_span: u8,
    pub allow_open_strings: bool,
    pub omit_fifth: bool,
    pub allow_rootless: bool,
    pub pinned_frets: Vec<Option<u8>>,
    pub limit: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            min_fret: 0,
            max_fret: 15,
            max_span: 4,
            allow_open_strings: true,
            omit_fifth: true,
            allow_rootless: false,
            pinned_frets: Vec::new(),
            limit: 25,
        }
    }
}

/// Finger 1 barre spanning `from_string..=to_string` at `fret`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Barre {
    pub fret: u8,
    pub from_string: usize,
    pub to_string: usize,
}

/// Sounded string detail inside a solved voicing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoicingNote {
    pub string_index: usize,
    pub fret: u8,
    pub midi: MidiNote,
    pub note_name: String,
    pub interval: String,
    pub is_root: bool,
    pub is_bass: bool,
    pub finger: Option<u8>,
    pub octave_pair_offset: Option<i8>,
}

/// A biomechanically valid, scored chord voicing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoicingResult {
    pub frets: Vec<Option<u8>>,
    pub fingers: Vec<Option<u8>>,
    pub barre: Option<Barre>,
    pub notes: Vec<VoicingNote>,
    pub min_fret: u8,
    pub max_fret: u8,
    pub score: f64,
}

/// Computes top-ranked playable chord voicings for `chord` on `instrument`.
pub fn solve_chord(
    chord: &ChordSpec,
    instrument: &InstrumentConfig,
    options: &SearchOptions,
) -> Vec<VoicingResult> {
    if instrument.validate().is_err() {
        return Vec::new();
    }

    let num_strings = instrument.num_strings();
    let allow_rootless_effective =
        options.allow_rootless && num_strings == 4 && chord.distinct_tone_count() >= 4;

    let allowed_mask = chord.allowed_pitch_mask();
    let required_mask = chord.required_pitch_mask(options.omit_fifth, allow_rootless_effective);
    let altered_mask = chord.altered_pool_mask();
    let bass_mask = chord
        .slash_bass
        .map(|b| 1u16 << b.as_u8())
        .unwrap_or(0u16);
    let any_valid_mask = allowed_mask | bass_mask;

    let max_fret = options.max_fret.min(instrument.num_frets);
    let min_fret = options.min_fret.min(max_fret);
    let min_sounded_strings = if chord.distinct_tone_count() >= 3 {
        3
    } else {
        2
    };

    // Build per-string candidate fret options
    let mut candidates: Vec<Vec<Option<u8>>> = Vec::with_capacity(num_strings);
    for (s, open_midi) in instrument.tuning.iter().enumerate() {
        let mut opts = Vec::new();
        if let Some(Some(pinned)) = options.pinned_frets.get(s).copied() {
            if pinned <= instrument.num_frets {
                let pc = MidiNote(open_midi.0.saturating_add(pinned)).pitch_class();
                if (any_valid_mask & (1u16 << pc.as_u8())) != 0 {
                    opts.push(Some(pinned));
                }
            }
            candidates.push(opts);
            continue;
        }

        if options.allow_open_strings {
            let pc = open_midi.pitch_class();
            if (any_valid_mask & (1u16 << pc.as_u8())) != 0 {
                opts.push(Some(0));
            }
        }

        let start_fret = min_fret.max(1);
        for fret in start_fret..=max_fret {
            let pc = MidiNote(open_midi.0.saturating_add(fret)).pitch_class();
            if (any_valid_mask & (1u16 << pc.as_u8())) != 0 {
                opts.push(Some(fret));
            }
        }

        // Muted string option
        opts.push(None);
        candidates.push(opts);
    }

    struct SearchCtx<'a> {
        chord: &'a ChordSpec,
        instrument: &'a InstrumentConfig,
        options: &'a SearchOptions,
        candidates: &'a [Vec<Option<u8>>],
        allowed_mask: u16,
        required_mask: u16,
        altered_mask: u16,
        min_sounded_strings: usize,
        results: Vec<VoicingResult>,
        max_raw_results: usize,
    }

    #[allow(clippy::too_many_arguments)]
    fn backtrack(
        ctx: &mut SearchCtx<'_>,
        s: usize,
        current_frets: &mut Vec<Option<u8>>,
        collected_mask: u16,
        min_fretted: Option<u8>,
        max_fretted: Option<u8>,
        lowest_sounded: Option<usize>,
        sounded_count: usize,
    ) {
        if ctx.results.len() >= ctx.max_raw_results {
            return;
        }

        let num_strings = ctx.instrument.num_strings();
        if s == num_strings {
            if sounded_count < ctx.min_sounded_strings {
                return;
            }
            if (collected_mask & ctx.required_mask) != ctx.required_mask {
                return;
            }
            if ctx.chord.is_altered && (collected_mask & ctx.altered_mask) == 0 {
                return;
            }

            let Some((fingers, barre)) = assign_fingers_and_barre(current_frets) else {
                return;
            };

            let mut notes = Vec::with_capacity(sounded_count);
            for (idx, fret_opt) in current_frets.iter().enumerate() {
                if let Some(fret) = *fret_opt {
                    let midi = MidiNote(ctx.instrument.tuning[idx].0 + fret);
                    let pc = midi.pitch_class();
                    let is_bass = lowest_sounded == Some(idx);
                    let octave_pair_offset = ctx
                        .instrument
                        .course_octave_offsets
                        .get(idx)
                        .copied()
                        .flatten();
                    notes.push(VoicingNote {
                        string_index: idx,
                        fret,
                        midi,
                        note_name: ctx.chord.spell_note(pc),
                        interval: ctx.chord.interval_label_for(pc),
                        is_root: pc == ctx.chord.root,
                        is_bass,
                        finger: fingers[idx],
                        octave_pair_offset,
                    });
                }
            }

            let mut voicing = VoicingResult {
                frets: current_frets.clone(),
                fingers,
                barre,
                notes,
                min_fret: min_fretted.unwrap_or(0),
                max_fret: max_fretted.unwrap_or(0),
                score: 0.0,
            };
            voicing.score = score_voicing(&voicing, ctx.chord, ctx.instrument, ctx.options);
            ctx.results.push(voicing);
            return;
        }

        let remaining_after_this = num_strings - (s + 1);

        for &choice in &ctx.candidates[s] {
            let mut next_mask = collected_mask;
            let mut next_min = min_fretted;
            let mut next_max = max_fretted;
            let mut next_lowest = lowest_sounded;
            let mut next_sounded = sounded_count;

            if let Some(fret) = choice {
                let pc = MidiNote(ctx.instrument.tuning[s].0 + fret).pitch_class();
                let pc_bit = 1u16 << pc.as_u8();

                if next_lowest.is_none() {
                    if let Some(slash_bass) = ctx.chord.slash_bass
                        && pc != slash_bass
                    {
                        continue;
                    }
                    next_lowest = Some(s);
                } else if (ctx.allowed_mask & pc_bit) == 0 {
                    // Non-chord slash bass may only appear as the lowest sounded note
                    continue;
                }

                if fret > 0 {
                    let new_min = next_min.map_or(fret, |m| m.min(fret));
                    let new_max = next_max.map_or(fret, |m| m.max(fret));
                    if new_max - new_min > ctx.options.max_span {
                        continue;
                    }
                    next_min = Some(new_min);
                    next_max = Some(new_max);
                }

                next_mask |= pc_bit;
                next_sounded += 1;
            }

            // Early prune: not enough strings left to satisfy remaining required tones
            let missing_required = (ctx.required_mask & !next_mask).count_ones() as usize;
            if missing_required > remaining_after_this {
                continue;
            }

            // Early prune: not enough strings left to meet minimum sounded count
            if next_sounded + remaining_after_this < ctx.min_sounded_strings {
                continue;
            }

            current_frets.push(choice);
            backtrack(
                ctx,
                s + 1,
                current_frets,
                next_mask,
                next_min,
                next_max,
                next_lowest,
                next_sounded,
            );
            current_frets.pop();
        }
    }

    let mut ctx = SearchCtx {
        chord,
        instrument,
        options,
        candidates: &candidates,
        allowed_mask,
        required_mask,
        altered_mask,
        min_sounded_strings,
        results: Vec::new(),
        max_raw_results: 10_000,
    };

    let mut current_frets = Vec::with_capacity(num_strings);
    backtrack(&mut ctx, 0, &mut current_frets, 0, None, None, None, 0);

    ctx.results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.min_fret.cmp(&b.min_fret))
            .then_with(|| a.frets.cmp(&b.frets))
    });

    if options.limit > 0 && ctx.results.len() > options.limit {
        ctx.results.truncate(options.limit);
    }

    ctx.results
}

/// Simulates left-hand barre formation and assigns fingers `1..=4`.
/// Returns `None` if the fingering requires more than 4 fingers.
pub fn assign_fingers_and_barre(frets: &[Option<u8>]) -> Option<(Vec<Option<u8>>, Option<Barre>)> {
    let n = frets.len();
    let mut fingers = vec![None; n];

    let min_fretted = frets.iter().filter_map(|&f| f).filter(|&f| f > 0).min();
    let Some(min_f) = min_fretted else {
        // All sounded strings are open (0) or muted (None)
        return Some((fingers, None));
    };

    let at_min: Vec<usize> = frets
        .iter()
        .enumerate()
        .filter_map(|(i, &f)| if f == Some(min_f) { Some(i) } else { None })
        .collect();

    if at_min.len() >= 2 {
        let first = at_min[0];
        let last = at_min[at_min.len() - 1];
        let has_open_or_lower = (first..=last).any(|i| matches!(frets[i], Some(f) if f < min_f));

        if !has_open_or_lower {
            let mut others: Vec<(u8, usize)> = frets
                .iter()
                .enumerate()
                .filter_map(|(i, &f)| match f {
                    Some(v) if v > min_f => Some((v, i)),
                    _ => None,
                })
                .collect();

            if others.len() > 3 {
                return None;
            }

            for &i in &at_min {
                fingers[i] = Some(1);
            }

            others.sort_unstable();
            for (offset, &(_, string_idx)) in others.iter().enumerate() {
                fingers[string_idx] = Some((offset + 2) as u8);
            }

            let barre = Some(Barre {
                fret: min_f,
                from_string: first,
                to_string: last,
            });
            return Some((fingers, barre));
        }
    }

    // No barre: every fretted note consumes 1 finger
    let mut fretted_positions: Vec<(u8, usize)> = frets
        .iter()
        .enumerate()
        .filter_map(|(i, &f)| match f {
            Some(v) if v > 0 => Some((v, i)),
            _ => None,
        })
        .collect();

    if fretted_positions.len() > 4 {
        return None;
    }

    fretted_positions.sort_unstable();
    for (idx, &(_, string_idx)) in fretted_positions.iter().enumerate() {
        fingers[string_idx] = Some((idx + 1) as u8);
    }

    Some((fingers, None))
}
