import { useEffect } from 'react';
import type {
  ChordSpec,
  FretOrientation,
  InstrumentConfig,
  LabelMode,
  PlayMode,
  VoicingResult,
} from '../types/fret';
import { FretboardSvg } from './FretboardSvg';

interface VoicingGridProps {
  chordSpec: ChordSpec | null;
  instrument: InstrumentConfig;
  voicings: VoicingResult[];
  selectedIndex: number;
  orientation: FretOrientation;
  labelMode: LabelMode;
  pinnedFrets: (number | null)[];
  onSelectIndex: (idx: number) => void;
  onOrientationChange: (next: FretOrientation) => void;
  onLabelModeChange: (next: LabelMode) => void;
  onTogglePin: (stringIndex: number, fret: number) => void;
  onClearPins: () => void;
  onPlay: (voicing: VoicingResult, mode: PlayMode) => void;
}

function formatFretsBadge(frets: (number | null)[]): string {
  return frets.map((f) => (f === null ? 'X' : String(f))).join(' ');
}

export function VoicingGrid({
  chordSpec,
  instrument,
  voicings,
  selectedIndex,
  orientation,
  labelMode,
  pinnedFrets,
  onSelectIndex,
  onOrientationChange,
  onLabelModeChange,
  onTogglePin,
  onClearPins,
  onPlay,
}: VoicingGridProps) {
  const activeVoicing = voicings[selectedIndex] ?? null;
  const activePinCount = pinnedFrets.filter((f) => f !== null).length;

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const tag = target?.tagName?.toLowerCase();
      if (tag === 'input' || tag === 'textarea' || tag === 'select' || target?.isContentEditable) {
        return;
      }

      if (e.key === 'ArrowLeft') {
        if (voicings.length > 0) {
          e.preventDefault();
          onSelectIndex((selectedIndex - 1 + voicings.length) % voicings.length);
        }
      } else if (e.key === 'ArrowRight') {
        if (voicings.length > 0) {
          e.preventDefault();
          onSelectIndex((selectedIndex + 1) % voicings.length);
        }
      } else if (e.code === 'Space') {
        if (activeVoicing) {
          e.preventDefault();
          onPlay(activeVoicing, 'strum');
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [voicings.length, selectedIndex, activeVoicing, onSelectIndex, onPlay]);

  const placeholderVoicing: VoicingResult = {
    frets: Array.from({ length: instrument.tuning.length }, () => null),
    fingers: Array.from({ length: instrument.tuning.length }, () => null),
    barre: null,
    notes: [],
    min_fret: 0,
    max_fret: 0,
    score: 0,
  };

  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-12">
      {/* Main Featured Diagram */}
      <section className="flex flex-col gap-4 rounded-2xl border border-stone-800/90 bg-stone-900/75 p-4 shadow-xl shadow-black/25 backdrop-blur-sm sm:p-5 lg:col-span-5">
        <div className="flex flex-wrap items-center justify-between gap-2 border-b border-stone-800/80 pb-3">
          <div>
            <h2 className="text-sm font-semibold text-stone-100">
              {chordSpec ? chordSpec.symbol : 'Diagrama'}
              {voicings.length > 0 && (
                <span className="ml-2 font-mono text-xs font-normal text-stone-400">
                  #{selectedIndex + 1} de {voicings.length}
                </span>
              )}
            </h2>
            <p className="text-[11px] text-stone-500">
              Clique em qualquer casa ou bolinha para travar/destravar uma nota
            </p>
          </div>

          {/* Orientation Toggle */}
          <div className="inline-flex rounded-lg border border-stone-800 bg-stone-950 p-0.5">
            <button
              type="button"
              onClick={() => onOrientationChange('vertical')}
              className={`rounded-md px-2.5 py-1 text-xs font-medium transition ${
                orientation === 'vertical'
                  ? 'bg-stone-800 text-amber-300'
                  : 'text-stone-400 hover:text-stone-200'
              }`}
            >
              Vertical
            </button>
            <button
              type="button"
              onClick={() => onOrientationChange('horizontal')}
              className={`rounded-md px-2.5 py-1 text-xs font-medium transition ${
                orientation === 'horizontal'
                  ? 'bg-stone-800 text-amber-300'
                  : 'text-stone-400 hover:text-stone-200'
              }`}
            >
              Horizontal
            </button>
          </div>
        </div>

        {/* Label Mode Switcher */}
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="inline-flex rounded-lg border border-stone-800 bg-stone-950 p-0.5">
            {(
              [
                { id: 'fingers', label: 'Dedos (1–4)' },
                { id: 'notes', label: 'Notas' },
                { id: 'intervals', label: 'Intervalos' },
              ] as { id: LabelMode; label: string }[]
            ).map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => onLabelModeChange(item.id)}
                className={`rounded-md px-2.5 py-1 text-xs font-medium transition ${
                  labelMode === item.id
                    ? 'bg-amber-500/20 text-amber-300'
                    : 'text-stone-400 hover:text-stone-200'
                }`}
              >
                {item.label}
              </button>
            ))}
          </div>

          {activePinCount > 0 && (
            <button
              type="button"
              onClick={onClearPins}
              className="inline-flex items-center gap-1.5 rounded-lg border border-emerald-500/40 bg-emerald-500/15 px-2.5 py-1 text-xs font-medium text-emerald-300 transition hover:bg-emerald-500/25"
            >
              <span>Limpar travas ({activePinCount})</span>
            </button>
          )}
        </div>

        {/* Interactive Fretboard SVG */}
        <div className="flex min-h-[290px] items-center justify-center rounded-xl border border-stone-800/80 bg-stone-950/60 p-3">
          <FretboardSvg
            instrument={instrument}
            voicing={activeVoicing ?? placeholderVoicing}
            orientation={orientation}
            labelMode={labelMode}
            pinnedFrets={pinnedFrets}
            interactive
            onTogglePin={onTogglePin}
          />
        </div>

        {/* Voicing Metadata & Playback Controls */}
        {activeVoicing ? (
          <div className="flex flex-col gap-3">
            <div className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-stone-800 bg-stone-950/70 px-3 py-2">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-mono text-sm font-bold tracking-wider text-stone-100">
                  {formatFretsBadge(activeVoicing.frets)}
                </span>
                {activeVoicing.barre && (
                  <span className="rounded-md bg-sky-500/15 px-2 py-0.5 text-[11px] font-medium text-sky-300">
                    Pestana casa {activeVoicing.barre.fret}
                  </span>
                )}
              </div>
              <span className="font-mono text-[11px] text-stone-400">
                Score {activeVoicing.score.toFixed(1)}
              </span>
            </div>

            <div className="grid grid-cols-2 gap-2">
              <button
                type="button"
                onClick={() => onPlay(activeVoicing, 'strum')}
                className="flex items-center justify-center gap-2 rounded-xl bg-amber-500 px-3.5 py-2.5 text-sm font-semibold text-stone-950 shadow-lg shadow-amber-500/15 transition hover:bg-amber-400 active:scale-[0.98]"
              >
                <PlayIcon />
                <span>Rasgueado</span>
              </button>
              <button
                type="button"
                onClick={() => onPlay(activeVoicing, 'arpeggio')}
                className="flex items-center justify-center gap-2 rounded-xl border border-stone-700 bg-stone-800 px-3.5 py-2.5 text-sm font-semibold text-stone-100 transition hover:border-amber-500/50 hover:bg-stone-700 active:scale-[0.98]"
              >
                <ArpeggioIcon />
                <span>Arpejo</span>
              </button>
            </div>

            <div className="flex items-center justify-between text-[11px] text-stone-500">
              <div className="flex items-center gap-3">
                <span className="inline-flex items-center gap-1">
                  <span className="h-2.5 w-2.5 rounded-full bg-amber-400" />
                  Tônica
                </span>
                <span className="inline-flex items-center gap-1">
                  <span className="h-2.5 w-2.5 rounded-full bg-sky-500" />
                  Nota do acorde
                </span>
                <span className="inline-flex items-center gap-1">
                  <span className="h-2.5 w-2.5 rounded-full border border-dashed border-emerald-400" />
                  Trava
                </span>
              </div>
              <span>← / → alterna · Espaço toca</span>
            </div>
          </div>
        ) : (
          <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-200">
            <p className="font-semibold">Nenhum voicing encontrado para os filtros atuais.</p>
            <p className="mt-1 text-amber-300/80">
              {activePinCount > 0
                ? 'Tente destravar uma das notas marcadas ou ampliar a abertura máxima.'
                : instrument.tuning.length === 4
                  ? 'Em instrumentos de 4 cordas, tente ativar "Rootless (4 cordas)" ou "Omitir 5ª".'
                  : 'Tente ampliar a região do braço, liberar cordas soltas ou aumentar a abertura máxima.'}
            </p>
          </div>
        )}
      </section>

      {/* Navigable Grid of Mini Voicings */}
      <section className="flex flex-col gap-4 rounded-2xl border border-stone-800/90 bg-stone-900/75 p-4 shadow-xl shadow-black/25 backdrop-blur-sm sm:p-5 lg:col-span-7">
        <div className="flex items-center justify-between border-b border-stone-800/80 pb-3">
          <div>
            <h2 className="text-sm font-semibold text-stone-100">
              Voicings Calculados
            </h2>
            <p className="text-[11px] text-stone-500">
              Ordenados por conforto biomecânico, ressonância e distribuição de vozes
            </p>
          </div>

          <div className="flex items-center gap-1.5">
            <button
              type="button"
              disabled={voicings.length === 0}
              onClick={() =>
                onSelectIndex((selectedIndex - 1 + voicings.length) % voicings.length)
              }
              className="rounded-lg border border-stone-700/80 bg-stone-800/80 px-2.5 py-1 font-mono text-xs text-stone-200 transition hover:bg-stone-700 disabled:opacity-40"
              title="Voicing anterior (←)"
            >
              ←
            </button>
            <button
              type="button"
              disabled={voicings.length === 0}
              onClick={() => onSelectIndex((selectedIndex + 1) % voicings.length)}
              className="rounded-lg border border-stone-700/80 bg-stone-800/80 px-2.5 py-1 font-mono text-xs text-stone-200 transition hover:bg-stone-700 disabled:opacity-40"
              title="Próximo voicing (→)"
            >
              →
            </button>
          </div>
        </div>

        {voicings.length === 0 ? (
          <div className="flex flex-1 items-center justify-center rounded-xl border border-dashed border-stone-800 py-16 text-center text-sm text-stone-500">
            Ajuste a cifra ou os filtros para listar digitações compatíveis.
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 xl:grid-cols-4">
            {voicings.map((voicing, idx) => {
              const isSelected = idx === selectedIndex;
              return (
                <button
                  key={`${formatFretsBadge(voicing.frets)}-${idx}`}
                  type="button"
                  onClick={() => onSelectIndex(idx)}
                  className={`group flex flex-col items-center gap-2 rounded-xl border p-2.5 text-left transition ${
                    isSelected
                      ? 'border-amber-400/80 bg-amber-500/10 ring-1 ring-amber-400/40'
                      : 'border-stone-800 bg-stone-950/60 hover:border-stone-700 hover:bg-stone-900'
                  }`}
                >
                  <div className="flex w-full items-center justify-between text-[10px]">
                    <span
                      className={`font-mono font-bold ${
                        isSelected ? 'text-amber-300' : 'text-stone-400'
                      }`}
                    >
                      #{idx + 1}
                    </span>
                    <span className="font-mono text-stone-500">
                      {voicing.min_fret === 0 && voicing.max_fret === 0
                        ? 'Solto'
                        : `${voicing.min_fret}–${voicing.max_fret}ª`}
                    </span>
                  </div>

                  <div className="h-28 w-full">
                    <FretboardSvg
                      instrument={instrument}
                      voicing={voicing}
                      orientation="vertical"
                      mini
                    />
                  </div>

                  <div className="w-full truncate text-center font-mono text-[11px] font-medium text-stone-300 group-hover:text-white">
                    {formatFretsBadge(voicing.frets)}
                  </div>
                </button>
              );
            })}
          </div>
        )}
      </section>
    </div>
  );
}

function PlayIcon() {
  return (
    <svg viewBox="0 0 20 20" fill="currentColor" className="h-4 w-4">
      <path d="M6.3 3.7A1.2 1.2 0 0 0 4.5 4.75v10.5a1.2 1.2 0 0 0 1.8 1.05l8.9-5.25a1.2 1.2 0 0 0 0-2.1L6.3 3.7Z" />
    </svg>
  );
}

function ArpeggioIcon() {
  return (
    <svg
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      className="h-4 w-4"
    >
      <path d="M4 14v-2M8 14V9M12 14V6M16 14V4" />
    </svg>
  );
}
