import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { formatBytes, formatDuration } from "../src/app/format.js";
import { defaultSettings } from "../src/app/state.js";

test("timecodes remain editor-friendly", () => {
  assert.equal(formatDuration(0), "00:00");
  assert.equal(formatDuration(65.9), "01:05");
  assert.equal(formatDuration(3661), "1:01:01");
});

test("media sizes use compact binary units", () => {
  assert.equal(formatBytes(0), "0 B");
  assert.equal(formatBytes(1024), "1 KB");
  assert.equal(formatBytes(1024 ** 3), "1.0 GB");
});

test("Slice 1 defaults to the agreed product settings", () => {
  assert.equal(defaultSettings.theme, "dark");
  assert.equal(defaultSettings.computeMode, "auto");
  assert.equal(defaultSettings.openrouterModel, "openrouter/free");
  assert.equal(defaultSettings.onboardingComplete, false);
});

test("the working editor avoids gradient styling", async () => {
  const styles = await Promise.all([
    "tokens.css",
    "base.css",
    "home.css",
    "workspace.css",
    "settings.css",
    "recovery.css",
  ].map((file) => readFile(new URL(`../src/styles/${file}`, import.meta.url), "utf8")));
  assert.doesNotMatch(styles.join("\n"), /linear-gradient|radial-gradient|repeating-linear-gradient/);
});

test("all visible workspace and settings actions are wired", async () => {
  const views = await Promise.all([
    "workspace.js",
    "overlays.js",
  ].map((file) => readFile(new URL(`../src/app/views/${file}`, import.meta.url), "utf8")));
  const renderer = await readFile(new URL("../src/app/render.js", import.meta.url), "utf8");
  const actions = [...views.join("\n").matchAll(/data-action="([^"]+)"/g)].map((match) => match[1]);
  for (const action of new Set(actions)) {
    assert.match(renderer, new RegExp(`action === "${action}"`), `Missing action handler for ${action}`);
  }
});

test("recovery commands are connected through the native boundary", async () => {
  const api = await readFile(new URL("../src/app/api.js", import.meta.url), "utf8");
  const native = await readFile(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");
  for (const command of ["reset_settings", "reset_app_data", "reset_cache", "repair_app", "reset_all"]) {
    assert.match(api, new RegExp(`invoke\\("${command}"\\)`));
    assert.match(native, new RegExp(command));
  }
});
