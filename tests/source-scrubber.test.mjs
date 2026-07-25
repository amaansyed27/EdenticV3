import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("the source waveform is a real mouse and keyboard scrubber", async () => {
  const [workspace, runtime, main, controls] = await Promise.all([
    readFile(new URL("../src/app/views/workspace.js", import.meta.url), "utf8"),
    readFile(new URL("../src/app/workspace-runtime.js", import.meta.url), "utf8"),
    readFile(new URL("../src/main.js", import.meta.url), "utf8"),
    readFile(new URL("../src/styles/workspace-controls.css", import.meta.url), "utf8"),
  ]);

  assert.match(workspace, /data-source-scrubber/);
  assert.match(workspace, /role="slider"/);
  assert.match(workspace, /waveform-progress/);
  assert.match(runtime, /pointerdown/);
  assert.match(runtime, /pointermove/);
  assert.match(runtime, /setPointerCapture/);
  assert.match(runtime, /ArrowLeft/);
  assert.match(runtime, /event\.code === "Space"/);
  assert.match(runtime, /target instanceof HTMLMediaElement/);
  assert.match(runtime, /"toggle-play"/);
  assert.match(runtime, /export function stopWorkspacePlayback/);
  assert.doesNotMatch(runtime, /target instanceof HTMLVideoElement/);
  assert.match(main, /selectedAssetChanged/);
  assert.match(main, /stopWorkspacePlayback\(\)/);
  assert.match(controls, /cursor: ew-resize/);
  assert.match(controls, /touch-action: none/);
});
