# SnapLingo Runtime Map

## Frontend Runtime

`src/` is the React/Vite frontend. Window modules render Settings Window, Capture Window, Result Window, and Pinned Image Window.

Design prototypes live under `designs/` so the production frontend tree stays focused on runtime code.

## Backend Runtime

`src-tauri/` is the Tauri/Rust backend runtime. `src-tauri/src/lib.rs` is the Tauri startup shell, `src-tauri/src/app_state.rs` owns the AppState shape, and `src-tauri/src/commands/` is the frontend-facing adapter seam. `application/` owns workflow modules, `domain/` owns shared domain types, and `infrastructure/` owns OS, storage, HTTP, window, and event adapters.

The app shell is menu-bar resident: `app_shell` owns tray/menu setup and explicit menu actions. `settings_window` owns the Settings Window lifecycle. Business windows are lazy-created by their owning modules, such as capture, result, and pinned-image window adapters.

## Frontend/Backend Seam

Frontend code calls backend behavior through `src/tauri/*` adapters. Those adapters own command names and payload mapping, then call Tauri commands declared under `src-tauri/src/commands/`.

`src-tauri/src/composition.rs` owns runtime dependency construction: Provider registration, Coordinator construction, and startup event subscriptions. `src-tauri/src/startup_shortcuts.rs` owns startup global shortcut registration. Custom Translation Provider creation and runtime add/register/activate behavior live in the Provider Configuration Module.

## Deep Modules

- Provider Coordinators: provider activation, persistence, and execution.
- Provider Configuration Module: credential validation, custom Translation Provider definitions, runtime add/register/activate rollback, and Provider reconfiguration support.
- Capture Session Runtime: frozen desktop, selection rendering, output, and OCR handoff behind one Application interface.
- Pinned Image: in-memory pinned image state and window adapter behavior.
- Settings Navigation State: pure frontend model for resolving and guarding Settings secondary navigation.
- Capture Interaction Model: pure frontend model for capture completion flow decisions.
