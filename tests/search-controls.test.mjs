import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("transcript search is handled persistently and exposes results", async () => {
  const [workspace, runtime, controls] = await Promise.all([
    readFile(new URL("../src/app/views/workspace.js", import.meta.url), "utf8"),
    readFile(new URL("../src/app/workspace-runtime.js", import.meta.url), "utf8"),
    readFile(new URL("../src/styles/workspace-controls.css", import.meta.url), "utf8"),
  ]);

  assert.match(workspace, /id="video-map-search"/);
  assert.match(workspace, /id="video-map-search-status"/);
  assert.match(runtime, /event\.target\.id === "video-map-search"/);
  assert.match(runtime, /row\.hidden = !matched/);
  assert.match(runtime, /matches === 1 \? "match" : "matches"/);
  assert.match(controls, /\.transcript-row\[hidden\]/);
});
