import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), "utf8");

test("workspace job updates do not rebuild the video element", async () => {
  const [state, main, workspace] = await Promise.all([
    read("src/app/state.js"),
    read("src/main.js"),
    read("src/app/views/workspace.js"),
  ]);
  assert.match(state, /listener\(state, patch\)/);
  assert.match(main, /applyWorkspaceTransientPatch/);
  assert.match(workspace, /Object\.hasOwn\(patch, "jobs"\)/);
  assert.match(workspace, /data-asset-id/);
});

test("media panels collapse and assets can be removed safely", async () => {
  const [workspace, runtime, api, nativeLib, nativeDelete] = await Promise.all([
    read("src/app/views/workspace.js"),
    read("src/app/workspace-runtime.js"),
    read("src/app/api.js"),
    read("src-tauri/src/lib.rs"),
    read("src-tauri/src/asset_commands.rs"),
  ]);
  assert.match(workspace, /toggle-media-panel/);
  assert.match(workspace, /toggle-video-map-panel/);
  assert.match(workspace, /request-delete-asset/);
  assert.match(runtime, /deleteMediaAsset/);
  assert.match(api, /invoke\("delete_media_asset"/);
  assert.match(nativeLib, /asset_commands::delete_media_asset/);
  assert.match(nativeDelete, /DELETE FROM assets/);
  assert.doesNotMatch(nativeDelete, /remove_project_file\(&project_path, &asset\.original_path\)/);
});

test("indexing avoids legacy-incompatible negative scale dimensions", async () => {
  const indexing = await read("src-tauri/src/indexing.rs");
  assert.doesNotMatch(indexing, /scale=[^"\n]*:-2/);
  assert.match(indexing, /scaled_dimensions/);
  assert.match(indexing, /scale=\{width\}:\{height\}/);
  assert.match(indexing, /Cache\/waveforms/);
  assert.match(indexing, /<svg/);
  assert.match(indexing, /compact_process_error/);
});
