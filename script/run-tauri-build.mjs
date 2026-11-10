import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { createTauriBuildEnvironment } from "./macos-disk-image.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const tauriExecutable = resolve(
  root,
  "node_modules",
  ".bin",
  process.platform === "win32" ? "tauri.cmd" : "tauri",
);
const environment = createTauriBuildEnvironment(process.platform, process.env);

if (process.platform === "darwin") {
  console.log(
    "[tauri-build] Skipping redundant Finder DMG layout automation on macOS.",
  );
}

const result = spawnSync(tauriExecutable, ["build", ...process.argv.slice(2)], {
  cwd: root,
  env: environment,
  shell: process.platform === "win32",
  stdio: "inherit",
});

if (result.error) {
  throw result.error;
}

process.exitCode = result.status ?? 1;
