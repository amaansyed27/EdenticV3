import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("Slice 1 imports and renders video, audio and image sources", async () => {
  const [commands, media, indexing, workspace] = await Promise.all([
    readFile(new URL("../src-tauri/src/commands.rs", import.meta.url), "utf8"),
    readFile(new URL("../src-tauri/src/media.rs", import.meta.url), "utf8"),
    readFile(new URL("../src-tauri/src/indexing.rs", import.meta.url), "utf8"),
    readFile(new URL("../src/app/views/workspace.js", import.meta.url), "utf8"),
  ]);

  assert.match(commands, /add_filter\("Supported media"/);
  assert.match(commands, /"wav", "mp3", "m4a"/);
  assert.match(commands, /"png", "jpg", "jpeg", "webp"/);
  assert.match(media, /pub enum MediaKind/);
  assert.match(indexing, /kind == MediaKind::Audio/);
  assert.match(indexing, /kind == MediaKind::Image/);
  assert.match(workspace, /<audio id="source-player"/);
  assert.match(workspace, /class="source-image"/);
});

test("Slice 1 verifies settings and context persistence", async () => {
  const [storage, render, workspace] = await Promise.all([
    readFile(new URL("../src-tauri/src/storage.rs", import.meta.url), "utf8"),
    readFile(new URL("../src/app/render.js", import.meta.url), "utf8"),
    readFile(new URL("../src/app/views/workspace.js", import.meta.url), "utf8"),
  ]);

  assert.match(storage, /fn replace_file/);
  assert.match(storage, /Saved settings could not be verified/);
  assert.match(storage, /context_survives_a_fresh_database_connection/);
  assert.match(render, /settings: readSettingsForm\(\)/);
  assert.match(render, /await refreshProject\(\);/);
  assert.match(render, /Settings saved and verified/);
  assert.match(render, /action === "view-context"/);
  assert.match(render, /videoMapTab: "context"/);
  assert.match(workspace, /data-action="view-context"/);
  assert.match(workspace, /class="context-body"/);
  assert.match(workspace, /context\.content/);
});
