# Slice 1 creator testing

This checklist is for the first local test after cloning Edentic V3. It covers only Slice 1 and its bugs.

## Prerequisites

- Windows 10 or 11
- Rust stable with the MSVC toolchain
- Microsoft C++ Build Tools and WebView2
- Node.js 20 or newer
- Current FFmpeg and ffprobe on `PATH`
- Python 3.12 for optional local transcription

Install transcription support:

```powershell
py -3.12 -m pip install -r .\python\requirements.txt
```

The first transcription with a selected Faster-Whisper model downloads that model once. Scene detection, posters and waveforms still work when transcription is unavailable.

## Launch

```powershell
git clone https://github.com/amaansyed27/EdenticV3.git
Set-Location .\EdenticV3
npm test
npm run tauri:dev
```

## A. First launch and home

- [ ] First launch asks for the projects folder.
- [ ] Choosing a folder and continuing creates it when necessary.
- [ ] Edentic reopens to the home screen without asking again.
- [ ] Dark mode is warm dark gray/light black, not blue.
- [ ] Light mode is beige-white with gold accents.
- [ ] The official abstract logo fills the right onboarding stage without a white rectangle or clipping.
- [ ] A restrained sheen moves across the abstract logo and respects reduced-motion settings.
- [ ] The home screen includes the smaller animated abstract mark without crowding New Project.
- [ ] The home screen has New Project, Open Project, search, view controls, recent projects and Settings.
- [ ] No interface area looks like cards nested inside cards.

## B. Project creation and persistence

- [ ] Create a project with a name, format, resolution and frame rate.
- [ ] Confirm the project contains `Media`, `Edit`, `Autosaves`, `Proxies`, `Cache` and `Backups`.
- [ ] Confirm `Edit\project.edentic` and `Edit\project.sqlite` exist.
- [ ] Close Edentic, relaunch it and reopen the project from Recents.
- [ ] Open Project can open a valid project folder.
- [ ] An ordinary folder is rejected clearly.

## C. Media and Video Map

- [ ] Import an MP4 or MOV and confirm it is copied under `Media\Originals`.
- [ ] The source appears in Media with its real duration, resolution, size and codecs.
- [ ] The video plays in the main viewer.
- [ ] Build Video Map shows visible progress and supports cancellation.
- [ ] A poster, waveform and scene thumbnails are created.
- [ ] Clicking a scene jumps the viewer to the correct time.
- [ ] When transcription is installed, transcript segments appear and are searchable.
- [ ] Without transcription support, Edentic reports a partial index without discarding scenes or waveforms.
- [ ] Close and reopen the project; the index remains available.

## D. Context

- [ ] Paste context with a name and content.
- [ ] Import a `.txt` or `.md` context file.
- [ ] Context survives project reopening.
- [ ] Context stays inside the project database.

## E. Settings and providers

- [ ] Dark, Light and System themes work.
- [ ] Auto, GPU preferred, GPU + CPU and CPU-only processing modes persist.
- [ ] Hardware diagnostics show GPU, FFmpeg, ffprobe and Python status truthfully.
- [ ] Proxy quality, cache limit, job count and Whisper model persist.
- [ ] Save a valid OpenRouter key.
- [ ] Test Connection returns a real model count.
- [ ] Delete Key removes the stored credential.
- [ ] `openrouter/free` remains the default model.

## F. Recovery

Create and index a disposable project before testing these controls.

- [ ] Settings contains a Recovery section with Reset, Reset data, Reset cache, Repair and Reset all.
- [ ] Every action opens a confirmation dialog before changing anything.
- [ ] Reset restores preference defaults while preserving the selected projects folder, Recents and OpenRouter key.
- [ ] Reset data clears Recents and the OpenRouter key but leaves every project folder and original source file on disk.
- [ ] Reset cache removes `Cache`, `Proxies` and `Edit\index-data` contents, recreates the folder structure and marks sources for Video Map rebuilding.
- [ ] Reset cache does not remove `Media\Originals`, project context, manifests or SQLite project data.
- [ ] Repair recreates missing standard folders, checks project databases and removes missing projects from Recents.
- [ ] Reset all requires typing `RESET`.
- [ ] Reset all returns directly to first-launch onboarding.
- [ ] Reset all leaves project folders and `Media\Originals` intact.
- [ ] Reopen a preserved project folder after Reset data or Reset all and confirm it still works.

## What to send back for a failure

- Screenshot or screen recording
- Exact action that failed
- Expected and actual result
- PowerShell/Tauri terminal output
- Source file type, duration and resolution
- Whether the problem remains after reopening the project
