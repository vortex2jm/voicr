import type { FretRegion, InstrumentConfig, SearchOptions } from '../types/fret';

interface InstrumentControlsProps {
  presets: InstrumentConfig[];
  selectedPresetId: string;
  instrument: InstrumentConfig;
  options: SearchOptions;
  region: FretRegion;
  onSelectPreset: (presetId: string) => void;
  onUpdateCustomTuning: (nextTuning: number[], nextNumFrets: number) => void;
  onRegionChange: (region: FretRegion) => void;
  onOptionsChange: (patch: Partial<SearchOptions>) => void;
}

const NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];

function formatMidiNote(midi: number): string {
  const note = NOTE_NAMES[((midi % 12) + 12) % 12];
  const octave = Math.floor(midi / 12) - 1;
  return `${note}${octave}`;
}

const REGIONS: { id: FretRegion; label: string }[] = [
  { id: 'all', label: 'Braço Inteiro' },
  { id: '0-4', label: 'Casas 0–4' },
  { id: '5-9', label: 'Casas 5–9' },
  { id: '9-14', label: 'Casas 9–14' },
];

export function InstrumentControls({
  presets,
  selectedPresetId,
  instrument,
  options,
  region,
  onSelectPreset,
  onUpdateCustomTuning,
  onRegionChange,
  onOptionsChange,
}: InstrumentControlsProps) {
  const isCustom = selectedPresetId === 'custom' || instrument.kind === 'Custom';
  const isFourString = instrument.tuning.length === 4;

  const groupedPresets = [
    {
      label: 'Violão (6 cordas)',
      items: presets.filter((p) => p.kind === 'Guitar'),
    },
    {
      label: 'Cavaquinho (4 cordas)',
      items: presets.filter((p) => p.kind === 'Cavaquinho'),
    },
    {
      label: 'Viola Caipira (5 ordens)',
      items: presets.filter((p) => p.kind === 'ViolaCaipira'),
    },
  ];

  const handleStringMidiStep = (stringIdx: number, delta: number) => {
    const next = [...instrument.tuning];
    const updated = Math.min(84, Math.max(24, next[stringIdx] + delta));
    next[stringIdx] = updated;
    onUpdateCustomTuning(next, instrument.num_frets);
  };

  const handleAddString = () => {
    if (instrument.tuning.length >= 7) return;
    const last = instrument.tuning[instrument.tuning.length - 1] ?? 64;
    onUpdateCustomTuning([...instrument.tuning, Math.min(84, last + 5)], instrument.num_frets);
  };

  const handleRemoveString = () => {
    if (instrument.tuning.length <= 4) return;
    onUpdateCustomTuning(instrument.tuning.slice(0, -1), instrument.num_frets);
  };

  return (
    <section className="rounded-2xl border border-stone-800/90 bg-stone-900/75 p-4 shadow-xl shadow-black/25 backdrop-blur-sm sm:p-5">
      <div className="grid grid-cols-1 gap-5 lg:grid-cols-12">
        {/* Instrument & Tuning Selector */}
        <div className="flex flex-col gap-3 lg:col-span-5">
          <div className="flex items-center justify-between">
            <label
              htmlFor="instrument-select"
              className="text-xs font-semibold uppercase tracking-wider text-stone-400"
            >
              Instrumento & Afinação
            </label>
            <span className="rounded-full bg-stone-800 px-2.5 py-0.5 font-mono text-[11px] text-stone-300">
              {instrument.tuning.length} {instrument.kind === 'ViolaCaipira' ? 'ordens' : 'cordas'}
            </span>
          </div>

          <select
            id="instrument-select"
            value={isCustom ? 'custom' : selectedPresetId}
            onChange={(e) => onSelectPreset(e.target.value)}
            className="w-full rounded-xl border border-stone-700/80 bg-stone-950/90 px-3 py-2.5 text-sm font-medium text-stone-100 outline-none transition focus:border-amber-400 focus:ring-2 focus:ring-amber-500/20"
          >
            {groupedPresets.map((group) => (
              <optgroup key={group.label} label={group.label}>
                {group.items.map((preset) => (
                  <option key={preset.id} value={preset.id}>
                    {preset.name} — {preset.tuning_name}
                  </option>
                ))}
              </optgroup>
            ))}
            <optgroup label="Personalizado">
              <option value="custom">Afinação Customizada (4–7 cordas)</option>
            </optgroup>
          </select>

          {/* Open strings summary or inline custom tuning editor */}
          {!isCustom ? (
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="text-[11px] text-stone-500">Cordas soltas:</span>
              {instrument.tuning.map((midi, idx) => (
                <span
                  key={idx}
                  className="rounded-md border border-stone-800 bg-stone-950/70 px-2 py-0.5 font-mono text-xs text-stone-300"
                  title={`Corda ${instrument.tuning.length - idx}: MIDI ${midi}`}
                >
                  {formatMidiNote(midi)}
                </span>
              ))}
              {instrument.kind === 'ViolaCaipira' && (
                <span className="ml-1 rounded-md bg-amber-500/10 px-2 py-0.5 text-[11px] text-amber-300">
                  Oitava nos pares graves
                </span>
              )}
            </div>
          ) : (
            <div className="flex flex-col gap-2.5 rounded-xl border border-stone-800 bg-stone-950/70 p-3">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className="text-xs font-medium text-stone-300">
                  Editor de Cordas Soltas (grave → aguda)
                </span>
                <div className="flex items-center gap-1.5">
                  <button
                    type="button"
                    onClick={handleRemoveString}
                    disabled={instrument.tuning.length <= 4}
                    className="rounded-md border border-stone-700 bg-stone-800 px-2 py-0.5 text-xs text-stone-200 transition hover:bg-stone-700 disabled:opacity-40"
                  >
                    - Corda
                  </button>
                  <button
                    type="button"
                    onClick={handleAddString}
                    disabled={instrument.tuning.length >= 7}
                    className="rounded-md border border-stone-700 bg-stone-800 px-2 py-0.5 text-xs text-stone-200 transition hover:bg-stone-700 disabled:opacity-40"
                  >
                    + Corda
                  </button>
                </div>
              </div>

              <div className="flex flex-wrap gap-2">
                {instrument.tuning.map((midi, idx) => (
                  <div
                    key={idx}
                    className="flex items-center gap-1 rounded-lg border border-stone-800 bg-stone-900 px-1.5 py-1"
                  >
                    <button
                      type="button"
                      onClick={() => handleStringMidiStep(idx, -1)}
                      className="rounded px-1.5 text-xs font-bold text-stone-400 hover:bg-stone-800 hover:text-white"
                      title="Diminuir 1 semitom"
                    >
                      −
                    </button>
                    <div className="min-w-[3.1rem] text-center font-mono text-xs">
                      <div className="font-semibold text-amber-300">{formatMidiNote(midi)}</div>
                      <div className="text-[10px] text-stone-500">MIDI {midi}</div>
                    </div>
                    <button
                      type="button"
                      onClick={() => handleStringMidiStep(idx, 1)}
                      className="rounded px-1.5 text-xs font-bold text-stone-400 hover:bg-stone-800 hover:text-white"
                      title="Aumentar 1 semitom"
                    >
                      +
                    </button>
                  </div>
                ))}
              </div>

              <div className="flex items-center justify-between gap-2 border-t border-stone-800/80 pt-2">
                <label htmlFor="custom-frets" className="text-xs text-stone-400">
                  Número de casas: <span className="font-mono text-stone-200">{instrument.num_frets}</span>
                </label>
                <input
                  id="custom-frets"
                  type="range"
                  min={12}
                  max={22}
                  value={instrument.num_frets}
                  onChange={(e) => onUpdateCustomTuning(instrument.tuning, Number(e.target.value))}
                  className="h-1.5 w-36 accent-amber-400"
                />
              </div>
            </div>
          )}
        </div>

        {/* Region, Span & Harmonic Toggles */}
        <div className="flex flex-col justify-between gap-4 lg:col-span-7">
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            {/* Neck Region */}
            <div className="flex flex-col gap-2">
              <span className="text-xs font-semibold uppercase tracking-wider text-stone-400">
                Região do Braço
              </span>
              <div className="grid grid-cols-2 gap-1.5">
                {REGIONS.map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => onRegionChange(item.id)}
                    className={`rounded-lg px-2.5 py-1.5 text-xs font-medium transition ${
                      region === item.id
                        ? 'bg-amber-500/20 text-amber-300 ring-1 ring-amber-500/50'
                        : 'bg-stone-800/75 text-stone-300 hover:bg-stone-800 hover:text-white'
                    }`}
                  >
                    {item.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Max Span */}
            <div className="flex flex-col gap-2">
              <span className="text-xs font-semibold uppercase tracking-wider text-stone-400">
                Abertura Máxima
              </span>
              <div className="grid grid-cols-3 gap-1.5">
                {[3, 4, 5].map((span) => (
                  <button
                    key={span}
                    type="button"
                    onClick={() => onOptionsChange({ max_span: span })}
                    className={`rounded-lg px-2.5 py-2 font-mono text-xs font-semibold transition ${
                      options.max_span === span
                        ? 'bg-amber-500/20 text-amber-300 ring-1 ring-amber-500/50'
                        : 'bg-stone-800/75 text-stone-300 hover:bg-stone-800 hover:text-white'
                    }`}
                  >
                    {span} casas
                  </button>
                ))}
              </div>
            </div>
          </div>

          {/* Toggles */}
          <div className="flex flex-wrap items-center gap-2.5 border-t border-stone-800/80 pt-3">
            <TogglePill
              label="Omitir 5ª"
              checked={options.omit_fifth}
              onChange={(checked) => onOptionsChange({ omit_fifth: checked })}
            />
            <TogglePill
              label="Cordas Soltas"
              checked={options.allow_open_strings}
              onChange={(checked) => onOptionsChange({ allow_open_strings: checked })}
            />
            <TogglePill
              label="Rootless (4 cordas)"
              checked={options.allow_rootless}
              disabled={!isFourString}
              title={
                isFourString
                  ? 'Permite omitir a tônica em acordes de 4+ notas no cavaquinho/4 cordas'
                  : 'Disponível para instrumentos de 4 cordas'
              }
              onChange={(checked) => onOptionsChange({ allow_rootless: checked })}
            />
          </div>
        </div>
      </div>
    </section>
  );
}

interface TogglePillProps {
  label: string;
  checked: boolean;
  disabled?: boolean;
  title?: string;
  onChange: (next: boolean) => void;
}

function TogglePill({ label, checked, disabled, title, onChange }: TogglePillProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      title={title}
      onClick={() => onChange(!checked)}
      className={`inline-flex items-center gap-2 rounded-full border px-3 py-1.5 text-xs font-medium transition ${
        disabled
          ? 'cursor-not-allowed border-stone-800 bg-stone-900/50 text-stone-600'
          : checked
            ? 'border-amber-500/50 bg-amber-500/15 text-amber-200'
            : 'border-stone-700/80 bg-stone-800/70 text-stone-400 hover:border-stone-600 hover:text-stone-200'
      }`}
    >
      <span
        className={`h-2 w-2 rounded-full transition ${
          disabled ? 'bg-stone-700' : checked ? 'bg-amber-400' : 'bg-stone-500'
        }`}
      />
      {label}
    </button>
  );
}
