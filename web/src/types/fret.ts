export type ToneRole = 'Root' | 'Required' | 'Optional' | 'AlteredPool';

export interface ChordTone {
  pitch_class: number;
  semitones: number;
  interval_label: string;
  note_name: string;
  role: ToneRole;
}

export interface ChordSpec {
  symbol: string;
  root: number;
  root_name: string;
  slash_bass: number | null;
  slash_bass_name: string | null;
  tones: ChordTone[];
  prefer_flats: boolean;
  is_altered: boolean;
}

export type InstrumentKind = 'Guitar' | 'Cavaquinho' | 'ViolaCaipira' | 'Custom';

export interface InstrumentConfig {
  id: string;
  name: string;
  tuning_name: string;
  kind: InstrumentKind;
  tuning: number[];
  num_frets: number;
  course_octave_offsets: (number | null)[];
}

export interface SearchOptions {
  min_fret: number;
  max_fret: number;
  max_span: number;
  allow_open_strings: boolean;
  omit_fifth: boolean;
  allow_rootless: boolean;
  pinned_frets: (number | null)[];
  limit: number;
}

export interface Barre {
  fret: number;
  from_string: number;
  to_string: number;
}

export interface VoicingNote {
  string_index: number;
  fret: number;
  midi: number;
  note_name: string;
  interval: string;
  is_root: boolean;
  is_bass: boolean;
  finger: number | null;
  octave_pair_offset: number | null;
}

export interface VoicingResult {
  frets: (number | null)[];
  fingers: (number | null)[];
  barre: Barre | null;
  notes: VoicingNote[];
  min_fret: number;
  max_fret: number;
  score: number;
}

export type FretOrientation = 'vertical' | 'horizontal';
export type LabelMode = 'fingers' | 'notes' | 'intervals';
export type FretRegion = 'all' | '0-4' | '5-9' | '9-14';
export type PlayMode = 'strum' | 'arpeggio';
