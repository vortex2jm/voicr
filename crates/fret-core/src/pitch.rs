use serde::{Deserialize, Serialize};

/// Pitch class in 12-tone equal temperament (0 = C, 1 = C#/Db, ..., 11 = B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PitchClass(pub u8);

impl PitchClass {
    /// Creates a normalized pitch class modulo 12.
    pub const fn new(val: u8) -> Self {
        Self(val % 12)
    }

    /// Creates a pitch class from any signed semitone offset.
    pub fn from_semitones(semitones: i32) -> Self {
        Self(semitones.rem_euclid(12) as u8)
    }

    /// Transposes this pitch class by `semitones` modulo 12.
    pub fn transpose(self, semitones: i8) -> Self {
        Self::from_semitones(self.0 as i32 + semitones as i32)
    }

    /// Returns the ascending interval in semitones (0..12) from `self` to `other`.
    pub const fn interval_to(self, other: Self) -> u8 {
        (other.0 + 12 - self.0) % 12
    }

    /// Returns the underlying value in `0..12`.
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

/// MIDI note number (`0..=127`, where `60` = C4 and `69` = A4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MidiNote(pub u8);

impl MidiNote {
    /// Returns the pitch class of this MIDI note.
    pub const fn pitch_class(self) -> PitchClass {
        PitchClass(self.0 % 12)
    }

    /// Returns the scientific pitch octave (C4 = 60 => octave 4).
    pub const fn octave(self) -> i8 {
        (self.0 as i8 / 12) - 1
    }

    /// Converts this MIDI note to frequency in Hz (`A4 = 440 Hz`).
    pub fn frequency_hz(self) -> f64 {
        440.0 * 2.0_f64.powf((self.0 as f64 - 69.0) / 12.0)
    }

    /// Formats the MIDI note with its octave (e.g. `"E2"`, `"C#4"`).
    pub fn format_with_octave(self, prefer_flats: bool) -> String {
        format!(
            "{}{}",
            spell_chromatic(self.pitch_class(), prefer_flats),
            self.octave()
        )
    }
}

/// Accidental preference for enharmonic spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccidentalPreference {
    Sharps,
    Flats,
}

const SHARP_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

const FLAT_NAMES: [&str; 12] = [
    "C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B",
];

const LETTERS: [&str; 7] = ["C", "D", "E", "F", "G", "A", "B"];
const LETTER_SEMITONES: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];

/// Spells a pitch class chromatically using sharps or flats.
pub fn spell_chromatic(pc: PitchClass, prefer_flats: bool) -> &'static str {
    let idx = (pc.0 % 12) as usize;
    if prefer_flats {
        FLAT_NAMES[idx]
    } else {
        SHARP_NAMES[idx]
    }
}

/// Spells a chord tone using diatonic letter step relative to the root when possible,
/// falling back to practical chromatic spelling to avoid double accidentals.
pub fn spell_chord_tone(
    root: PitchClass,
    root_letter: u8,
    semitones: u8,
    degree_steps: u8,
    prefer_flats: bool,
) -> String {
    let pc = root.transpose(semitones as i8);
    let letter_idx = ((root_letter as usize) + (degree_steps as usize)) % 7;
    let natural = LETTER_SEMITONES[letter_idx] as i8;
    let mut diff = (pc.0 as i8) - natural;
    if diff > 6 {
        diff -= 12;
    } else if diff < -6 {
        diff += 12;
    }

    match diff {
        0 => LETTERS[letter_idx].to_string(),
        1 => format!("{}#", LETTERS[letter_idx]),
        -1 => format!("{}b", LETTERS[letter_idx]),
        _ => spell_chromatic(pc, prefer_flats).to_string(),
    }
}

/// Parsed note token metadata from the start of a string slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedNote {
    pub pitch_class: PitchClass,
    pub letter_index: u8,
    pub canonical_name: String,
    pub preference: AccidentalPreference,
    pub bytes_consumed: usize,
}

/// Parses a note name (`A`..`G` with optional `#`, `b`, `♯`, `♭`, `##`, `bb`) from the start of `input`.
pub fn parse_note_prefix(input: &str) -> Option<ParsedNote> {
    let mut chars = input.char_indices();
    let (_, first) = chars.next()?;

    let (letter_char, letter_index, base_pc) = match first.to_ascii_uppercase() {
        'C' => ('C', 0, 0i32),
        'D' => ('D', 1, 2),
        'E' => ('E', 2, 4),
        'F' => ('F', 3, 5),
        'G' => ('G', 4, 7),
        'A' => ('A', 5, 9),
        'B' => ('B', 6, 11),
        _ => return None,
    };

    let mut offset: i32 = 0;
    let mut acc_str = String::new();
    let mut consumed = first.len_utf8();

    for (idx, ch) in chars {
        match ch {
            '#' | '♯' => {
                if offset < 0 || acc_str.len() >= 2 {
                    break;
                }
                offset += 1;
                acc_str.push('#');
                consumed = idx + ch.len_utf8();
            }
            'b' | '♭' => {
                if offset > 0 || acc_str.len() >= 2 {
                    break;
                }
                offset -= 1;
                acc_str.push('b');
                consumed = idx + ch.len_utf8();
            }
            _ => break,
        }
    }

    let pc = PitchClass::from_semitones(base_pc + offset);
    let preference = if offset < 0 {
        AccidentalPreference::Flats
    } else if offset > 0 {
        AccidentalPreference::Sharps
    } else {
        // Natural roots that conventionally lean flat in harmonic spellings (F)
        if letter_char == 'F' {
            AccidentalPreference::Flats
        } else {
            AccidentalPreference::Sharps
        }
    };

    let canonical_name = format!("{}{}", letter_char, acc_str);

    Some(ParsedNote {
        pitch_class: pc,
        letter_index,
        canonical_name,
        preference,
        bytes_consumed: consumed,
    })
}
