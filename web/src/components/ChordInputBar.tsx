import { useEffect, useRef } from 'react';
import type { ChordSpec } from '../types/fret';

interface ChordInputBarProps {
  query: string;
  onQueryChange: (next: string) => void;
  chordSpec: ChordSpec | null;
  parseError: string | null;
  omitFifth: boolean;
}

const QUICK_TOKENS = ['#', 'b', 'm7', 'maj7', '7(b9)', 'm7(b5)', '6/9', 'sus4', '#11'];

const EXAMPLE_CHORDS = ['Cmaj9(#11)', 'F#m7(b5)/C', 'G7(b9)', 'D6/9', 'Am(maj7)', 'E7alt'];

export function ChordInputBar({
  query,
  onQueryChange,
  chordSpec,
  parseError,
  omitFifth,
}: ChordInputBarProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const handleInsertToken = (token: string) => {
    const el = inputRef.current;
    if (!el) {
      onQueryChange(query + token);
      return;
    }
    const start = el.selectionStart ?? query.length;
    const end = el.selectionEnd ?? query.length;
    const nextValue = query.slice(0, start) + token + query.slice(end);
    onQueryChange(nextValue);

    requestAnimationFrame(() => {
      el.focus();
      const nextCaret = start + token.length;
      el.setSelectionRange(nextCaret, nextCaret);
    });
  };

  return (
    <section className="rounded-2xl border border-stone-800/90 bg-stone-900/75 p-4 shadow-xl shadow-black/25 backdrop-blur-sm sm:p-5">
      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <label
            htmlFor="chord-input"
            className="text-xs font-semibold uppercase tracking-wider text-stone-400"
          >
            Cifra do Acorde
          </label>
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="text-[11px] text-stone-500">Exemplos:</span>
            {EXAMPLE_CHORDS.map((example) => (
              <button
                key={example}
                type="button"
                onClick={() => onQueryChange(example)}
                className={`rounded-md px-2 py-0.5 font-mono text-[11px] transition ${
                  query.trim() === example
                    ? 'bg-amber-500/20 text-amber-300 ring-1 ring-amber-500/40'
                    : 'bg-stone-800/80 text-stone-400 hover:bg-stone-800 hover:text-stone-200'
                }`}
              >
                {example}
              </button>
            ))}
          </div>
        </div>

        <div className="relative">
          <input
            ref={inputRef}
            id="chord-input"
            type="text"
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
            placeholder="Ex: Cmaj9(#11), F#m7(b5)/C, G7(b9)..."
            spellCheck={false}
            autoComplete="off"
            className={`w-full rounded-xl border bg-stone-950/90 px-4 py-3 pr-24 font-mono text-xl font-semibold tracking-tight text-stone-100 placeholder-stone-600 outline-none transition sm:text-2xl ${
              parseError
                ? 'border-amber-500/50 focus:border-amber-400 focus:ring-2 focus:ring-amber-500/20'
                : 'border-stone-700/80 focus:border-amber-400 focus:ring-2 focus:ring-amber-500/20'
            }`}
          />
          {query && (
            <button
              type="button"
              onClick={() => {
                onQueryChange('');
                inputRef.current?.focus();
              }}
              className="absolute right-3 top-1/2 -translate-y-1/2 rounded-lg border border-stone-700/70 bg-stone-800/80 px-2.5 py-1 text-xs font-medium text-stone-300 transition hover:bg-stone-700 hover:text-white"
            >
              Limpar
            </button>
          )}
        </div>

        {/* Quick symbol buttons */}
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="mr-1 text-[11px] font-medium text-stone-500">Símbolos:</span>
          {QUICK_TOKENS.map((token) => (
            <button
              key={token}
              type="button"
              onClick={() => handleInsertToken(token)}
              className="rounded-lg border border-stone-700/80 bg-stone-800/90 px-2.5 py-1 font-mono text-xs font-medium text-stone-200 transition hover:border-amber-500/50 hover:bg-stone-800 hover:text-amber-300 active:scale-95"
            >
              {token}
            </button>
          ))}
        </div>

        {/* Parsed chord tones or elegant error feedback */}
        <div className="min-h-[2rem] border-t border-stone-800/80 pt-3">
          {parseError ? (
            <div className="flex items-center gap-2 text-xs text-amber-300/90">
              <span className="inline-block h-2 w-2 rounded-full bg-amber-400" />
              <span>{parseError}</span>
            </div>
          ) : chordSpec ? (
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="flex flex-wrap items-center gap-1.5 text-sm">
                <span className="mr-1 rounded-md bg-stone-800 px-2 py-0.5 font-mono text-xs font-bold text-stone-200">
                  {chordSpec.symbol}
                </span>
                {chordSpec.tones.map((tone, idx) => {
                  const isRoot = tone.role === 'Root';
                  const isOptional = tone.role === 'Optional' && omitFifth;
                  return (
                    <span key={`${tone.interval_label}-${idx}`} className="inline-flex items-center">
                      {idx > 0 && <span className="mx-1.5 text-stone-600">·</span>}
                      <span
                        className={`inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 font-mono text-xs ${
                          isRoot
                            ? 'bg-amber-500/20 font-semibold text-amber-300 ring-1 ring-amber-500/40'
                            : isOptional
                              ? 'bg-stone-800/50 text-stone-400'
                              : 'bg-sky-500/10 text-sky-200'
                        }`}
                        title={
                          isOptional
                            ? '5ª justa (opcional com Omitir 5ª ativo)'
                            : tone.role === 'AlteredPool'
                              ? 'Tensão alterada disponível (7alt)'
                              : 'Nota do acorde'
                        }
                      >
                        <span>{tone.note_name}</span>
                        <span className="text-[11px] opacity-75">({tone.interval_label})</span>
                      </span>
                    </span>
                  );
                })}
                {chordSpec.slash_bass_name && (
                  <span className="ml-2 inline-flex items-center gap-1 rounded-md bg-emerald-500/15 px-2 py-0.5 font-mono text-xs font-medium text-emerald-300 ring-1 ring-emerald-500/30">
                    Baixo: {chordSpec.slash_bass_name}
                  </span>
                )}
              </div>
            </div>
          ) : null}
        </div>
      </div>
    </section>
  );
}
