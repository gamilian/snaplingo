# SnapLingo Runtime Map

## Frontend Runtime

`src/App.tsx` selects a lazy composition entry from `src/composition/` for the native window label and launch parameters. Capture retains its separate entry in `src/CaptureApp.tsx`. Each entry supplies an Application runtime to its View:

```text
SettingsWindow       -> application/settings
CaptureWorkspace     -> application/capture-workspace
ResultWindow         -> application/result-window
PinnedImageWindow    -> application/pinned-image
Scrolling Capture    -> application/scrolling-capture
Scrolling Preview    -> application/scrolling-capture (sampling disabled)
Scrolling Outline    -> viewport rendering only
Permission Gate      -> application/permissions
```

The Settings runtime includes the Library workflow. It combines History, Favorites, and Screenshot Favorites through narrow ports while Views retain only rendering and local interaction state. A backend Library Index returns the final page's ordered stable IDs before the frontend hydrates only those records. History pages include favorite membership for their visible content identities, without loading all Favorite records or thumbnails.

Pinned and Scrolling entries initialize only their own workflow and durable Settings projection. Their Settings subscription is installed before initial hydration and disposed even when subscription registration finishes late. They do not construct the Settings/Library, Result, or Provider runtimes. Settings and standalone Result share a Result runtime factory in Composition.

Settings mutations send field patches to the backend. The backend merges patches with the current snapshot while holding its update lock, including start-on-boot effects and window geometry updates, so stale frontend snapshots cannot overwrite independent changes.

Result Window owns its translation state, session validity, Provider retries, and automatic translation timers. Its Zustand Store subscribes to runtime snapshots and projects them with Settings for the View. Capture Session loading/output live in `application/capture-workspace/runtime.ts`, while shared edit transitions live in its `editing.ts` module; Views supply React, Canvas, DOM decoding/painting, input conversion, and printing.

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

Provider registries hold immutable `Arc` snapshots. Requests release registry locks before executing; credential changes prepare replacements and publish them only after persistence succeeds. Custom Provider definitions, credentials, and affected active/inactive ordering are committed in a single SQLite transaction.

History is subscribed synchronously during Composition, before any Provider request can publish. Quit stops admission, cancels provider futures and Capture OCR, waits for spawned requests and admitted History writes, then drains the EventBus with its existing five-second bound before exiting. Local PaddleOCR runs through an async child process: cancellation terminates it, execution times out after 60 seconds, and at most one running request plus one waiter is admitted.

Composition connects `SelectionTranslation` to Settings, Selected Text, and Result Window so Hotkey/IPC entry points share sequencing. `CaptureOcrDelivery` maps OCR targets and rejects stale handoffs with the injected Result Window runtime. File OCR reads through an asynchronous filesystem adapter and shares the original image bytes with its preview.

`app_actions.rs` maps shell actions to workflow calls. `startup_shortcuts.rs` maps validated hotkey category/action pairs to `AppAction`; `infrastructure/system/shortcut.rs` implements global shortcut registration. `application/hotkeys` owns parsing, registration state, and pressed/released policy.

## Native Targets

The CI workflow runs frontend tests/build plus backend formatting, tests, and checks on native macOS, Ubuntu, and Windows runners. Platform dependencies are installed on the runner that executes the target; no workflow treats that as cross-compilation verification.
