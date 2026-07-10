# SnapLingo Runtime Map

## Frontend Runtime

`src/` is the React/Vite frontend. Window modules render Settings Window, Capture Window, Result Window, and Pinned Image Window.

Design prototypes live under `designs/` so the production frontend tree stays focused on runtime code.

## Backend Runtime

`src-tauri/` is the Tauri/Rust backend runtime. `src-tauri/src/lib.rs` is the Tauri startup shell, `src-tauri/src/app_state.rs` owns the AppState shape, and `src-tauri/src/commands/` is the frontend-facing adapter seam. `application/` owns domain-oriented workflow modules (`capture`, `pinned_image`, `history`, `selected_text`, `hotkeys`, `providers`, and `settings`), `domain/` owns shared domain types, and `infrastructure/` owns OS, storage, HTTP, window, and event adapters.

The app shell is menu-bar resident: `app_shell` owns tray/menu setup and menu ID adaptation/lifecycle policy helpers. `app_actions` owns shared menu/Hotkey AppAction dispatch. `settings_window` owns the Settings Window lifecycle. Business windows are lazy-created by their owning modules: `system/capture_window`, `system/result_window`, and `system/pinned_window`.

## Frontend/Backend Seam

Frontend code calls backend behavior through `src/tauri/*` adapters. Those adapters own command names and payload mapping, then call Tauri commands declared under `src-tauri/src/commands/`.

`src-tauri/src/composition.rs` owns runtime dependency construction: Provider registration, Coordinator construction, and startup event subscriptions. `src-tauri/src/application/hotkeys/runtime.rs` owns startup/global shortcut registration and update lifecycle. `src-tauri/src/startup_shortcuts.rs` owns Hotkey category/action binding, display parser, and pressed/released timing. Custom Translation Provider creation and runtime add/register/activate behavior live in the Provider Configuration Module.

Capture Session owns the portable `CaptureSessionSource` port and Capture data. `infrastructure/system/screenshot` supplies the current macOS, Windows, or Linux adapter through Composition. Tesseract follows the same direction: the Provider owns language policy, while `infrastructure/system/ocr/tesseract.rs` implements executable discovery and native engine mechanics.

The frontend Capture Workspace is split by ownership. `useCaptureWorkspaceController` composes state, derived geometry, the selection overlay, and magnifier pixels. `useCaptureWorkspaceHostController` owns Tauri session actions, hydration/performance tracking, reveal, and subscriptions. `useCaptureWorkspaceEditorController` owns text and annotation transactions. `useCaptureWorkspaceInputController` assembles keyboard and pointer contexts. `ScreenshotSession/index.tsx` only wires those interfaces to host hooks and `CaptureWorkspaceView`.

## Deep Modules

- Provider Coordinators: provider activation, persistence, and execution.
- Provider Configuration Module: credential validation, custom Translation Provider definitions, runtime add/register/activate rollback, and Provider reconfiguration support.
- Capture Session Runtime: frozen desktop, selection rendering, output, and OCR handoff behind one Application interface; its source port is owned inward and implemented by platform adapters.
- Pinned Image Runtime: in-memory state transitions, image output, group workflows, and window effects behind one Application interface.
- Settings Navigation State: pure frontend model for resolving and guarding Settings secondary navigation.
- Capture Interaction Model: pure frontend model for capture completion flow decisions.
- Capture Workspace Controllers: separate host, editor, and input interfaces behind a small composition hook.
