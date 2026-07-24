import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import test from "node:test";

test("native command names are unique across Rust modules", async () => {
  const rustRoot = new URL("../src-tauri/src/", import.meta.url);
  const files = (await readdir(rustRoot)).filter((file) => file.endsWith(".rs"));
  const commands = new Map();

  for (const file of files) {
    const source = await readFile(new URL(file, rustRoot), "utf8");
    const matches = source.matchAll(
      /#\[tauri::command\]\s*pub\s+(?:async\s+)?fn\s+([a-zA-Z0-9_]+)/g,
    );
    for (const match of matches) {
      const command = match[1];
      const owners = commands.get(command) ?? [];
      owners.push(file);
      commands.set(command, owners);
    }
  }

  const duplicates = [...commands]
    .filter(([, owners]) => owners.length > 1)
    .map(([command, owners]) => `${command}: ${owners.join(", ")}`);

  assert.deepEqual(duplicates, [], `Duplicate Tauri commands:\n${duplicates.join("\n")}`);
});
