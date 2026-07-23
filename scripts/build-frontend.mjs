import { cpSync, mkdirSync, rmSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const output = join(root, "dist");
rmSync(output, { recursive: true, force: true });
mkdirSync(output, { recursive: true });
cpSync(join(root, "index.html"), join(output, "index.html"));
cpSync(join(root, "src"), join(output, "src"), { recursive: true });
process.stdout.write("Edentic frontend assembled in dist/\n");

