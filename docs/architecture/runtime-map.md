# SnapLingo Runtime Map

## Native application entry

`crates/app/main.rs` and `native_main.rs` enter the same native
application. The default `native-app` feature supplies the complete macOS and
Windows hosts. `lib.rs` declares/exports those modules and the portable Core
interfaces; it does not build a Tauri application or register IPC commands.

```text
macOS native_app.rs / Windows native_windows.rs
    -> AppState assembled by composition.rs
    -> Application runtimes <- Infrastructure adapters
    -> native Settings / capture / result / pin / scrolling windows
```

`native_app.rs` owns AppKit startup and application delegation. `native_windows.rs`
owns the winit event loop and routes native window/input/accessibility events.
`ui_thread` marshals platform effects to those loops; `infrastructure/runtime.rs`
owns Tokio workers. `RuntimeLifecycle` owns startup work, hotkey registration and
quit draining. Closing Settings hides the window while the resident app remains.
Single-instance activation and tray/menu actions enter the same application.

## Shared Settings and native surfaces

All seven Settings pages use shared Rust layout, hit testing and typed intents.
macOS `native_settings_window.rs` mounts `SharedFeaturePage`; Windows
`native_settings_windows.rs` connects the desktop host to `SettingsWindowSession`.
The adapters under `composition/settings` bind real preferences,
provider and Library workflows, permission/update controllers and native sheets.
Asynchronous notifications wake the host and refresh snapshots. Cached capacity
totals use lightweight `LibraryIndex` queries and reject stale completions.

`snaplingo-ui` produces logical-DIP Scenes and actions; `snaplingo-render` produces
CPU pixels, and `snaplingo-accessibility` projects scene semantics. Platform hosts
own device DPI, IME, clipboard, menus/dialogs, accessibility and presentation.
Opaque Windows Settings uses the desktop presenter; alpha capture, result, pin
and scrolling surfaces use native layered presentation. AppKit supplies macOS
native windows and text controls. Interface zoom is separate from device DPI.

Capture serializes startup across entry points. Each session retains frozen
monitor pixels, geometry and its coordinate convention; selection, editing,
magnification and output consume that same snapshot. Windows capture uses the
primary desktop scale. Pin origins cross into Windows as physical coordinates;
logical sources are converted once, and center/cascade uses the virtual desktop
and actual physical window extent. Pin output stores the image through
`PinnedImageRuntime` before opening the native window.

Result Window owns request validity, Provider retries, automatic translation and
file-OCR/favorite sequencing. Scrolling Capture owns one sampler and revisioned
preview updates, with native controls, a pass-through preview and viewport outline.
Native window adapters call these portable runtimes directly.

## Application workflows

`crates/core/application` owns Capture, Providers, History, Result
Window, Pinned Image, Settings, Hotkeys and Selected Text policy. Each module
consumes local ports. `snaplingo-storage` supplies durable repositories and image
assets. `app/infrastructure` supplies OS/network/native-library
capabilities; `composition.rs` and its builders choose implementations.

Composition connects `SelectionTranslation` to Settings, Selected Text and Result
Window so hotkey and menu actions share sequencing. `CaptureOcrDelivery` rejects
stale handoffs through the injected Result Window runtime. File OCR shares the
original image bytes with its preview. Required Permissions owns grant checks and
explicit requests; TTS owns normalization, locale-based voice selection and saved
voice/rate policy. Provider credentials use atomic SQLite multi-field writes.

Library Index returns lightweight globally ordered IDs before Library loads only
the selected History/Favorite records. History pages project visible content's
favorite membership without loading complete Favorite bodies or thumbnails.

`app_actions.rs` maps native actions to workflow calls. `startup_shortcuts.rs`
maps validated hotkey category/action pairs to `AppAction`; native shortcut
registration stays in Infrastructure. Typed `DesktopEvents` refresh native
subscribers without forwarding browser IPC events.

## Browser reference harness

The retained React tree under `reference/web/` is independent reference/test code.
`reference/web/App.tsx` forwards browser requests to isolated Settings, Result and Permission
fixtures under `reference/web/fixtures`. Those fixtures mount original
components with test data and preserve collector query parameters. Domain,
Application, View and Store regression tests remain alongside the original code.
The production Tauri adapters and npm dependencies are deleted.

Use `npm run reference:dev` or `reference:build` for these references. Default
`dev`/`build` commands use the native application, and native packages contain no
browser assets. AppKit/React frame and raster collectors supply independent visual
baselines for shared UI tests.

## Build and acceptance evidence

CI/release matrices target macOS Apple Silicon, macOS Intel and Windows, with
native build, final artifact verification and installation smoke. Windows
compilation and PE linking have passed; actual Windows GUI/installer execution
requires the Windows runner and desktop acceptance. macOS runtime checks, pure
geometry/workflow tests, cross compilation, linking and manual IME/accessibility
acceptance describe separate verification boundaries. Portable crates retain
other platform capabilities; the complete Linux application host is unsupported.
