pub mod chord;
pub mod instrument;
pub mod pitch;
pub mod scoring;
pub mod solver;

pub use chord::{parse_chord, ChordParseError, ChordSpec, ChordTone, ToneRole};
pub use instrument::{builtin_presets, InstrumentConfig, InstrumentKind};
pub use pitch::{MidiNote, PitchClass};
pub use solver::{assign_fingers_and_barre, solve_chord, Barre, SearchOptions, VoicingNote, VoicingResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_complex_chords_and_enharmonics() {
        let cmaj9_s11 = parse_chord("Cmaj9(#11)").expect("should parse Cmaj9(#11)");
        assert_eq!(cmaj9_s11.root, PitchClass(0));
        let labels: Vec<(&str, &str)> = cmaj9_s11
            .tones
            .iter()
            .map(|t| (t.note_name.as_str(), t.interval_label.as_str()))
            .collect();
        assert!(labels.contains(&("C", "R")));
        assert!(labels.contains(&("E", "3")));
        assert!(labels.contains(&("G", "5")));
        assert!(labels.contains(&("B", "7")));
        assert!(labels.contains(&("D", "9")));
        assert!(labels.contains(&("F#", "#11")));

        let half_dim = parse_chord("F#m7(b5)/C").expect("should parse F#m7(b5)/C");
        assert_eq!(half_dim.root, PitchClass(6));
        assert_eq!(half_dim.slash_bass, Some(PitchClass(0)));
        assert_eq!(half_dim.slash_bass_name.as_deref(), Some("C"));
        let half_dim_notes: Vec<&str> = half_dim.tones.iter().map(|t| t.note_name.as_str()).collect();
        assert_eq!(half_dim_notes, vec!["F#", "A", "C", "E"]);

        let six_nine = parse_chord("C6/9").expect("should parse C6/9");
        assert!(six_nine.slash_bass.is_none());
        assert!(six_nine.tones.iter().any(|t| t.interval_label == "6"));
        assert!(six_nine.tones.iter().any(|t| t.interval_label == "9"));

        let min_maj7 = parse_chord("Am(maj7)").expect("should parse Am(maj7)");
        assert!(min_maj7.tones.iter().any(|t| t.interval_label == "b3"));
        assert!(min_maj7.tones.iter().any(|t| t.interval_label == "7"));

        let alt = parse_chord("G7alt").expect("should parse G7alt");
        assert!(alt.is_altered);

        assert!(parse_chord("").is_err());
        assert!(parse_chord("H7").is_err());
        assert!(parse_chord("Cmaj7/").is_err());
        assert!(parse_chord("Cxyz").is_err());
    }

    #[test]
    fn rejects_positions_needing_more_than_four_fingers() {
        // 5 fretted notes on distinct frets (no barre possible) -> rejected
        let too_many_frets = [Some(1), Some(2), Some(3), Some(4), Some(5), None];
        assert!(assign_fingers_and_barre(&too_many_frets).is_none());

        // Barre at fret 1 on strings 0 and 5, PLUS 4 other fretted notes -> 5 fingers -> rejected
        let barre_plus_four = [Some(1), Some(3), Some(3), Some(2), Some(2), Some(1)];
        assert!(assign_fingers_and_barre(&barre_plus_four).is_none());

        // Open string inside potential barre range blocks barre -> 5 separate fretted notes -> rejected
        let open_blocks_barre = [Some(1), Some(0), Some(2), Some(3), Some(2), Some(1)];
        assert!(assign_fingers_and_barre(&open_blocks_barre).is_none());

        // Valid E-shape barre chord -> finger 1 barre + fingers 2, 3, 4
        let e_shape = [Some(1), Some(3), Some(3), Some(2), Some(1), Some(1)];
        let (fingers, barre) = assign_fingers_and_barre(&e_shape).expect("E-shape must be valid");
        assert_eq!(
            barre,
            Some(Barre {
                fret: 1,
                from_string: 0,
                to_string: 5
            })
        );
        assert_eq!(
            fingers,
            vec![Some(1), Some(3), Some(4), Some(2), Some(1), Some(1)]
        );
    }

    #[test]
    fn solves_known_chords_across_instruments() {
        let presets = builtin_presets();
        let guitar = presets
            .iter()
            .find(|p| p.id == "guitar-standard")
            .expect("guitar-standard exists");
        let cavaquinho = presets
            .iter()
            .find(|p| p.id == "cavaquinho-standard")
            .expect("cavaquinho-standard exists");
        let viola = presets
            .iter()
            .find(|p| p.id == "viola-cebolao-e")
            .expect("viola-cebolao-e exists");

        // Guitar: C major top voicing should include classic x32010
        let c_chord = parse_chord("C").unwrap();
        let opts = SearchOptions::default();
        let guitar_c = solve_chord(&c_chord, guitar, &opts);
        assert!(!guitar_c.is_empty());
        assert_eq!(
            guitar_c[0].frets,
            vec![None, Some(3), Some(2), Some(0), Some(1), Some(0)]
        );

        // Guitar: F#m7(b5)/C must have C as lowest sounded note in every voicing
        let slash_chord = parse_chord("F#m7(b5)/C").unwrap();
        let guitar_slash = solve_chord(&slash_chord, guitar, &opts);
        assert!(!guitar_slash.is_empty());
        for v in &guitar_slash {
            assert_eq!(v.notes[0].midi.pitch_class(), PitchClass(0));
            assert!(v.notes[0].is_bass);
        }

        // Cavaquinho: G7 and Cmaj7 voicings
        let g7 = parse_chord("G7").unwrap();
        let cav_g7 = solve_chord(&g7, cavaquinho, &opts);
        assert!(!cav_g7.is_empty());
        assert!(cav_g7.iter().all(|v| v.frets.len() == 4));

        // Cavaquinho rootless Cmaj9(#11) or C9 on 4 strings
        let c9 = parse_chord("C9").unwrap();
        let cav_c9_rooted = solve_chord(&c9, cavaquinho, &opts);
        // With 4 required tones (R, 3, b7, 9) and 4 strings, rooted C9 is possible;
        // Cmaj9(#11) has 5 required tones so needs allow_rootless on 4 strings to drop root,
        // wait: still 4 remaining required tones (3, 7, 9, #11) fits on 4 strings!
        assert!(!cav_c9_rooted.is_empty());
        let cmaj9_s11 = parse_chord("Cmaj9(#11)").unwrap();
        let cav_without_rootless = solve_chord(&cmaj9_s11, cavaquinho, &opts);
        assert!(cav_without_rootless.is_empty());
        let rootless_opts = SearchOptions {
            allow_rootless: true,
            ..Default::default()
        };
        let cav_with_rootless = solve_chord(&cmaj9_s11, cavaquinho, &rootless_opts);
        assert!(!cav_with_rootless.is_empty());

        // Viola Caipira Cebolão em Mi: E major open [0, 0, 0, 0, 0] should rank #1!
        let e_major = parse_chord("E").unwrap();
        let viola_e = solve_chord(&e_major, viola, &opts);
        assert!(!viola_e.is_empty());
        assert_eq!(
            viola_e[0].frets,
            vec![Some(0), Some(0), Some(0), Some(0), Some(0)]
        );
        assert_eq!(viola_e[0].notes[0].octave_pair_offset, Some(12));
    }

    #[test]
    fn pinned_frets_constrains_results() {
        let guitar = builtin_presets()
            .into_iter()
            .find(|p| p.id == "guitar-standard")
            .unwrap();
        let cmaj7 = parse_chord("Cmaj7").unwrap();

        // Pin string 0 (low E) to fret 8 (C3)
        let opts = SearchOptions {
            pinned_frets: vec![Some(8), None, None, None, None, None],
            max_fret: 12,
            ..Default::default()
        };
        let results = solve_chord(&cmaj7, &guitar, &opts);
        assert!(!results.is_empty());
        for v in &results {
            assert_eq!(v.frets[0], Some(8));
        }
    }
}
