import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig(async ({ mode }) => ({
  plugins: mode === "test" ? [] : [react()],
  esbuild: mode === "test" ? undefined : {
    logOverride: { "unsupported-regexp": "error" },
  },
  build: {
    // macOS 12 ships Safari 15; Windows uses the installed WebView2 runtime.
    target: ["safari15", "chrome105"],
    cssTarget: ["safari15", "chrome105"],
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
