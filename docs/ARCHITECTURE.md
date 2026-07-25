# Slice 2 architecture

## Product flow

```text
Managed source import
  -> local Video/Audio Map
  -> scene-aware frame sampling and visual deduplication
  -> local semantic map
  -> exact remote disclosure and first-analysis approval
  -> validated remote semantic map
  -> persistent project assistant
  -> strict, reviewable edit plan
```

## Ownership

- `src/app/assistant-runtime.js`: semantic, assistant and plan workflow coordination.
- `src/app/views/intelligence.js`: Source/Assistant/Plan and disclosure views.
- `src-tauri/src/semantic.rs`: local sampling, deduplication, disclosure and semantic validation.
- `src-tauri/src/assistant.rs`: streamed assistant requests and structured plan validation.
- `src-tauri/src/slice2_storage.rs`: semantic, disclosure, conversation and plan persistence.
- `src-tauri/src/openrouter.rs`: credential vault, model capability checks and streamed structured output.

## Project data

`Cache\analysis\<asset-id>` contains rebuildable representative JPEGs. SQLite owns sampled-frame metadata, semantic segments, immutable request previews, first-analysis approval, assistant conversations/messages and edit plans.

Semantic segments store source asset, range, title, description, transcript excerpt, visual observations, importance, keep/remove/shorten suggestion, confidence and local/both provenance.

## Remote boundary

Edentic never sends the entire source video. Before every request, it persists and displays the exact selected-source metadata, JPEG frames and timestamps, semantic map, transcript, Context and instruction. The first remote semantic analysis needs additional explicit project approval.

OpenRouter model metadata is checked for image and structured-output support. `openrouter/free` uses provider-side required-parameter routing. Responses use strict JSON Schema and are validated again in Rust for unknown fields, enums, source IDs, timestamp bounds and confidence.

## Plan-only boundary

Assistant requests show live streamed progress and support cancellation and retry. Plans can be played, toggled, timestamp-edited, reordered, selectively regenerated, accepted or rejected. These actions only persist planning state; Slice 2 has no timeline mutation, rendering or export path.
