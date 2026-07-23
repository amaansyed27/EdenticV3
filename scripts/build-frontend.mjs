import { cpSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";

const root = process.cwd();
const output = join(root, "dist");
const brandingAssets = [
  "combined/png/transparent/edentic-combined-256w.png",
  "combined/png/transparent/edentic-combined-512w.png",
  "icon/png/transparent/edentic-icon-128x128.png",
  "icon/favicons/favicon-16x16.png",
  "icon/favicons/favicon-32x32.png",
  "icon/favicons/apple-touch-icon.png",
];

rmSync(output, { recursive: true, force: true });
mkdirSync(output, { recursive: true });
cpSync(join(root, "index.html"), join(output, "index.html"));
cpSync(join(root, "src"), join(output, "src"), { recursive: true });

for (const relativePath of brandingAssets) {
  const destination = join(output, "assets", relativePath);
  mkdirSync(dirname(destination), { recursive: true });
  cpSync(join(root, "assets", relativePath), destination);
}

process.stdout.write("Edentic frontend assembled in dist/\n");
