# SnapLingo Runtime Map

## Frontend Runtime

`src/` is the React/Vite frontend. Window modules render Settings Window, Capture Window, Result Window, and Pinned Image Window.

Design prototypes live under `designs/` so the production frontend tree stays focused on runtime code.

## Backend Runtime

`src-tauri/` is the Tauri/Rust backend runtime. `src-tauri/src/commands/` is the frontend-facing adapter seam. `application/` owns workflow modules, `domain/` owns shared domain types, and `infrastructure/` owns OS, storage, HTTP, window, and event adapters.

## Frontend/Backend Seam

Frontend code should call backend behavior through `src/tauri/*` adapters. Those adapters call Tauri commands declared under `src-tauri/src/commands/`.

During the migration, some existing runtime files still call Tauri commands directly. New frontend command calls should go through the adapter seam.

## Deep Modules

- Provider Coordinators: provider activation, persistence, and execution.
- Capture Session: frozen desktop, selection rendering, output, and OCR handoff.
- Pinned Image: in-memory pinned image state and window adapter behavior.
