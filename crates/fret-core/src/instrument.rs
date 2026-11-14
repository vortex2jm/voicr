use serde::{Deserialize, Serialize};

use crate::pitch::{spell_chromatic, MidiNote};

/// Instrument family category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstrumentKind {
    Guitar,
    Cavaquinho,
    ViolaCaipira,
    Custom,
}

/// Configuration of a fretted string instrument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentConfig {
    pub id: String,
    pub name: String,
    pub tuning_name: String,
    pub kind: InstrumentKind,
    /// Open MIDI notes from lowest (bass) string/course to highest (treble) string/course.
    pub tuning: Vec<MidiNote>,
    /// Number of frets on the fingerboard (default: 15).
    pub num_frets: u8,
    /// Optional paired-string offset in semitones per course (e.g. `Some(12)` for octave bass pairs
    /// and `Some(0)` for unison pairs on Viola Caipira; empty or `None` for single-string courses).
    #[serde(default)]
    pub course_octave_offsets: Vec<Option<i8>>,
}

impl InstrumentConfig {
    /// Default number of frets.
    pub const DEFAULT_FRETS: u8 = 15;

    /// Returns the number of logical strings/courses.
    pub fn num_strings(&self) -> usize {
        self.tuning.len()
    }

    /// Validates that the instrument has 4..=7 strings and at least 1 fret.
    pub fn validate(&self) -> Result<(), String> {
        let n = self.tuning.len();
        if !(4..=7).contains(&n) {
            return Err(format!(
                "O instrumento deve ter entre 4 e 7 cordas (recebido: {})",
                n
            ));
        }
        if self.num_frets == 0 || self.num_frets > 24 {
            return Err(format!(
                "O número de casas deve estar entre 1 e 24 (recebido: {})",
                self.num_frets
            ));
        }
        Ok(())
    }

    /// Creates a custom instrument configuration with 4..=7 strings.
    pub fn custom(tuning: Vec<u8>, num_frets: u8) -> Result<Self, String> {
        let cfg = Self {
            id: "custom".to_string(),
            name: "Customizado".to_string(),
            tuning_name: "Afinação Customizada".to_string(),
            kind: InstrumentKind::Custom,
            tuning: tuning.into_iter().map(MidiNote).collect(),
            num_frets,
            course_octave_offsets: Vec::new(),
        };
        cfg.validate()?;
        Ok(cfg)
    }

    /// Returns human-readable open string names (e.g., `["E2", "A2", "D3", "G3", "B3", "E4"]`).
    pub fn open_string_labels(&self, prefer_flats: bool) -> Vec<String> {
        self.tuning
            .iter()
            .map(|m| m.format_with_octave(prefer_flats))
            .collect()
    }

    /// Returns short pitch class names for open strings (e.g., `["E", "A", "D", "G", "B", "E"]`).
    pub fn open_pitch_names(&self, prefer_flats: bool) -> Vec<String> {
        self.tuning
            .iter()
            .map(|m| spell_chromatic(m.pitch_class(), prefer_flats).to_string())
            .collect()
    }
}

/// Returns all built-in instrument and tuning presets.
pub fn builtin_presets() -> Vec<InstrumentConfig> {
    let viola_octaves = vec![Some(12), Some(12), Some(0), Some(0), Some(0)];

    vec![
        // Violão (6 cordas)
        InstrumentConfig {
            id: "guitar-standard".to_string(),
            name: "Violão (6 cordas)".to_string(),
            tuning_name: "Padrão (E-A-D-G-B-E)".to_string(),
            kind: InstrumentKind::Guitar,
            tuning: [40, 45, 50, 55, 59, 64].into_iter().map(MidiNote).collect(),
            num_frets: InstrumentConfig::DEFAULT_FRETS,
            course_octave_offsets: vec![None; 6],
        },
        InstrumentConfig {
            id: "guitar-drop-d".to_string(),
            name: "Violão (6 cordas)".to_string(),
            tuning_name: "Drop D (D-A-D-G-B-E)".to_string(),
            kind: InstrumentKind::Guitar,
            tuning: [38, 45, 50, 55, 59, 64].into_iter().map(MidiNote).collect(),
            num_frets: InstrumentConfig::DEFAULT_FRETS,
            course_octave_offsets: vec![None; 6],
        },
        InstrumentConfig {
            id: "guitar-open-g".to_string(),
            name: "Violão (6 cordas)".to_string(),
            tuning_name: "Open G (D-G-D-G-B-D)".to_string(),
            kind: InstrumentKind::Guitar,
            tuning: [38, 43, 50, 55, 59, 62].into_iter().map(MidiNote).collect(),
            num_frets: InstrumentConfig::DEFAULT_FRETS,
            course_octave_offsets: vec![None; 6],
        },
        InstrumentConfig {
            id: "guitar-open-d".to_string(),
            name: "Violão (6 cordas)".to_string(),
            tuning_name: "Open D (D-A-D-F#-A-D)".to_string(),
            kind: InstrumentKind::Guitar,
            tuning: [38, 45, 50, 54, 57, 62].into_iter().map(MidiNote).collect(),
            num_frets: InstrumentConfig::DEFAULT_FRETS,
            course_octave_offsets: vec![None; 6],
        },
        // Cavaquinho (4 cordas)
        InstrumentConfig {
            id: "cavaquinho-standard".to_string(),
            name: "Cavaquinho (4 cordas)".to_string(),
            tuning_name: "Padrão (D-G-B-D)".to_string(),
            kind: InstrumentKind::Cavaquinho,
            tuning: [62, 67, 71, 74].into_iter().map(MidiNote).collect(),
            num_frets: InstrumentConfig::DEFAULT_FRETS,
            course_octave_offsets: vec![None; 4],
        },
        InstrumentConfig {
            id: "cavaquinho-guitar".to_string(),
            name: "Cavaquinho (4 cordas)".to_string(),
            tuning_name: "Afinação de Violão (D-G-B-E)".to_string(),
            kind: InstrumentKind::Cavaquinho,
            tuning: [62, 67, 71, 76].into_iter().map(MidiNote).collect(),
            num_frets: InstrumentConfig::DEFAULT_FRETS,
            course_octave_offsets: vec![None; 4],
        },
        // Viola Caipira (5 ordens)
        InstrumentConfig {
            id: "viola-cebolao-e".to_string(),
            name: "Viola Caipira (5 ordens)".to_string(),
            tuning_name: "Cebolão em Mi (E-B-E-G#-B)".to_string(),
            kind: InstrumentKind::ViolaCaipira,
            tuning: [40, 47, 52, 56, 59].into_iter().map(MidiNote).collect(),
            num_frets: InstrumentConfig::DEFAULT_FRETS,
            course_octave_offsets: viola_octaves.clone(),
        },
        InstrumentConfig {
            id: "viola-cebolao-d".to_string(),
            name: "Viola Caipira (5 ordens)".to_string(),
            tuning_name: "Cebolão em Ré (D-A-D-F#-A)".to_string(),
            kind: InstrumentKind::ViolaCaipira,
            tuning: [38, 45, 50, 54, 57].into_iter().map(MidiNote).collect(),
            num_frets: InstrumentConfig::DEFAULT_FRETS,
            course_octave_offsets: viola_octaves.clone(),
        },
        InstrumentConfig {
            id: "viola-rio-abaixo".to_string(),
            name: "Viola Caipira (5 ordens)".to_string(),
            tuning_name: "Rio Abaixo (D-G-D-G-B)".to_string(),
            kind: InstrumentKind::ViolaCaipira,
            tuning: [38, 43, 50, 55, 59].into_iter().map(MidiNote).collect(),
            num_frets: InstrumentConfig::DEFAULT_FRETS,
            course_octave_offsets: viola_octaves,
        },
    ]
}
