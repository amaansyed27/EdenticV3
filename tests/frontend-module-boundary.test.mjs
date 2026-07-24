import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("workspace transient rendering is imported from its actual owner", async () => {
  const [main, workspaceRuntime, workspaceView] = await Promise.all([
    readFile(new URL("../src/main.js", import.meta.url), "utf8"),
    readFile(new URL("../src/app/workspace-runtime.js", import.meta.url), "utf8"),
    readFile(new URL("../src/app/views/workspace.js", import.meta.url), "utf8"),
  ]);

  assert.match(
    main,
    /import\s*\{\s*applyWorkspaceTransientPatch\s*\}\s*from\s*"\.\/app\/views\/workspace\.js"/,
  );
  assert.doesNotMatch(
    main,
    /import\s*\{[^}]*applyWorkspaceTransientPatch[^}]*\}\s*from\s*"\.\/app\/workspace-runtime\.js"/,
  );
  assert.doesNotMatch(workspaceRuntime, /export function applyWorkspaceTransientPatch/);
  assert.match(workspaceView, /export function applyWorkspaceTransientPatch/);
});
