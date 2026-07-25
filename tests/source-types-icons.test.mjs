import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("source processing labels match video audio and image behavior", async () => {
  const [workspace, render, indexing] = await Promise.all([
    readFile(new URL("../src/app/views/workspace.js", import.meta.url), "utf8"),
    readFile(new URL("../src/app/render.js", import.meta.url), "utf8"),
    readFile(new URL("../src-tauri/src/indexing.rs", import.meta.url), "utf8"),
  ]);

  assert.match(workspace, /Build Video Map/);
  assert.match(workspace, /Build Audio Map/);
  assert.match(workspace, /Prepare Image/);
  assert.match(workspace, /Image Preview/);
  assert.match(render, /preparing locally/);
  assert.match(indexing, /MediaKind::Audio => "Audio Map"/);
  assert.match(indexing, /MediaKind::Image => "Image preview"/);
});

test("the application no longer renders the sparkle icon", async () => {
  const sources = await Promise.all([
    "../src/app/icons.js",
    "../src/app/render.js",
    "../src/app/views/home.js",
    "../src/app/views/workspace.js",
    "../src/app/views/overlays.js",
  ].map((path) => readFile(new URL(path, import.meta.url), "utf8")));

  for (const source of sources) {
    assert.doesNotMatch(source, /icon\("spark"/);
    assert.doesNotMatch(source, /paths\.spark/);
  }
});
