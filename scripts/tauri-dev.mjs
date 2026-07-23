import { spawn } from "node:child_process";

const server = spawn(process.execPath, ["scripts/dev-server.mjs"], {
  stdio: "inherit",
  env: process.env,
});

const cargo = spawn("cargo", ["run", "--manifest-path", "src-tauri/Cargo.toml"], {
  stdio: "inherit",
  shell: process.platform === "win32",
  env: process.env,
});

const shutdown = (code = 0) => {
  if (!server.killed) server.kill();
  if (!cargo.killed) cargo.kill();
  process.exit(code);
};

cargo.on("exit", (code) => shutdown(code ?? 0));
server.on("exit", (code) => {
  if (code && !cargo.killed) shutdown(code);
});
process.on("SIGINT", () => shutdown(130));
process.on("SIGTERM", () => shutdown(143));

