# SnapLingo Runtime Map

## Frontend Runtime

Each window starts in `src/views/`. Its view-local runtime context supplies one of the frontend Application runtimes:

```text
SettingsWindow       -> application/settings
CaptureWorkspace     -> application/capture-workspace
ResultWindow         -> application/result-window
PinnedImageWindow    -> application/pinned-image
Permission Gate      -> application/permissions
```

The Settings runtime includes the Library workflow. It combines History, Favorites, and Screenshot Favorites through narrow ports while Views retain only rendering and local interaction state. A backend Library Index returns the final page's ordered source references before the frontend hydrates those records.

Result Window owns its translation state, session validity, Provider retries, and automatic translation timers. Its Zustand Store subscribes to runtime snapshots and projects them with Settings for the View. Capture Workspace's runtime and editing policy live in `application/capture-workspace/runtime.ts`; Views supply React, Canvas, DOM decoding/painting, input conversion, and printing.

The runtime receives typed adapters from `src/platform/tauri/`. Those adapters own Tauri command names, event payload parsing, subscriptions, and Tauri-window effects. Views and frontend Application modules do not import Tauri packages or Platform modules.

## Backend Runtime

`src-tauri/src/lib.rs` is the Tauri startup shell. It builds `AppState` through `composition.rs`, registers commands, and coordinates shell lifecycle concerns.

```text
Tauri command -> Application runtime <- Infrastructure adapter
                        ^
                  Composition wiring
```

`application/` owns Capture, Providers, History, Result Window, Pinned Image, Settings, Hotkeys, and Selected Text workflows. Each module declares the port it needs. `infrastructure/` owns the implementations for storage, credentials, HTTP, LLM transport, events, database, clipboard, windows, shortcuts, screenshots, selection, and native OCR.

Capture's runtime serializes startup across all entry points: overlapping window-open intents are ignored, while direct session creation returns a busy error. Startup completion or failure permits retry. Each session keeps its frozen monitor geometry and coordinate convention; preparation and reveal name the session so refreshes cannot reuse an older session's scale. Pin output passes the rendered PNG through `CapturePinOutput` to the existing Pinned Image runtime, which stores the image before opening its window.

Required Permissions owns polling and the ordered explicit request workflow. TTS owns text normalization, locale-based voice selection, and persisted voice/rate policy; Composition selects the native speech adapter. Provider credentials are implemented directly by `SqliteCredentialStore`, including atomic multi-field writes.

`app_actions.rs` maps shell actions to workflow calls. `startup_shortcuts.rs` maps validated hotkey category/action pairs to `AppAction`; `infrastructure/system/shortcut.rs` implements global shortcut registration. `application/hotkeys` owns parsing, registration state, and pressed/released policy.

## Native Targets

The CI workflow runs frontend tests/build plus backend formatting, tests, and checks on native macOS, Ubuntu, and Windows runners. Platform dependencies are installed on the runner that executes the target; no workflow treats that as cross-compilation verification.
