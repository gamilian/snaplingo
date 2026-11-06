# Native annotation styles on macOS

This continuation extends the [native annotation toolbar](2026-11-05-native-editor-toolbar.md). The complete macOS annotation toolbar, including its style panel, now renders through Rust/AppKit. The annotation canvas and text input still use the existing frontend. OCR models and runtime resources are unchanged.

## Implementation

- AppKit renders the color well, preset menu and add/replace/delete buttons, stroke-width slider, fill checkbox, font menu, bold/italic controls, font-size buttons, corner-radius slider and errors. The main slider switches between stroke width, font size and blur radius according to the active tool or selected annotation.
- The system color picker supports alpha and preserves sRGB byte values. Its window appears above the screenshot editor. The toolbar leases and restores the picker's previous level and appearance, deactivating and hiding it on editor hide or destruction while respecting another active color well.
- Native style events carry the capture session ID and a single property. The existing editor actions merge that property into the current style, preserving other fields and updating selected annotations or pending text. Native slider tracking previews changes continuously and commits the default stroke/font size after a completed interaction. Font-size buttons retain their existing behavior.
- Preset edits use the existing settings store. Mutation buttons remain disabled until a confirmed preset update or error arrives. Add selects the confirmed appended preset, replace preserves its selection, and delete clears selection. A persistence failure retains the editor and allows retry; a late failure from an old session cannot affect its replacement.
- The font list comes from the existing system font provider. Draft text quotes font-family names in the same way as the annotation canvas, including names such as `.Al Bayan PUA` that WebKit otherwise rejects as an unquoted CSS family.
- Before canceling a blurred preview, the runtime asks the host whether the current editor or its native picker owns focus. The host checks the session, active application and key window. A check completing after session replacement is ignored. Text blur preserves the draft while focus belongs to native controls; returning to the text input and pressing Enter still commits it.
- The frontend measures an invisible toolbar placeholder for placement. It reattaches its size observer when the native toolbar is adopted or released. Style rows adapt to narrow widths, interface scale and the current tool. Relevant style snapshots are memoized; there is no native render timer.
- A failed or unavailable native host restores the complete frontend toolbar. Windows and Linux keep that toolbar.

Sources: [presentation and validation](../../src-tauri/src/application/capture/editor_toolbar.rs), [AppKit styles](../../src-tauri/src/infrastructure/system/capture_window/editor_style_macos.rs), [native toolbar](../../src-tauri/src/infrastructure/system/capture_window/editor_toolbar_macos.rs), [action routing](../../src/application/capture-workspace/runtime.ts), [frontend bridge](../../src/views/CaptureWorkspace/useCaptureWorkspaceRuntimeView.ts).

## Validation

- Frontend: 1,028 tests pass, one is skipped; TypeScript, production build and dependency-notice checks pass.
- Rust: 693 unit tests pass, two existing tests are ignored; nine architecture and three infrastructure integration tests pass. The full AppKit workflow and the pinned, result-dragging, scrolling-controls and scrolling-overlay harnesses pass.
- The full native workflow also passes with optimized release compilation, covering result windows, quick capture, the annotation editor, scrolling, pins, settings persistence and resource cleanup.

Regression coverage includes selected shape styles, pending text and font defaults, preset persistence failure, stale callbacks, native focus ownership, host fallback and disposal. The real AppKit workflow exercises RGBA round trips, preset add/replace/delete, slider mouse tracking and settings persistence, fonts and bold/italic, blur and fill, text editing with the picker as key window, Enter commit and picker cleanup during output.

Existing drawing, undo, redo, clear, copy, frozen-pixel equality, ordinary screenshot handoff and F5 cleanup checks remain. Clear retains the existing behavior of discarding both annotations and undo history.

Snapshots include `capture-native-editor-toolbar.png`, `capture-native-editor-slider.png` and `capture-native-editor-text-styles.png`. Slider snapshots are taken before and after AppKit event dispatch, allowing its knob animation to finish; confirmed values, ranges and subsequent style updates are also checked directly. Optimized-build raster output was inspected as well. The fixture uses the production system font provider with an isolated database and synthetic screenshot/clipboard ports.

## Bundle

The locally signed, unnotarized macOS arm64 0.1.23 app and DMG pass the repository's packaging, signature and mounted-image checks. They are built from the current working tree based on `ddc7d7e`. The installed application is unchanged.

Sizes sum regular file bytes and exclude `Contents/Resources/ocr/`:

| Build | Executable | Bundle excluding OCR |
| --- | ---: | ---: |
| Native annotation toolbar | 43,045,312 B | 43,680,663 B |
| Native annotation styles | 43,128,096 B | 43,763,447 B |

This stage adds 82,784 bytes (80.8 KiB, 0.19%) and brings the non-OCR bundle to 41.74 MiB. The combined migration and release optimization remain 25.5% smaller than the installed release's 58,714,119 bytes excluding OCR. The shared frontend is still shipped; moving the style panel alone does not shrink it.

Local artifacts are `target/release/bundle/macos/SnapLingo.app` and `target/release/bundle/dmg/SnapLingo_0.1.23_aarch64.dmg`. The DMG includes the unchanged OCR resources and is not the size reported in the comparison above.

## Reproduction

    npm run build
    npm test
    cargo test --locked --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml
    cargo test --locked --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test native_ui_workflows -- --surface editor
    cargo test --locked --release --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test native_ui_workflows --no-run

Optional snapshots use `SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR=/tmp/snaplingo-native-editor-styles`.

Run the optimized executable printed by the final command without a surface filter to exercise the complete workflow. This build's executable is `target/release/deps/native_ui_workflows-c360399695c2878d`.

The existing MNN 3.6.1 archive is available through `MNN_PREBUILT_CACHE_DIR=/tmp/snaplingo-mnn-prebuilt-cache`; the dependency performs its normal checksum verification. No dependency version, checksum or OCR resource was changed.

## Remaining scope

The annotation canvas, in-editor selection and text rendering/input, embedded result overlay and Settings still require the frontend. Tauri remains the application shell and bridge to those surfaces. Replacing them and the shell is required to finish the requested native Rust application. Windows/Linux native ports remain future work.

The previous [quick-selection benchmark](2026-11-05-native-quick-capture.md#quick-selection-performance) measures a different surface. No new editor CPU/RAM comparison is claimed: migrating its style controls does not establish a reduction in the complete editor's footprint while its canvas still uses a WebView.
