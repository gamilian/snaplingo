# Native annotation toolbar on macOS

This continuation follows the [native ordinary screenshot entry](2026-11-05-native-screenshot-entry.md). Tool selection and capture commands now use a Rust/AppKit toolbar. The annotation canvas and style controls still use the existing frontend. OCR models and runtime resources are unchanged.

## Implementation

- A borderless, nonactivating AppKit child panel renders the buttons. It cannot become a key or main window, preserving the editor's keyboard and text-input focus. Tool icons use system symbols with text fallbacks.
- Native controls include selection/move and all 12 annotation tools, plus undo, redo, clear, cancel, OCR, scrolling capture, favorites, pin, save and copy. Shift-clicking save retains quick save.
- The frontend sends a viewport-relative frame, interface scale, light/dark appearance, selected tool, history availability and output-busy state through the capture application port. AppKit converts the frame to desktop coordinates. There is no render timer.
- Button events carry the owning capture session ID and call the existing annotation/output actions. Stale events, duplicate output and callbacks after disposal are ignored. Presentation writes are serialized and superseded queued frames are skipped.
- Application validates geometry and current presentation ownership. If the native host is unavailable or fails, the complete frontend toolbar remains usable. Late acknowledgments after disposal are followed by cleanup.
- Color presets, stroke/fill, fonts, blur radius and corner-radius controls remain in the frontend. Opening its color popup temporarily hides the native panel so the popup remains visible and clickable.
- Hiding or destroying the editor also hides or releases the native toolbar. Destruction detaches the child before releasing the WebView. An old session cannot remove a newer toolbar.

Sources: [presentation](../../src-tauri/src/application/capture/editor_toolbar.rs), [AppKit controls](../../src-tauri/src/infrastructure/system/capture_window/editor_toolbar_macos.rs), [native host](../../src-tauri/src/infrastructure/system/capture_window/native_host.rs), [frontend bridge](../../src/views/CaptureWorkspace/useCaptureWorkspaceRuntimeView.ts), [action routing](../../src/application/capture-workspace/runtime.ts).

## Validation

- Rust: 692 unit tests pass, two existing tests are ignored; nine architecture and three infrastructure integration tests pass. All AppKit harnesses pass.
- The complete native workflow also passes with optimized release compilation, including result, quick capture, the annotation toolbar, editor refresh and resource cleanup.
- Frontend: 1,023 tests pass, one is skipped; TypeScript, production build and dependency-notice checks pass.
- Bridge tests cover tools, annotation history and output, stale clicks, duplicate output, disposal, failed/unavailable hosts and late acknowledgments. An existing toolbar is released before restoring frontend controls when an update becomes unavailable. Style controls remain available in native and fallback modes. Appearance follows the application's `data-theme` attribute.
- The real AppKit workflow exercises all 12 tool switches, focus policy, removal of frontend tool/output buttons, retained styles and color-popup visibility. A rectangle is drawn in the actual editor, followed by native undo, redo, clear and copy. Clear retains the existing behavior of discarding both annotations and undo history.
- Copy after clear produces byte-identical PNG output from the frozen crop. Completion and F5 release toolbar ownership. Ordinary selection still starts with zero WebViews; Enter copies without opening the editor.

Native raster output includes `capture-native-editor-toolbar.png`. Synthetic input uses the rendered selection's bounds and resolves the current surface between events.

## Bundle

The locally signed, unnotarized macOS arm64 0.1.23 app and DMG pass the repository's packaging/signature checks. The installed app is unchanged.

Sizes sum regular file bytes and exclude `Contents/Resources/ocr/`:

| Build | Executable | Bundle excluding OCR |
| --- | ---: | ---: |
| Native ordinary screenshot entry | 42,994,352 B | 43,629,703 B |
| Native annotation toolbar | 43,045,312 B | 43,680,663 B |

This stage adds 50,960 bytes (49.8 KiB, 0.12%) and brings the non-OCR bundle to 41.7 MiB. The combined migration and release optimization remain 25.6% smaller than the installed release's 58,714,119 bytes excluding OCR. The shared frontend is still shipped; this toolbar stage alone does not shrink it.

## Reproduction

    npm run build
    cargo test --locked --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test native_ui_workflows -- --surface editor

Optional snapshots use `SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR=/tmp/snaplingo-editor-toolbar-snapshots`.

An intermittent GitHub download failure was resolved with the existing MNN 3.6.1 archive. `MNN_PREBUILT_CACHE_DIR` points to that cache; the dependency performs its normal checksum verification. No dependency version, checksum or OCR resource was changed.

## Remaining scope and cost

At this checkpoint, the annotation canvas, style panel and Settings still require the frontend. The [native style continuation](2026-11-06-native-editor-styles.md) also moves the macOS style panel to AppKit. Tauri remains the application shell and temporary bridge to the remaining canvas, text input, embedded result overlay and Settings. Replacing the remaining UI and shell is required to finish the requested native Rust application. Windows/Linux retain their existing UI.

The previous [quick-selection benchmark](2026-11-05-native-quick-capture.md#quick-selection-performance) concerns a different surface. No new toolbar CPU/RAM comparison is claimed. Replacing these DOM buttons does not establish a reduction in the entire editor's footprint while its canvas still uses a WebView.
