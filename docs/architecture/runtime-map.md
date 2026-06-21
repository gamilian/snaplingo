# SnapLingo Runtime Map

## Frontend Runtime

`src/` is the React/Vite frontend. Window modules render Settings Window, Capture Window, Result Window, and Pinned Image Window.

Design prototypes live under `designs/` so the production frontend tree stays focused on runtime code.

## Backend Runtime

`src-tauri/` is the Tauri/Rust backend runtime. `src-tauri/src/commands/` is the frontend-facing adapter seam. `application/` owns workflow modules, `domain/` owns shared domain types, and `infrastructure/` owns OS, storage, HTTP, window, and event adapters.

## Frontend/Backend Seam

Frontend code calls backend behavior through `src/tauri/*` adapters. Those adapters own command names and payload mapping, then call Tauri commands declared under `src-tauri/src/commands/`.

`src-tauri/src/composition.rs` owns runtime dependency construction: Provider registration, Coordinator construction, custom Translation Provider restoration, and startup event subscriptions.

## Deep Modules

- Provider Coordinators: provider activation, persistence, and execution.
- Provider Configuration Module: credential validation, custom Translation Provider definitions, and runtime Provider reconfiguration support.
- Capture Session Runtime: frozen desktop, selection rendering, output, and OCR handoff behind one Application interface.
- Pinned Image: in-memory pinned image state and window adapter behavior.
