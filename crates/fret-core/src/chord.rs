use serde::{Deserialize, Serialize};

use crate::pitch::{
    parse_note_prefix, spell_chord_tone, spell_chromatic, AccidentalPreference, PitchClass,
};

/// Role of a pitch class inside a parsed chord specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToneRole {
    /// Chord root (required unless `allow_rootless` applies on 4-string instruments).
    Root,
    /// Essential chord tone (3rd/sus, 6th/7th, explicit tensions/alterations).
    Required,
    /// Perfect 5th (7 semitones) that may be omitted when `omit_fifth == true`.
    Optional,
    /// Altered color tones for `7alt` chords (at least one must sound).
    AlteredPool,
}

/// A single harmonic tone belonging to a chord.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChordTone {
    pub pitch_class: PitchClass,
    pub semitones: u8,
    pub interval_label: String,
    pub note_name: String,
    pub role: ToneRole,
}

/// Complete harmonic specification produced by parsing a chord symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChordSpec {
    pub symbol: String,
    pub root: PitchClass,
    pub root_name: String,
    pub slash_bass: Option<PitchClass>,
    pub slash_bass_name: Option<String>,
    pub tones: Vec<ChordTone>,
    pub prefer_flats: bool,
    pub is_altered: bool,
}

impl ChordSpec {
    /// Returns the bitmask of all pitch classes allowed anywhere on the fretboard
    /// (excluding a non-chord slash bass, which is enforced on the lowest sounded string).
    pub fn allowed_pitch_mask(&self) -> u16 {
        self.tones
            .iter()
            .fold(0u16, |acc, t| acc | (1u16 << t.pitch_class.as_u8()))
    }

    /// Returns the bitmask of pitch classes required under the given search rules.
    pub fn required_pitch_mask(&self, omit_fifth: bool, allow_rootless_effective: bool) -> u16 {
        let mut mask = 0u16;
        for tone in &self.tones {
            let required = match tone.role {
                ToneRole::Root => !allow_rootless_effective,
                ToneRole::Required => true,
                ToneRole::Optional => !omit_fifth,
                ToneRole::AlteredPool => false,
            };
            if required {
                mask |= 1u16 << tone.pitch_class.as_u8();
            }
        }
        mask
    }

    /// Returns the bitmask of altered pool tones (used for `alt` chords).
    pub fn altered_pool_mask(&self) -> u16 {
        self.tones
            .iter()
            .filter(|t| t.role == ToneRole::AlteredPool)
            .fold(0u16, |acc, t| acc | (1u16 << t.pitch_class.as_u8()))
    }

    /// Number of distinct chord pitch classes (excluding external slash bass).
    pub fn distinct_tone_count(&self) -> usize {
        self.allowed_pitch_mask().count_ones() as usize
    }

    /// Looks up the interval label for a pitch class in this chord.
    pub fn interval_label_for(&self, pc: PitchClass) -> String {
        if let Some(tone) = self.tones.iter().find(|t| t.pitch_class == pc) {
            tone.interval_label.clone()
        } else if self.slash_bass == Some(pc) {
            "Bass".to_string()
        } else {
            "?".to_string()
        }
    }

    /// Spells a pitch class consistently with this chord's harmonic context.
    pub fn spell_note(&self, pc: PitchClass) -> String {
        if let Some(tone) = self.tones.iter().find(|t| t.pitch_class == pc) {
            return tone.note_name.clone();
        }
        if self.slash_bass == Some(pc)
            && let Some(name) = &self.slash_bass_name
        {
            return name.clone();
        }
        spell_chromatic(pc, self.prefer_flats).to_string()
    }
}

/// Error returned when a chord symbol cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChordParseError {
    pub message: String,
}

impl std::fmt::Display for ChordParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChordParseError {}

#[derive(Debug, Clone)]
struct ToneBuilder {
    semitones: u8,
    degree_steps: u8,
    label: String,
    role: ToneRole,
}

fn upsert_tone(list: &mut Vec<ToneBuilder>, tone: ToneBuilder) {
    if let Some(existing) = list.iter_mut().find(|t| t.semitones == tone.semitones) {
        *existing = tone;
    } else {
        list.push(tone);
    }
}

fn remove_by_degree(list: &mut Vec<ToneBuilder>, degree_steps: u8) {
    list.retain(|t| t.degree_steps != degree_steps);
}

type SlashBassSplit<'a> = (&'a str, Option<(PitchClass, String)>);

fn split_slash_bass(body: &str) -> Result<SlashBassSplit<'_>, ChordParseError> {
    if let Some(idx) = body.rfind('/') {
        let after = &body[idx + 1..];
        if after.is_empty() {
            return Err(ChordParseError {
                message: "Baixo invertido incompleto após '/'".to_string(),
            });
        }
        // Distinguish "6/9" from "/E" or "/G#"
        if after.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
            let parsed = parse_note_prefix(after).ok_or_else(|| ChordParseError {
                message: format!("Nota de baixo inválida: '/{}'", after),
            })?;
            if parsed.bytes_consumed != after.len() {
                return Err(ChordParseError {
                    message: format!("Baixo inválido após '/': '{}'", after),
                });
            }
            return Ok((&body[..idx], Some((parsed.pitch_class, parsed.canonical_name))));
        }
    }
    Ok((body, None))
}

/// Parses a chord symbol such as `"Cmaj9(#11)"`, `"F#m7(b5)/C"`, `"G7(b9)"`, `"C6/9"`.
pub fn parse_chord(input: &str) -> Result<ChordSpec, ChordParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ChordParseError {
            message: "Digite uma cifra (ex: Cmaj7, F#m7(b5)/C)".to_string(),
        });
    }

    let root_parsed = parse_note_prefix(trimmed).ok_or_else(|| ChordParseError {
        message: format!("Nota tônica inválida em '{}'", trimmed),
    })?;

    let rest = &trimmed[root_parsed.bytes_consumed..];
    let (quality_and_mods, slash_bass_info) = split_slash_bass(rest)?;

    let mut tones: Vec<ToneBuilder> = vec![ToneBuilder {
        semitones: 0,
        degree_steps: 0,
        label: "R".to_string(),
        role: ToneRole::Root,
    }];

    let mut is_altered = false;
    let mut flat_bias = root_parsed.preference == AccidentalPreference::Flats;

    // Match base quality prefix from longest/most-specific to shortest
    let (mut cursor, norm_quality) = parse_base_quality(
        quality_and_mods,
        &mut tones,
        &mut is_altered,
        &mut flat_bias,
    )?;

    // Parse remaining modifier tokens (e.g. (b5), (#11), b9, add9, etc.)
    let mut norm_mods = String::new();
    while !cursor.is_empty() {
        // Skip whitespace, parens, commas, slashes between modifiers
        if let Some(ch) = cursor.chars().next()
            && (ch.is_whitespace() || ch == '(' || ch == ')' || ch == ',' || ch == '/')
        {
            cursor = &cursor[ch.len_utf8()..];
            continue;
        }

        let (next_cursor, mod_display) =
            parse_modifier_token(cursor, &mut tones, &mut is_altered, &mut flat_bias)?;
        if !mod_display.is_empty() {
            norm_mods.push_str(&format!("({})", mod_display));
        }
        cursor = next_cursor;
    }

    // Check for any accidental sharp/flat preference from tones
    let mut final_tones = Vec::with_capacity(tones.len());
    for t in tones {
        let pc = root_parsed.pitch_class.transpose(t.semitones as i8);
        let note_name = if t.semitones == 0 {
            root_parsed.canonical_name.clone()
        } else {
            spell_chord_tone(
                root_parsed.pitch_class,
                root_parsed.letter_index,
                t.semitones,
                t.degree_steps,
                flat_bias,
            )
        };
        final_tones.push(ChordTone {
            pitch_class: pc,
            semitones: t.semitones,
            interval_label: t.label,
            note_name,
            role: t.role,
        });
    }

    let (slash_bass, slash_bass_name) = match slash_bass_info {
        Some((pc, name)) => (Some(pc), Some(name)),
        None => (None, None),
    };

    let mut symbol = format!("{}{}{}", root_parsed.canonical_name, norm_quality, norm_mods);
    if let Some(ref bass_name) = slash_bass_name {
        symbol.push('/');
        symbol.push_str(bass_name);
    }

    Ok(ChordSpec {
        symbol,
        root: root_parsed.pitch_class,
        root_name: root_parsed.canonical_name,
        slash_bass,
        slash_bass_name,
        tones: final_tones,
        prefer_flats: flat_bias,
        is_altered,
    })
}

fn add_fifth(tones: &mut Vec<ToneBuilder>, required: bool) {
    upsert_tone(
        tones,
        ToneBuilder {
            semitones: 7,
            degree_steps: 4,
            label: "5".to_string(),
            role: if required {
                ToneRole::Required
            } else {
                ToneRole::Optional
            },
        },
    );
}

fn add_major_third(tones: &mut Vec<ToneBuilder>, role: ToneRole) {
    upsert_tone(
        tones,
        ToneBuilder {
            semitones: 4,
            degree_steps: 2,
            label: "3".to_string(),
            role,
        },
    );
}

fn add_minor_third(tones: &mut Vec<ToneBuilder>) {
    upsert_tone(
        tones,
        ToneBuilder {
            semitones: 3,
            degree_steps: 2,
            label: "b3".to_string(),
            role: ToneRole::Required,
        },
    );
}

fn add_alt_pool(tones: &mut Vec<ToneBuilder>) {
    upsert_tone(
        tones,
        ToneBuilder {
            semitones: 1,
            degree_steps: 1,
            label: "b9".to_string(),
            role: ToneRole::AlteredPool,
        },
    );
    upsert_tone(
        tones,
        ToneBuilder {
            semitones: 3,
            degree_steps: 1,
            label: "#9".to_string(),
            role: ToneRole::AlteredPool,
        },
    );
    upsert_tone(
        tones,
        ToneBuilder {
            semitones: 6,
            degree_steps: 4,
            label: "b5".to_string(),
            role: ToneRole::AlteredPool,
        },
    );
    upsert_tone(
        tones,
        ToneBuilder {
            semitones: 8,
            degree_steps: 4,
            label: "#5".to_string(),
            role: ToneRole::AlteredPool,
        },
    );
}

fn parse_base_quality<'a>(
    input: &'a str,
    tones: &mut Vec<ToneBuilder>,
    is_altered: &mut bool,
    flat_bias: &mut bool,
) -> Result<(&'a str, &'static str), ChordParseError> {
    // Minor-major 7th variants
    for prefix in [
        "m(maj7)",
        "m(M7)",
        "m(7M)",
        "min(maj7)",
        "mmaj7",
        "mMaj7",
        "minmaj7",
        "mΔ7",
        "mΔ",
        "-(maj7)",
    ] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_minor_third(tones);
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 11,
                    degree_steps: 6,
                    label: "7".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "m(maj7)"));
        }
    }

    // Half-diminished variants
    for prefix in ["m7(b5)", "m7b5", "min7(b5)", "min7b5", "ø7", "ø"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            *flat_bias = true;
            add_minor_third(tones);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 6,
                    degree_steps: 4,
                    label: "b5".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 10,
                    degree_steps: 6,
                    label: "b7".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "m7(b5)"));
        }
    }

    // Diminished 7th
    for prefix in ["dim7", "°7"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            *flat_bias = true;
            add_minor_third(tones);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 6,
                    degree_steps: 4,
                    label: "b5".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 9,
                    degree_steps: 6,
                    label: "bb7".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "dim7"));
        }
    }

    // Diminished triad
    for prefix in ["dim", "°"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            *flat_bias = true;
            add_minor_third(tones);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 6,
                    degree_steps: 4,
                    label: "b5".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "dim"));
        }
    }

    // Major 13 / 11 / 9 / 7
    for prefix in ["maj13", "M13", "Δ13", "13M"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_major_third(tones, ToneRole::Required);
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 11,
                    degree_steps: 6,
                    label: "7".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 9,
                    degree_steps: 5,
                    label: "13".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "maj13"));
        }
    }

    for prefix in ["maj11", "M11", "Δ11", "11M"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_major_third(tones, ToneRole::Required);
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 11,
                    degree_steps: 6,
                    label: "7".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 5,
                    degree_steps: 3,
                    label: "11".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "maj11"));
        }
    }

    for prefix in ["maj9", "M9", "Δ9", "9M", "7M(9)", "maj7(9)"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_major_third(tones, ToneRole::Required);
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 11,
                    degree_steps: 6,
                    label: "7".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 2,
                    degree_steps: 1,
                    label: "9".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "maj9"));
        }
    }

    for prefix in ["maj7", "M7", "7M", "Δ7", "Δ"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_major_third(tones, ToneRole::Required);
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 11,
                    degree_steps: 6,
                    label: "7".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "maj7"));
        }
    }

    // Minor extensions
    for prefix in ["min13", "m13", "-13"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_minor_third(tones);
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 10,
                    degree_steps: 6,
                    label: "b7".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 9,
                    degree_steps: 5,
                    label: "13".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "m13"));
        }
    }

    for prefix in ["min11", "m11", "-11"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_minor_third(tones);
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 10,
                    degree_steps: 6,
                    label: "b7".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 5,
                    degree_steps: 3,
                    label: "11".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "m11"));
        }
    }

    for prefix in ["min9", "m9", "-9"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_minor_third(tones);
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 10,
                    degree_steps: 6,
                    label: "b7".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 2,
                    degree_steps: 1,
                    label: "9".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "m9"));
        }
    }

    for prefix in ["min7", "m7", "-7"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_minor_third(tones);
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 10,
                    degree_steps: 6,
                    label: "b7".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "m7"));
        }
    }

    for prefix in ["min6/9", "m6/9", "m69"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_minor_third(tones);
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 9,
                    degree_steps: 5,
                    label: "6".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 2,
                    degree_steps: 1,
                    label: "9".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "m6/9"));
        }
    }

    for prefix in ["min6", "m6", "-6"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_minor_third(tones);
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 9,
                    degree_steps: 5,
                    label: "6".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "m6"));
        }
    }

    for prefix in ["min", "m", "-"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            // Do not mistake "-5" or "-9" modifier on major chord for minor triad
            if prefix == "-" && rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                break;
            }
            add_minor_third(tones);
            add_fifth(tones, true);
            return Ok((rest, "m"));
        }
    }

    // Augmented
    for prefix in ["aug7", "+7"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_major_third(tones, ToneRole::Required);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 8,
                    degree_steps: 4,
                    label: "#5".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 10,
                    degree_steps: 6,
                    label: "b7".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "7(#5)"));
        }
    }

    for prefix in ["aug", "+"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            if prefix == "+" && rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                break;
            }
            add_major_third(tones, ToneRole::Required);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 8,
                    degree_steps: 4,
                    label: "#5".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "aug"));
        }
    }

    // Suspended chords
    for prefix in ["13sus4", "13sus"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 5,
                    degree_steps: 3,
                    label: "4".to_string(),
                    role: ToneRole::Required,
                },
            );
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 10,
                    degree_steps: 6,
                    label: "b7".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 9,
                    degree_steps: 5,
                    label: "13".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "13sus4"));
        }
    }

    for prefix in ["9sus4", "9sus"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 5,
                    degree_steps: 3,
                    label: "4".to_string(),
                    role: ToneRole::Required,
                },
            );
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 10,
                    degree_steps: 6,
                    label: "b7".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 2,
                    degree_steps: 1,
                    label: "9".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "9sus4"));
        }
    }

    for prefix in ["7sus4", "7sus"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 5,
                    degree_steps: 3,
                    label: "4".to_string(),
                    role: ToneRole::Required,
                },
            );
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 10,
                    degree_steps: 6,
                    label: "b7".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "7sus4"));
        }
    }

    for prefix in ["sus4", "sus"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 5,
                    degree_steps: 3,
                    label: "4".to_string(),
                    role: ToneRole::Required,
                },
            );
            add_fifth(tones, true);
            return Ok((rest, "sus4"));
        }
    }

    if let Some(rest) = input.strip_prefix("sus2") {
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 2,
                degree_steps: 1,
                label: "2".to_string(),
                role: ToneRole::Required,
            },
        );
        add_fifth(tones, true);
        return Ok((rest, "sus2"));
    }

    // 6/9 and 6
    for prefix in ["6/9", "69"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_major_third(tones, ToneRole::Required);
            add_fifth(tones, false);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 9,
                    degree_steps: 5,
                    label: "6".to_string(),
                    role: ToneRole::Required,
                },
            );
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 2,
                    degree_steps: 1,
                    label: "9".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "6/9"));
        }
    }

    if let Some(rest) = input.strip_prefix('6') {
        add_major_third(tones, ToneRole::Required);
        add_fifth(tones, false);
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 9,
                degree_steps: 5,
                label: "6".to_string(),
                role: ToneRole::Required,
            },
        );
        return Ok((rest, "6"));
    }

    // Altered dominant
    for prefix in ["7alt", "alt"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            *is_altered = true;
            *flat_bias = true;
            add_major_third(tones, ToneRole::Required);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 10,
                    degree_steps: 6,
                    label: "b7".to_string(),
                    role: ToneRole::Required,
                },
            );
            add_alt_pool(tones);
            return Ok((rest, "7alt"));
        }
    }

    // Dominant extensions
    if let Some(rest) = input.strip_prefix("13") {
        add_major_third(tones, ToneRole::Required);
        add_fifth(tones, false);
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 10,
                degree_steps: 6,
                label: "b7".to_string(),
                role: ToneRole::Required,
            },
        );
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 9,
                degree_steps: 5,
                label: "13".to_string(),
                role: ToneRole::Required,
            },
        );
        return Ok((rest, "13"));
    }

    if let Some(rest) = input.strip_prefix("11") {
        add_major_third(tones, ToneRole::Optional);
        add_fifth(tones, false);
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 10,
                degree_steps: 6,
                label: "b7".to_string(),
                role: ToneRole::Required,
            },
        );
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 5,
                degree_steps: 3,
                label: "11".to_string(),
                role: ToneRole::Required,
            },
        );
        return Ok((rest, "11"));
    }

    if let Some(rest) = input.strip_prefix('9') {
        add_major_third(tones, ToneRole::Required);
        add_fifth(tones, false);
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 10,
                degree_steps: 6,
                label: "b7".to_string(),
                role: ToneRole::Required,
            },
        );
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 2,
                degree_steps: 1,
                label: "9".to_string(),
                role: ToneRole::Required,
            },
        );
        return Ok((rest, "9"));
    }

    if let Some(rest) = input.strip_prefix('7') {
        add_major_third(tones, ToneRole::Required);
        add_fifth(tones, false);
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 10,
                degree_steps: 6,
                label: "b7".to_string(),
                role: ToneRole::Required,
            },
        );
        return Ok((rest, "7"));
    }

    if let Some(rest) = input.strip_prefix('5') {
        add_fifth(tones, true);
        return Ok((rest, "5"));
    }

    // Explicit "maj" or "M" triad
    for prefix in ["maj", "M"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            add_major_third(tones, ToneRole::Required);
            add_fifth(tones, true);
            return Ok((rest, ""));
        }
    }

    // Default major triad (empty quality or followed directly by modifiers like Cadd9 / C(b5))
    add_major_third(tones, ToneRole::Required);
    add_fifth(tones, true);
    Ok((input, ""))
}

fn parse_modifier_token<'a>(
    input: &'a str,
    tones: &mut Vec<ToneBuilder>,
    is_altered: &mut bool,
    flat_bias: &mut bool,
) -> Result<(&'a str, &'static str), ChordParseError> {
    if let Some(rest) = input.strip_prefix("alt") {
        *is_altered = true;
        *flat_bias = true;
        remove_by_degree(tones, 4);
        add_alt_pool(tones);
        return Ok((rest, "alt"));
    }

    for prefix in ["sus4", "sus"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            remove_by_degree(tones, 2);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 5,
                    degree_steps: 3,
                    label: "4".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "sus4"));
        }
    }

    if let Some(rest) = input.strip_prefix("sus2") {
        remove_by_degree(tones, 2);
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 2,
                degree_steps: 1,
                label: "2".to_string(),
                role: ToneRole::Required,
            },
        );
        return Ok((rest, "sus2"));
    }

    for prefix in ["add13", "add6"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 9,
                    degree_steps: 5,
                    label: "13".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "add13"));
        }
    }

    for prefix in ["add11", "add4"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 5,
                    degree_steps: 3,
                    label: "11".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "add11"));
        }
    }

    for prefix in ["add9", "add2"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 2,
                    degree_steps: 1,
                    label: "9".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "add9"));
        }
    }

    for prefix in ["no5", "omit5"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            remove_by_degree(tones, 4);
            return Ok((rest, "no5"));
        }
    }

    for prefix in ["no3", "omit3"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            remove_by_degree(tones, 2);
            return Ok((rest, "no3"));
        }
    }

    for prefix in ["#11", "+11"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 6,
                    degree_steps: 3,
                    label: "#11".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "#11"));
        }
    }

    for prefix in ["b13", "-13"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            *flat_bias = true;
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 8,
                    degree_steps: 5,
                    label: "b13".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "b13"));
        }
    }

    if let Some(rest) = input.strip_prefix("13") {
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 9,
                degree_steps: 5,
                label: "13".to_string(),
                role: ToneRole::Required,
            },
        );
        return Ok((rest, "13"));
    }

    if let Some(rest) = input.strip_prefix("11") {
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 5,
                degree_steps: 3,
                label: "11".to_string(),
                role: ToneRole::Required,
            },
        );
        return Ok((rest, "11"));
    }

    for prefix in ["b9", "-9"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            *flat_bias = true;
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 1,
                    degree_steps: 1,
                    label: "b9".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "b9"));
        }
    }

    for prefix in ["#9", "+9"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 3,
                    degree_steps: 1,
                    label: "#9".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "#9"));
        }
    }

    if let Some(rest) = input.strip_prefix('9') {
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 2,
                degree_steps: 1,
                label: "9".to_string(),
                role: ToneRole::Required,
            },
        );
        return Ok((rest, "9"));
    }

    for prefix in ["b5", "-5"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            *flat_bias = true;
            remove_by_degree(tones, 4);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 6,
                    degree_steps: 4,
                    label: "b5".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "b5"));
        }
    }

    for prefix in ["#5", "+5", "+"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            remove_by_degree(tones, 4);
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 8,
                    degree_steps: 4,
                    label: "#5".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "#5"));
        }
    }

    for prefix in ["maj7", "M7", "7M"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            upsert_tone(
                tones,
                ToneBuilder {
                    semitones: 11,
                    degree_steps: 6,
                    label: "7".to_string(),
                    role: ToneRole::Required,
                },
            );
            return Ok((rest, "maj7"));
        }
    }

    if let Some(rest) = input.strip_prefix("b7") {
        *flat_bias = true;
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 10,
                degree_steps: 6,
                label: "b7".to_string(),
                role: ToneRole::Required,
            },
        );
        return Ok((rest, "b7"));
    }

    if let Some(rest) = input.strip_prefix('6') {
        upsert_tone(
            tones,
            ToneBuilder {
                semitones: 9,
                degree_steps: 5,
                label: "6".to_string(),
                role: ToneRole::Required,
            },
        );
        return Ok((rest, "6"));
    }

    Err(ChordParseError {
        message: format!("Modificador de cifra não reconhecido: '{}'", input),
    })
}
