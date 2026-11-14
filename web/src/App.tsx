import { useCallback, useEffect, useMemo, useState } from 'react';
import { get_presets, parse_chord, solve_chord } from './wasm/fret_wasm';
import { playVoicing } from './audio/karplusStrong';
import { ChordInputBar } from './components/ChordInputBar';
import { InstrumentControls } from './components/InstrumentControls';
import { VoicingGrid } from './components/VoicingGrid';
import type {
  ChordSpec,
  FretOrientation,
  FretRegion,
  InstrumentConfig,
  LabelMode,
  PlayMode,
  SearchOptions,
  VoicingResult,
} from './types/fret';

const STORAGE_KEYS = {
  presetId: 'voicr.presetId',
  customTuning: 'voicr.customTuning',
  customFrets: 'voicr.customFrets',
  orientation: 'voicr.orientation',
  labelMode: 'voicr.labelMode',
} as const;

function readStorage<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

function writeStorage<T>(key: string, value: T): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Ignore storage quota or private mode failures
  }
}

function regionBounds(region: FretRegion, numFrets: number): { min_fret: number; max_fret: number } {
  switch (region) {
    case '0-4':
      return { min_fret: 0, max_fret: Math.min(4, numFrets) };
    case '5-9':
      return { min_fret: 5, max_fret: Math.min(9, numFrets) };
    case '9-14':
      return { min_fret: 9, max_fret: Math.min(14, numFrets) };
    case 'all':
    default:
      return { min_fret: 0, max_fret: numFrets };
  }
}

export function App() {
  const presets = useMemo<InstrumentConfig[]>(() => get_presets() as InstrumentConfig[], []);

  const [selectedPresetId, setSelectedPresetId] = useState<string>(() =>
    readStorage<string>(STORAGE_KEYS.presetId, presets[0]?.id ?? 'guitar-standard'),
  );
  const [customTuning, setCustomTuning] = useState<number[]>(() =>
    readStorage<number[]>(STORAGE_KEYS.customTuning, [40, 45, 50, 55, 59, 64]),
  );
  const [customNumFrets, setCustomNumFrets] = useState<number>(() =>
    readStorage<number>(STORAGE_KEYS.customFrets, 15),
  );

  const [orientation, setOrientation] = useState<FretOrientation>(() =>
    readStorage<FretOrientation>(STORAGE_KEYS.orientation, 'vertical'),
  );
  const [labelMode, setLabelMode] = useState<LabelMode>(() =>
    readStorage<LabelMode>(STORAGE_KEYS.labelMode, 'fingers'),
  );

  const instrument = useMemo<InstrumentConfig>(() => {
    if (selectedPresetId === 'custom') {
      return {
        id: 'custom',
        name: 'Customizado',
        tuning_name: 'Afinação Customizada',
        kind: 'Custom',
        tuning: customTuning,
        num_frets: customNumFrets,
        course_octave_offsets: [],
      };
    }
    return presets.find((p) => p.id === selectedPresetId) ?? presets[0];
  }, [presets, selectedPresetId, customTuning, customNumFrets]);

  const [query, setQuery] = useState<string>('Cmaj9(#11)');
  const [debouncedQuery, setDebouncedQuery] = useState<string>('Cmaj9(#11)');
  const [region, setRegion] = useState<FretRegion>('all');

  const [options, setOptions] = useState<SearchOptions>(() => ({
    min_fret: 0,
    max_fret: 15,
    max_span: 4,
    allow_open_strings: true,
    omit_fifth: true,
    allow_rootless: false,
    pinned_frets: Array.from({ length: instrument.tuning.length }, () => null),
    limit: 25,
  }));

  const [chordSpec, setChordSpec] = useState<ChordSpec | null>(null);
  const [parseError, setParseError] = useState<string | null>(null);
  const [voicings, setVoicings] = useState<VoicingResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState<number>(0);

  // Persist user preferences
  useEffect(() => writeStorage(STORAGE_KEYS.presetId, selectedPresetId), [selectedPresetId]);
  useEffect(() => writeStorage(STORAGE_KEYS.customTuning, customTuning), [customTuning]);
  useEffect(() => writeStorage(STORAGE_KEYS.customFrets, customNumFrets), [customNumFrets]);
  useEffect(() => writeStorage(STORAGE_KEYS.orientation, orientation), [orientation]);
  useEffect(() => writeStorage(STORAGE_KEYS.labelMode, labelMode), [labelMode]);

  // 40ms debounce on chord text input
  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedQuery(query);
    }, 40);
    return () => window.clearTimeout(timer);
  }, [query]);

  // Execute Rust/WASM parser + solver whenever debounced query, instrument, or options change
  useEffect(() => {
    const trimmed = debouncedQuery.trim();
    if (!trimmed) {
      setChordSpec(null);
      setParseError('Digite uma cifra para calcular os voicings (ex: Cmaj7, F#m7(b5)/C)');
      setVoicings([]);
      setSelectedIndex(0);
      return;
    }

    try {
      const parsed = parse_chord(trimmed) as ChordSpec;
      setChordSpec(parsed);
      setParseError(null);

      const solved = solve_chord(trimmed, instrument, options) as VoicingResult[];
      setVoicings(solved);
      setSelectedIndex(0);
    } catch (err) {
      const msg = typeof err === 'string' ? err : err instanceof Error ? err.message : String(err);
      setParseError(msg);
      setChordSpec(null);
      setVoicings([]);
      setSelectedIndex(0);
    }
  }, [debouncedQuery, instrument, options]);

  const handleSelectPreset = useCallback(
    (presetId: string) => {
      setSelectedPresetId(presetId);
      const nextInst =
        presetId === 'custom'
          ? { tuning: customTuning, num_frets: customNumFrets }
          : presets.find((p) => p.id === presetId) ?? presets[0];
      const bounds = regionBounds(region, nextInst.num_frets);
      setOptions((prev) => ({
        ...prev,
        ...bounds,
        pinned_frets: Array.from({ length: nextInst.tuning.length }, () => null),
      }));
    },
    [customTuning, customNumFrets, presets, region],
  );

  const handleUpdateCustomTuning = useCallback(
    (nextTuning: number[], nextNumFrets: number) => {
      setSelectedPresetId('custom');
      setCustomTuning(nextTuning);
      setCustomNumFrets(nextNumFrets);
      const bounds = regionBounds(region, nextNumFrets);
      setOptions((prev) => ({
        ...prev,
        ...bounds,
        pinned_frets: Array.from({ length: nextTuning.length }, (_, idx) =>
          idx < prev.pinned_frets.length ? prev.pinned_frets[idx] : null,
        ),
      }));
    },
    [region],
  );

  const handleRegionChange = useCallback(
    (nextRegion: FretRegion) => {
      setRegion(nextRegion);
      const bounds = regionBounds(nextRegion, instrument.num_frets);
      setOptions((prev) => ({ ...prev, ...bounds }));
    },
    [instrument.num_frets],
  );

  const handleOptionsChange = useCallback((patch: Partial<SearchOptions>) => {
    setOptions((prev) => ({ ...prev, ...patch }));
  }, []);

  const handleTogglePin = useCallback(
    (stringIndex: number, fret: number) => {
      setOptions((prev) => {
        const nextPins = Array.from(
          { length: instrument.tuning.length },
          (_, i) => prev.pinned_frets[i] ?? null,
        );
        nextPins[stringIndex] = nextPins[stringIndex] === fret ? null : fret;
        return { ...prev, pinned_frets: nextPins };
      });
    },
    [instrument.tuning.length],
  );

  const handleClearPins = useCallback(() => {
    setOptions((prev) => ({
      ...prev,
      pinned_frets: Array.from({ length: instrument.tuning.length }, () => null),
    }));
  }, [instrument.tuning.length]);

  const handlePlay = useCallback(
    (voicing: VoicingResult, mode: PlayMode) => {
      playVoicing(voicing, instrument, mode);
    },
    [instrument],
  );

  return (
    <div className="min-h-screen bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-stone-900 via-stone-950 to-black text-stone-100">
      <div className="mx-auto flex max-w-7xl flex-col gap-5 px-4 py-6 sm:px-6 lg:px-8">
        {/* Header */}
        <header className="flex flex-wrap items-center justify-between gap-4 border-b border-stone-800/80 pb-4">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br from-amber-400 to-amber-600 font-mono text-lg font-black text-stone-950 shadow-lg shadow-amber-500/20">
              vr
            </div>
            <div>
              <h1 className="text-xl font-bold tracking-tight text-stone-100 sm:text-2xl">
                voicr
              </h1>
              <p className="text-xs text-stone-400">
                Motor algorítmico de voicings em Rust + WebAssembly · 100% client-side
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2 text-xs text-stone-400">
            <span className="inline-flex items-center gap-1.5 rounded-full border border-stone-800 bg-stone-900/90 px-3 py-1">
              <span className="h-2 w-2 rounded-full bg-emerald-400" />
              WASM Solver + Karplus-Strong Audio
            </span>
          </div>
        </header>

        {/* Chord Input */}
        <ChordInputBar
          query={query}
          onQueryChange={setQuery}
          chordSpec={chordSpec}
          parseError={parseError}
          omitFifth={options.omit_fifth}
        />

        {/* Instrument & Filter Controls */}
        <InstrumentControls
          presets={presets}
          selectedPresetId={selectedPresetId}
          instrument={instrument}
          options={options}
          region={region}
          onSelectPreset={handleSelectPreset}
          onUpdateCustomTuning={handleUpdateCustomTuning}
          onRegionChange={handleRegionChange}
          onOptionsChange={handleOptionsChange}
        />

        {/* Main Diagram & Voicings Grid */}
        <VoicingGrid
          chordSpec={chordSpec}
          instrument={instrument}
          voicings={voicings}
          selectedIndex={selectedIndex}
          orientation={orientation}
          labelMode={labelMode}
          pinnedFrets={options.pinned_frets}
          onSelectIndex={setSelectedIndex}
          onOrientationChange={setOrientation}
          onLabelModeChange={setLabelMode}
          onTogglePin={handleTogglePin}
          onClearPins={handleClearPins}
          onPlay={handlePlay}
        />
      </div>
    </div>
  );
}

export default App;
