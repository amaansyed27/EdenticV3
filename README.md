# Edentic V3

Edentic is a professional, local-first video editor where manual editing, focused AI assistance and goal-driven agentic editing will share one non-destructive project and timeline.

The product is built in verified slices. Slice 1 is Windows-verified. This repository adds **Slice 2 for creator testing**:

- first-run selection of the managed projects folder;
- a real project home with New, Open, Recents, search and view controls;
- self-contained project folders for originals, edit data, proxies, cache, autosaves and backups;
- durable project manifests and SQLite project storage;
- source-video import with FFmpeg/ffprobe metadata;
- local posters, waveforms, proxies, scene detection and optional Faster-Whisper transcription;
- a searchable, timestamp-linked Video Map;
- pasted or imported project context;
- dark gray/light-black and beige/gold themes without decorative gradients;
- Auto, GPU preferred, GPU + CPU and CPU-only processing modes;
- OpenRouter BYOK stored through the operating-system credential vault;
- `openrouter/free` as the initial default model.
- local scene-aware frame sampling and visual deduplication;
- local and approved remote semantic source maps persisted per project;
- exact disclosure of frames, semantic map, transcript, Context and instruction;
- a persistent assistant that produces strict structured edit plans;
- range playback, enable/disable, timestamp editing, reordering and single-decision regeneration;
- complete-plan acceptance or rejection without timeline changes.

Slice 2 produces reviewable plans only. Timeline editing, rendering, voiceover generation and export remain out of scope.

## Technology

- Tauri 2 desktop shell
- Rust native core
- Dependency-light modular HTML, CSS and JavaScript interface
- SQLite project data
- FFmpeg and ffprobe media processing
- Faster-Whisper local transcription through Python 3.12
- OpenRouter API with operating-system credential storage

## Run on Windows

Requirements:

- Node.js 20+
- Rust stable with the MSVC toolchain
- Microsoft C++ Build Tools and WebView2
- FFmpeg and ffprobe on `PATH`
- Python 3.12 for local transcription

```powershell
git clone https://github.com/amaansyed27/EdenticV3.git
Set-Location .\EdenticV3

npm test
py -3.12 -m pip install -r .\python\requirements.txt
npm run tauri:dev
```

The transcription model selected in Settings is downloaded by Faster-Whisper on its first use. Edentic still builds posters, waveforms, proxies and scene indexes when transcription is not installed.

## Verify Slice 2

Follow [docs/SLICE-2-TESTING.md](docs/SLICE-2-TESTING.md). The [Slice 1 checklist](docs/SLICE-1-TESTING.md) remains available for regressions.

The current module and storage boundaries are documented in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Development

```powershell
npm test
npm run build:frontend
cargo fmt --manifest-path .\src-tauri\Cargo.toml -- --check
cargo test --manifest-path .\src-tauri\Cargo.toml
```

Read [AGENTS.md](AGENTS.md) before continuing implementation.
