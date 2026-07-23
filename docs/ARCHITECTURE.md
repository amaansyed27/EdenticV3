# Slice 1 architecture

## Product flow

```text
First launch
  -> choose managed projects folder
  -> project home
  -> create/open project
  -> import source media
  -> automatic local index job
  -> searchable Video Map
```

## Ownership

```text
Frontend
  src/app/api.js              typed native boundary and browser demo boundary
  src/app/state.js            session-only UI state
  src/app/render.js           action wiring and native workflow coordination
  src/app/views               home, workspace and settings views
  src/styles                  tokens and screen-specific styling

Native Rust
  commands.rs                 thin Tauri commands and job coordination
  storage.rs                  global settings, manifests and SQLite storage
  media.rs                    probing, managed imports and local indexing
  openrouter.rs               credential-vault access and provider API
  models.rs                   serialized contracts shared with the frontend

Local transcription
  python/transcribe.py        JSON-only Faster-Whisper subprocess
```

## Project layout

```text
Project Name/
  Media/
    Originals/
    Audio/
    Images/
    Generated/
  Edit/
    project.edentic
    project.sqlite
    index-data/
  Autosaves/
  Proxies/
  Cache/
    posters/
    waveforms/
    scenes/
  Backups/
```

Originals are managed copies and are never modified by processing. Posters, waveforms, scene thumbnails and proxies are derived artifacts. The authoritative Slice 1 project state is the manifest plus SQLite database.

## Processing modes

- `auto`: use CUDA for transcription/proxies when available, otherwise CPU.
- `gpu`: prioritize CUDA/NVENC; proxy rendering can fall back to CPU if the installed FFmpeg lacks NVENC.
- `hybrid`: GPU transcription/proxies plus CPU scene/audio/index work.
- `cpu`: do not initialize CUDA processing.

The UI reports detected hardware separately from the configured preference.

## Provider boundary

OpenRouter is configuration-only during Slice 1. The API key is stored using the operating-system credential vault. Key validation uses OpenRouter's authenticated `GET /api/v1/key` endpoint, and the model catalogue uses `GET /api/v1/models`.

No source video or project context is sent to OpenRouter in Slice 1.

