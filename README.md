# voicr

Aplicação web 100% client-side para cálculo algorítmico de *voicings* e visualização interativa de braços de instrumentos de corda (**Violão**, **Cavaquinho**, **Viola Caipira** e afinações customizadas), alimentada por um motor matemático em **Rust + WebAssembly** e síntese de áudio **Karplus-Strong** nativa via Web Audio API.

## Arquitetura

```text
voicr/
├── Cargo.toml                  # Cargo Workspace (fret-core, fret-wasm)
├── package.json                # Scripts integrados (build:wasm, dev, build, test:rust)
├── crates/
│   ├── fret-core/              # Motor puro em Rust (zero deps de SO/JS/WASM)
│   │   └── src/
│   │       ├── pitch.rs        # PitchClass (Z_12), MidiNote e ortografia enarmônica
│   │       ├── chord.rs        # Parser de cifras complexas -> ChordSpec
│   │       ├── instrument.rs   # Presets (Violão, Cavaquinho, Viola Caipira) e Custom
│   │       ├── solver.rs       # Backtracking biomecânico, pestana e orçamento de 4 dedos
│   │       └── scoring.rs      # Heurística de conforto, densidade e ranqueamento
│   └── fret-wasm/              # Ponte wasm-bindgen + serde-wasm-bindgen
└── web/                        # Frontend Vite + React + TypeScript + Tailwind CSS
    └── src/
        ├── types/fret.ts
        ├── audio/karplusStrong.ts
        └── components/
            ├── ChordInputBar.tsx
            ├── InstrumentControls.tsx
            ├── FretboardSvg.tsx
            └── VoicingGrid.tsx
```

## Pré-requisitos

- **Rust** com target `wasm32-unknown-unknown` e `wasm-bindgen-cli`:
  ```bash
  rustup target add wasm32-unknown-unknown
  cargo install wasm-bindgen-cli --version 0.2.130
  ```
- **Node.js** (18+) e **npm**

## Instalação e Scripts

Instale as dependências do frontend em `web/`:

```bash
npm --prefix web install
```

### Scripts disponíveis na raiz

- `npm run build:wasm`: Compila `crates/fret-wasm` em `--release` para `wasm32-unknown-unknown` e gera os bindings JS/TS em `web/src/wasm`.
- `npm run dev`: Compila o módulo WASM e inicia o servidor de desenvolvimento Vite.
- `npm run dev:web`: Inicia apenas o Vite (quando o WASM já foi compilado).
- `npm run build`: Compila o WASM e gera o bundle de produção em `web/dist`.
- `npm run test:rust`: Executa os testes unitários do motor `fret-core` (`cargo test --workspace`).

## Recursos

- **Parser de cifras complexas**: tríades, `sus2`, `sus4`, `6`, `6/9`, `7`, `maj7`, `m7`, `m(maj7)`, `dim7`, `m7(b5)`, tensões (`9`, `b9`, `#9`, `11`, `#11`, `13`, `b13`), alterações (`b5`, `#5`, `alt`) e baixos invertidos (`/E`, `/G#`).
- **Solver biomecânico**: poda antecipada por extensão de casas (`max_span`), simulação automática de pestana (`Barre`) e orçamento de no máximo 4 dedos.
- **Trava Interativa (Pin Note)**: clique em qualquer casa/corda do diagrama principal para travar ou destravar uma nota e recalcular os *voicings* instantaneamente.
- **Áudio Karplus-Strong**: síntese de cordas dedilhadas sem samples externos, com suporte a **Rasgueado** (~35ms), **Arpejo** (~180ms) e oitavas nos pares graves da Viola Caipira.
- **Atalhos de teclado**: `←` / `→` navegam entre *voicings*; `Espaço` (fora do campo de texto) toca o acorde atual.
