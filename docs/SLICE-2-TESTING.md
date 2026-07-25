# Slice 2 Windows creator testing

Use a disposable WinReclaim project with the silent ~3:40 recording, the voiced ~2:17 version when available, and saved project Context.

## Launch

```powershell
git pull
npm test
cargo fmt --manifest-path .\src-tauri\Cargo.toml -- --check
cargo test --manifest-path .\src-tauri\Cargo.toml
npm run tauri:dev
```

## Local source understanding

- [ ] Build locally with OpenRouter unavailable; representative JPEGs appear under `Cache\analysis\<asset-id>`.
- [ ] Samples cover the opening, meaningful UI changes and late source sections without excessive near-duplicates.
- [ ] Local segments show ranges, descriptions, importance, decision, confidence and `local` provenance.
- [ ] Close and reopen the project; frames and semantic sections persist.

## Remote semantic analysis

- [ ] The disclosure shows the exact model, sources, frames/timestamps, semantic map, transcript, Context and instruction.
- [ ] It states that the source video is not uploaded.
- [ ] Cancel the disclosure; no request or remote result is created.
- [ ] The first remote semantic analysis requires the approval checkbox.
- [ ] An incompatible model fails clearly.
- [ ] Successful validated segments remain inside selected source durations and use `both` provenance.
- [ ] Reopen the project; remote results persist and the one-time approval is remembered.

## WinReclaim benchmark

- [ ] For the silent recording, ask for a clear two-minute walkthrough that removes waiting and preserves the safety story.
- [ ] Confirm the system uses visual frames and saved Context without inventing speech.
- [ ] Confirm uncertainty appears as warnings or lower confidence.
- [ ] For the voiced version, confirm transcript is disclosed and used alongside visuals.

## Assistant

- [ ] Source/Assistant/Plan switching does not disrupt playback.
- [ ] Select one or several sources and submit a natural-language goal.
- [ ] The user message persists immediately; progress and received characters visibly update.
- [ ] Cancel stops without a success plan; retry reopens disclosure.
- [ ] Conversations persist after reopening.
- [ ] Edentic says it created a plan and never claims it edited media.

## Plan review

- [ ] Inspect each source range, action, reason, pacing, warning and confidence.
- [ ] Play proposed ranges and confirm their start/stop timestamps.
- [ ] Toggle decisions, edit valid timestamps and reorder segments; reopen and confirm persistence.
- [ ] Invalid/out-of-source ranges are rejected.
- [ ] Regenerate one decision; other IDs, ordering and enabled states remain unchanged.
- [ ] Accept and reject change only persisted plan status.
- [ ] No timeline, render, voiceover or export is created.

## Recovery and removal

- [ ] Source removal is blocked during an active assistant request.
- [ ] Removing a disposable source clears its analysis frames and semantic rows.
- [ ] Reset cache clears rebuildable frames/maps but preserves Context, conversations, plans and originals.
- [ ] Repair invalidates semantic data whose derived source files are missing.

For failures send a screenshot/recording, exact action, expected/actual result, PowerShell/Tauri logs, source type/duration/resolution, selected model, and whether reopening changes the result.
