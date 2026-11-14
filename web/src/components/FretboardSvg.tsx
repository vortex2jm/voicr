import type {
  FretOrientation,
  InstrumentConfig,
  LabelMode,
  VoicingNote,
  VoicingResult,
} from '../types/fret';

interface FretboardSvgProps {
  instrument: InstrumentConfig;
  voicing: VoicingResult;
  orientation?: FretOrientation;
  labelMode?: LabelMode;
  pinnedFrets?: (number | null)[];
  interactive?: boolean;
  mini?: boolean;
  onTogglePin?: (stringIndex: number, fret: number) => void;
}

const SINGLE_DOT_FRETS = new Set([3, 5, 7, 9, 15, 17, 19, 21]);
const DOUBLE_DOT_FRETS = new Set([12, 24]);

export function FretboardSvg({
  instrument,
  voicing,
  orientation = 'vertical',
  labelMode = 'fingers',
  pinnedFrets = [],
  interactive = false,
  mini = false,
  onTogglePin,
}: FretboardSvgProps) {
  const numStrings = instrument.tuning.length;

  // Determine fret window
  const activeFrets: number[] = [];
  voicing.frets.forEach((f) => {
    if (f !== null && f > 0) activeFrets.push(f);
  });
  pinnedFrets.forEach((f) => {
    if (f !== null && f > 0) activeFrets.push(f);
  });

  const minActive = activeFrets.length > 0 ? Math.min(...activeFrets) : 1;
  const maxActive = activeFrets.length > 0 ? Math.max(...activeFrets) : 4;

  const startFret = maxActive <= 5 ? 1 : minActive;
  const minWindow = mini ? 4 : 5;
  const neededFrets = Math.max(minWindow, maxActive - startFret + 1);
  const fretCount = Math.min(neededFrets, Math.max(1, instrument.num_frets - startFret + 1));

  const noteByString = new Map<number, VoicingNote>();
  voicing.notes.forEach((n) => noteByString.set(n.string_index, n));

  // Layout geometry
  const stringGap = mini ? 20 : 38;
  const fretGap = mini ? 22 : 44;
  const boardSpan = (numStrings - 1) * stringGap;
  const boardLen = fretCount * fretGap;

  const headerPad = mini ? 20 : 44; // Space for X / O markers & nut
  const sidePad = mini ? 18 : 44; // Space for fret numbers ("5ª") & margins
  const tailPad = mini ? 12 : 28; // Space for open string note labels / padding

  const isVertical = orientation === 'vertical';
  const width = isVertical
    ? sidePad * 2 + boardSpan
    : headerPad + boardLen + tailPad;
  const height = isVertical
    ? headerPad + boardLen + tailPad
    : sidePad * 2 + boardSpan;

  const boardX0 = isVertical ? sidePad : headerPad;
  const boardY0 = isVertical ? headerPad : sidePad;

  // Map string index (0 = lowest/bass, numStrings-1 = highest/treble) to coordinate
  const stringCoord = (s: number): number => {
    if (isVertical) {
      return boardX0 + s * stringGap;
    }
    // Horizontal (player's perspective): highest string on top, lowest string at bottom
    return boardY0 + (numStrings - 1 - s) * stringGap;
  };

  // Coordinate of fret wire i (0 = nut/start boundary, fretCount = end)
  const fretWireCoord = (relFret: number): number => {
    return (isVertical ? boardY0 : boardX0) + relFret * fretGap;
  };

  // Center coordinate inside fret cell `actualFret`
  const fretCellCenter = (actualFret: number): number => {
    const rel = actualFret - startFret + 1;
    return fretWireCoord(rel) - fretGap / 2;
  };

  const dotRadius = mini ? 7 : 13.5;

  const getDotLabel = (note: VoicingNote): string => {
    if (labelMode === 'fingers') {
      return note.finger !== null ? String(note.finger) : '';
    }
    if (labelMode === 'notes') {
      return note.note_name;
    }
    return note.interval;
  };

  const renderInlays = () => {
    const inlays: React.ReactNode[] = [];
    for (let i = 0; i < fretCount; i++) {
      const actualFret = startFret + i;
      const centerFret = fretWireCoord(i + 1) - fretGap / 2;
      const midString = isVertical
        ? boardX0 + boardSpan / 2
        : boardY0 + boardSpan / 2;

      if (SINGLE_DOT_FRETS.has(actualFret)) {
        inlays.push(
          <circle
            key={`inlay-${actualFret}`}
            cx={isVertical ? midString : centerFret}
            cy={isVertical ? centerFret : midString}
            r={mini ? 2.2 : 4}
            className="fill-stone-700/70"
          />,
        );
      } else if (DOUBLE_DOT_FRETS.has(actualFret)) {
        const offset = Math.max(stringGap * 0.85, boardSpan * 0.25);
        inlays.push(
          <g key={`inlay-${actualFret}`}>
            <circle
              cx={isVertical ? midString - offset : centerFret}
              cy={isVertical ? centerFret : midString - offset}
              r={mini ? 2 : 3.8}
              className="fill-stone-700/70"
            />
            <circle
              cx={isVertical ? midString + offset : centerFret}
              cy={isVertical ? centerFret : midString + offset}
              r={mini ? 2 : 3.8}
              className="fill-stone-700/70"
            />
          </g>,
        );
      }
    }
    return inlays;
  };

  const renderBarre = () => {
    const barre = voicing.barre;
    if (!barre) return null;
    if (barre.fret < startFret || barre.fret >= startFret + fretCount) return null;

    const cFrom = stringCoord(barre.from_string);
    const cTo = stringCoord(barre.to_string);
    const cMin = Math.min(cFrom, cTo);
    const cMax = Math.max(cFrom, cTo);
    const fCenter = fretCellCenter(barre.fret);
    const pad = dotRadius * 0.85;
    const thickness = dotRadius * 1.55;

    if (isVertical) {
      return (
        <rect
          x={cMin - pad}
          y={fCenter - thickness / 2}
          width={cMax - cMin + pad * 2}
          height={thickness}
          rx={thickness / 2}
          className="fill-sky-500/35 stroke-sky-400/80"
          strokeWidth={mini ? 1 : 1.5}
        />
      );
    }

    return (
      <rect
        x={fCenter - thickness / 2}
        y={cMin - pad}
        width={thickness}
        height={cMax - cMin + pad * 2}
        rx={thickness / 2}
        className="fill-sky-500/35 stroke-sky-400/80"
        strokeWidth={mini ? 1 : 1.5}
      />
    );
  };

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      className={`select-none ${mini ? 'h-full w-full' : 'max-h-[420px] w-full'}`}
      role="img"
      aria-label="Diagrama do braço do instrumento"
    >
      {/* Fretboard Wood Surface */}
      <rect
        x={boardX0 - (isVertical ? 8 : 0)}
        y={boardY0 - (isVertical ? 0 : 8)}
        width={isVertical ? boardSpan + 16 : boardLen}
        height={isVertical ? boardLen : boardSpan + 16}
        rx={mini ? 4 : 6}
        className="fill-stone-900/95 stroke-stone-800"
        strokeWidth={1}
      />

      {/* Fret inlays */}
      {renderInlays()}

      {/* Fret wires */}
      {Array.from({ length: fretCount + 1 }, (_, i) => {
        const coord = fretWireCoord(i);
        const isNut = i === 0 && startFret === 1;
        return isVertical ? (
          <line
            key={`fret-${i}`}
            x1={boardX0 - 6}
            x2={boardX0 + boardSpan + 6}
            y1={coord}
            y2={coord}
            className={isNut ? 'stroke-stone-200' : 'stroke-stone-600/80'}
            strokeWidth={isNut ? (mini ? 3 : 5) : mini ? 1 : 1.5}
            strokeLinecap="round"
          />
        ) : (
          <line
            key={`fret-${i}`}
            x1={coord}
            x2={coord}
            y1={boardY0 - 6}
            y2={boardY0 + boardSpan + 6}
            className={isNut ? 'stroke-stone-200' : 'stroke-stone-600/80'}
            strokeWidth={isNut ? (mini ? 3 : 5) : mini ? 1 : 1.5}
            strokeLinecap="round"
          />
        );
      })}

      {/* Starting fret label when shifted above the nut */}
      {startFret > 1 && (
        <text
          x={isVertical ? boardX0 - (mini ? 12 : 22) : fretWireCoord(0) + fretGap / 2}
          y={isVertical ? fretWireCoord(0) + fretGap / 2 : boardY0 - (mini ? 12 : 20)}
          textAnchor="middle"
          dominantBaseline="central"
          className={`fill-amber-300 font-mono font-bold ${mini ? 'text-[9px]' : 'text-xs'}`}
        >
          {startFret}ª
        </text>
      )}

      {/* Strings (bass thicker -> treble thinner) */}
      {Array.from({ length: numStrings }, (_, s) => {
        const sc = stringCoord(s);
        const bassRatio = (numStrings - 1 - s) / Math.max(1, numStrings - 1);
        const strokeWidth = mini ? 0.9 + bassRatio * 0.9 : 1.15 + bassRatio * 1.55;
        return isVertical ? (
          <line
            key={`str-${s}`}
            x1={sc}
            x2={sc}
            y1={boardY0}
            y2={boardY0 + boardLen}
            className="stroke-stone-400/75"
            strokeWidth={strokeWidth}
          />
        ) : (
          <line
            key={`str-${s}`}
            x1={boardX0}
            x2={boardX0 + boardLen}
            y1={sc}
            y2={sc}
            className="stroke-stone-400/75"
            strokeWidth={strokeWidth}
          />
        );
      })}

      {/* Barre indicator */}
      {renderBarre()}

      {/* Header markers: X (muted), O (open), and clickable open-string pin zones */}
      {Array.from({ length: numStrings }, (_, s) => {
        const sc = stringCoord(s);
        const fret = voicing.frets[s];
        const note = noteByString.get(s);
        const isPinnedOpen = pinnedFrets[s] === 0;
        const markerCoord = isVertical ? boardY0 - (mini ? 10 : 21) : boardX0 - (mini ? 10 : 21);
        const cx = isVertical ? sc : markerCoord;
        const cy = isVertical ? markerCoord : sc;

        return (
          <g
            key={`head-${s}`}
            onClick={() => interactive && onTogglePin?.(s, 0)}
            className={interactive ? 'cursor-pointer' : undefined}
          >
            {interactive && (
              <circle
                cx={cx}
                cy={cy}
                r={14}
                fill="transparent"
                className="hover:fill-stone-800/60"
              />
            )}

            {isPinnedOpen && !mini && (
              <circle
                cx={cx}
                cy={cy}
                r={12.5}
                fill="none"
                className="stroke-emerald-400"
                strokeWidth={2}
                strokeDasharray="3 2"
              />
            )}

            {fret === null ? (
              <text
                x={cx}
                y={cy}
                textAnchor="middle"
                dominantBaseline="central"
                className={`fill-stone-500 font-mono font-bold ${mini ? 'text-[10px]' : 'text-sm'}`}
              >
                ×
              </text>
            ) : fret === 0 ? (
              <g>
                <circle
                  cx={cx}
                  cy={cy}
                  r={mini ? 4.2 : labelMode === 'fingers' ? 7 : 10.5}
                  className={
                    note?.is_root
                      ? 'fill-amber-500/20 stroke-amber-400'
                      : 'fill-sky-500/15 stroke-sky-400'
                  }
                  strokeWidth={mini ? 1.5 : 2}
                />
                {!mini && labelMode !== 'fingers' && note && (
                  <text
                    x={cx}
                    y={cy}
                    textAnchor="middle"
                    dominantBaseline="central"
                    className={`font-mono text-[9.5px] font-bold ${
                      note.is_root ? 'fill-amber-300' : 'fill-sky-200'
                    }`}
                  >
                    {labelMode === 'notes' ? note.note_name : note.interval}
                  </text>
                )}
              </g>
            ) : null}
          </g>
        );
      })}

      {/* Interactive hover/pin targets + Fretted dots */}
      {Array.from({ length: numStrings }, (_, s) => {
        const sc = stringCoord(s);
        const assignedFret = voicing.frets[s];
        const note = noteByString.get(s);

        return Array.from({ length: fretCount }, (_, i) => {
          const actualFret = startFret + i;
          const fc = fretCellCenter(actualFret);
          const cx = isVertical ? sc : fc;
          const cy = isVertical ? fc : sc;
          const isOccupied = assignedFret === actualFret && note !== undefined;
          const isPinned = pinnedFrets[s] === actualFret;

          return (
            <g
              key={`cell-${s}-${actualFret}`}
              onClick={() => interactive && onTogglePin?.(s, actualFret)}
              className={interactive ? 'cursor-pointer group' : undefined}
            >
              {/* Hit area for interactive pinning */}
              {interactive && (
                <circle
                  cx={cx}
                  cy={cy}
                  r={17}
                  fill="transparent"
                  className={!isOccupied ? 'group-hover:fill-stone-700/45' : undefined}
                />
              )}

              {/* Subtle ghost pin target on hover */}
              {interactive && !isOccupied && (
                <circle
                  cx={cx}
                  cy={cy}
                  r={5}
                  className="fill-transparent transition group-hover:fill-emerald-400/50"
                />
              )}

              {/* Fretted note dot */}
              {isOccupied && note && (
                <g>
                  <circle
                    cx={cx}
                    cy={cy}
                    r={dotRadius}
                    className={
                      note.is_root
                        ? 'fill-amber-400 stroke-amber-200'
                        : 'fill-sky-500 stroke-sky-300'
                    }
                    strokeWidth={mini ? 1 : 1.5}
                  />
                  {!mini && (
                    <text
                      x={cx}
                      y={cy}
                      textAnchor="middle"
                      dominantBaseline="central"
                      className={`font-mono font-bold ${
                        note.is_root ? 'fill-stone-950' : 'fill-white'
                      } ${getDotLabel(note).length >= 3 ? 'text-[9.5px]' : 'text-[11px]'}`}
                    >
                      {getDotLabel(note)}
                    </text>
                  )}
                </g>
              )}

              {/* Visual pin ring */}
              {isPinned && !mini && (
                <circle
                  cx={cx}
                  cy={cy}
                  r={dotRadius + 3.5}
                  fill="none"
                  className="stroke-emerald-400"
                  strokeWidth={2.2}
                  strokeDasharray="4 2.5"
                />
              )}
            </g>
          );
        });
      })}

      {/* Fret numbers along bottom/right edge on main diagram */}
      {!mini &&
        Array.from({ length: fretCount }, (_, i) => {
          const actualFret = startFret + i;
          const fc = fretCellCenter(actualFret);
          return (
            <text
              key={`fnum-${actualFret}`}
              x={isVertical ? boardX0 + boardSpan + 22 : fc}
              y={isVertical ? fc : boardY0 + boardSpan + 22}
              textAnchor="middle"
              dominantBaseline="central"
              className="fill-stone-500 font-mono text-[10px]"
            >
              {actualFret}
            </text>
          );
        })}
    </svg>
  );
}
