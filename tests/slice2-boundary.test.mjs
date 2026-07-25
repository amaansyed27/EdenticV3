import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const source = (path) => readFile(new URL(path, import.meta.url), "utf8");

test("Slice 2 commands cross frontend and native boundaries", async () => {
  const [api, native] = await Promise.all([source("../src/app/api.js"), source("../src-tauri/src/lib.rs")]);
  for (const command of ["build_local_semantic_map","prepare_semantic_analysis","run_semantic_analysis",
    "prepare_assistant_request","prepare_decision_regeneration","start_assistant_request",
    "get_ai_jobs","cancel_ai_job","save_edit_plan","set_edit_plan_status"]) {
    assert.match(api, new RegExp(`invoke\\("${command}"`));
    assert.match(native, new RegExp(command));
  }
});

test("semantic analysis is derived, disclosed and strict", async () => {
  const [semantic, storage, view] = await Promise.all([
    source("../src-tauri/src/semantic.rs"), source("../src-tauri/src/slice2_storage.rs"),
    source("../src/app/views/intelligence.js"),
  ]);
  assert.match(semantic, /scale=8:8,format=gray/);
  assert.match(semantic, /count_ones\(\) <= 7/);
  assert.match(semantic, /"type":"image_url"/);
  assert.match(semantic, /deny_unknown_fields/);
  assert.match(storage, /CREATE TABLE IF NOT EXISTS semantic_segments/);
  assert.match(storage, /CREATE TABLE IF NOT EXISTS remote_request_previews/);
  assert.match(view, /The source video is not uploaded/);
  assert.match(view, /Review exactly what will be sent/);
  assert.match(view, /Selected sources/);
  assert.match(view, /Semantic map/);
});

test("OpenRouter requests stream strict structured output", async () => {
  const [provider, assistant] = await Promise.all([
    source("../src-tauri/src/openrouter.rs"), source("../src-tauri/src/assistant.rs"),
  ]);
  assert.match(provider, /input_modalities/);
  assert.match(provider, /"type": "json_schema"/);
  assert.match(provider, /"strict": true/);
  assert.match(provider, /"stream": true/);
  assert.match(provider, /"require_parameters": true/);
  assert.match(assistant, /No source media or timeline was changed/);
});

test("structured plans are reviewable but do not edit", async () => {
  const [view, runtime, storage] = await Promise.all([
    source("../src/app/views/intelligence.js"), source("../src/app/assistant-runtime.js"),
    source("../src-tauri/src/slice2_storage.rs"),
  ]);
  for (const action of ["play-plan-range","move-plan-decision","regenerate-plan-decision",
    "set-plan-status","cancel-ai-job","retry-assistant"]) assert.match(view + runtime, new RegExp(action));
  assert.match(view, /data-plan-field="enabled"/);
  assert.match(view, /data-plan-field="start"/);
  assert.match(view, /no timeline changes applied/);
  assert.match(storage, /CREATE TABLE IF NOT EXISTS assistant_conversations/);
  assert.match(storage, /CREATE TABLE IF NOT EXISTS edit_plans/);
  assert.doesNotMatch(runtime, /applyTimeline|renderTimeline|destructiveCut/);
});
