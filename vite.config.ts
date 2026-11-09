import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig(async ({ mode }) => ({
  plugins: mode === "test" ? [] : [react()],
  esbuild: mode === "test" ? undefined : {
    logOverride: { "unsupported-regexp": "error" },
  },
  build: {
    rollupOptions: {
      input: {
        main: 'index.html',
        settingsReference: 'crates/snaplingo-app/tests/native_ui/settings_reference.html',
        resultReference: 'crates/snaplingo-app/tests/native_ui/result_reference.html',
        permissionReference: 'crates/snaplingo-app/tests/native_ui/permission_reference.html',
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
}));
