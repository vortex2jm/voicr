use crate::chord::{ChordSpec, ToneRole};
use crate::instrument::InstrumentConfig;
use crate::solver::{SearchOptions, VoicingResult};

/// Scores a valid voicing from most natural/playable (higher) to more exotic (lower).
pub fn score_voicing(
    voicing: &VoicingResult,
    chord: &ChordSpec,
    instrument: &InstrumentConfig,
    _options: &SearchOptions,
) -> f64 {
    let mut score = 100.0_f64;
    let num_strings = instrument.num_strings();

    let sounded_indices: Vec<usize> = voicing
        .frets
        .iter()
        .enumerate()
        .filter_map(|(i, f)| f.map(|_| i))
        .collect();

    let sounded_count = sounded_indices.len();
    if sounded_count == 0 {
        return -1000.0;
    }

    let lowest_sounded = sounded_indices[0];
    let highest_sounded = sounded_indices[sounded_count - 1];

    // 1. Penalize internal string skipping (muted strings between lowest and highest sounded)
    let mut internal_skips = 0usize;
    for i in (lowest_sounded + 1)..highest_sounded {
        if voicing.frets[i].is_none() {
            internal_skips += 1;
        }
    }
    if internal_skips > 0 {
        score -= (internal_skips as f64) * 20.0;
        if internal_skips >= 2 {
            score -= 10.0;
        }
    }

    // Slight penalty for muted treble strings so full strummable voicings beat truncated ones
    let trailing_muted = (num_strings - 1) - highest_sounded;
    score -= (trailing_muted as f64) * 3.5;

    // Slight penalty for excessive leading muted bass strings (beyond 1 on 6-string, etc.)
    if lowest_sounded > 0 {
        score -= (lowest_sounded as f64) * 1.5;
    }

    // 2. Reward fuller string coverage relative to instrument size
    let coverage = sounded_count as f64 / num_strings as f64;
    score += coverage * 16.0;

    // 3. Fret span penalty: compact shapes (0..=3) strongly preferred over 4-5 fret stretches
    let span = voicing.max_fret.saturating_sub(voicing.min_fret);
    match span {
        0 => score += 6.0,
        1 => score += 5.0,
        2 => score += 4.0,
        3 => score += 0.0,
        4 => score -= 11.0,
        _ => score -= 24.0 + ((span - 5) as f64) * 8.0,
    }

    // 4. Open string bonus (especially strong in lower neck region)
    let open_count = voicing.frets.iter().filter(|f| **f == Some(0)).count();
    if voicing.min_fret <= 4 {
        score += (open_count as f64) * 5.5;
    } else {
        score += (open_count as f64) * 1.5;
    }

    // 5. Gentle preference for lower positions on the neck (playable everyday voicings first)
    if voicing.max_fret > 0 {
        let avg_fret = voicing
            .frets
            .iter()
            .filter_map(|f| *f)
            .filter(|&f| f > 0)
            .map(|f| f as f64)
            .sum::<f64>()
            / (voicing.frets.iter().filter(|f| matches!(f, Some(v) if *v > 0)).count().max(1)
                as f64);
        score -= avg_fret * 1.35;
    }

    // 6. Bass note quality: reward root in the bass when no slash bass is specified
    if chord.slash_bass.is_none()
        && let Some(bass_note) = voicing.notes.first()
        && bass_note.midi.pitch_class() == chord.root
    {
        // Slightly gentler weight on 4-string instruments where inversions are idiomatic
        score += if num_strings <= 4 { 7.5 } else { 14.0 };
    }

    // 7. Voice leading / spacing & clarity (avoid low muddy intervals and awkward inversions/unisons)
    let mut prev_midi: Option<u8> = None;
    for note in &voicing.notes {
        if let Some(p) = prev_midi {
            let diff = note.midi.0 as i16 - p as i16;
            if diff < 0 {
                // Out-of-order descending pitch across strings (re-entrant feel or odd inversion)
                score -= 5.0;
            } else if diff == 0 {
                // Exact unison between adjacent strings
                score -= 3.0;
            } else if diff <= 2 && note.midi.0 < 52 {
                // Muddy minor/major 2nd in low bass register
                score -= 8.0;
            } else if (3..=12).contains(&diff) {
                // Clean, balanced harmonic spacing
                score += 1.2;
            }
        }
        prev_midi = Some(note.midi.0);
    }

    // 8. Finger economy & barre ergonomics
    let finger_count = voicing.fingers.iter().filter(|f| f.is_some()).count();
    if voicing.barre.is_some() {
        // Barre takes finger 1 across multiple strings, slight ergonomic cost vs open chord
        score -= 2.0;
    }
    if finger_count <= 3 {
        score += 2.5;
    }

    // 9. Bonus for including optional tones (like natural 5th) when it fits comfortably
    let optional_mask = chord
        .tones
        .iter()
        .filter(|t| t.role == ToneRole::Optional)
        .fold(0u16, |acc, t| acc | (1u16 << t.pitch_class.as_u8()));
    if optional_mask != 0 {
        let present_mask = voicing
            .notes
            .iter()
            .fold(0u16, |acc, n| acc | (1u16 << n.midi.pitch_class().as_u8()));
        if (present_mask & optional_mask) != 0 {
            score += 2.0;
        }
    }

    (score * 100.0).round() / 100.0
}
