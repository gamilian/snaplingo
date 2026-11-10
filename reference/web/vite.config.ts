import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

const referenceFile = (path: string) => fileURLToPath(new URL(path, import.meta.url));

export default defineConfig({
  root: referenceFile("."),
  plugins: [react()],
  esbuild: {
    logOverride: { "unsupported-regexp": "error" },
  },
  build: {
    rollupOptions: {
      input: {
        main: referenceFile('index.html'),
        settingsReference: referenceFile('fixtures/settings_reference.html'),
        resultReference: referenceFile('fixtures/result_reference.html'),
        permissionReference: referenceFile('fixtures/permission_reference.html'),
      },
    },
    // Match the retained reference collectors on macOS 12 and Windows.
    target: ["safari15", "chrome105"],
    cssTarget: ["safari15", "chrome105"],
  },

  // This server is only a visual-reference and frontend-test harness.
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // Keep the HTML/React references watchable while Rust builds run.
      ignored: ["**/crates/**/*.rs", "**/target/**"],
    },
  },
});
