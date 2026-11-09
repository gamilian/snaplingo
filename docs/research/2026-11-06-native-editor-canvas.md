# Native capture annotation canvas

This continuation replaces the macOS annotation canvas and text draft input with Rust/AppKit, following the [native style controls](2026-11-06-native-editor-styles.md). It reduces the normal screenshot selection/edit/output workflow to zero WebViews. Tauri remains the application shell; the complete application has not yet been migrated.

## Implementation

- `CaptureEditor` owns annotation tools, drawing, selected annotation movement and resizing, arrow endpoints and curve controls, rounded corners, styles, text drafts, and bounded undo/redo. History shares immutable annotation buffers. Empty edits delete existing text and support undo.
- The original native screenshot window retains the frozen session when entering the editor. NSView renders the annotation canvas; NSTextView supplies text editing and marked-text handling. Native style controls retain the draft while the color picker owns focus.
- Both committed and transient annotations use the existing Rust image compositor. Preview requests are serialized and coalesced, carry a revision and excluded annotation identity, and reject stale responses. Failed rendering releases the request so a subsequent update can retry. PNG encoding is reserved for output instead of native preview delivery.
- Selection movement, eight resize directions, Shift constraints, and keyboard nudging preserve local annotation coordinates. Selected annotation handles take priority over the screenshot boundary.
- Copy, save, pin, OCR, favorites, print and scrolling actions call the existing Application workflows with the native annotation snapshot. Global pin requests also use this snapshot. Output errors restore the editor; nonterminal style errors release preset controls without changing output ownership. Suspension hides the native toolbar and its color picker with the capture window.
- The obsolete native-to-WebView editor handoff, transfer state, host methods, IPC command and frontend adoption branch are removed. The remaining frontend editor serves Windows/Linux and the macOS embedded result flow.
- The unused `tauri-plugin-screenshots` dependency, initialization and capability are removed. Native screenshot sources already use Core Graphics/xcap. Cargo.lock also drops the plugin's redundant older xcap/Windows dependencies, and the third-party notice inventory is regenerated.

Sources: [editor model](../../src-tauri/src/application/capture/editor.rs), [geometry](../../src-tauri/src/application/capture/editor_geometry.rs), [native canvas](../../src-tauri/src/infrastructure/system/capture_window/editor_canvas_macos.rs), [host](../../src-tauri/src/infrastructure/system/capture_window/native_host.rs), [native tool workflow](../../src-tauri/tests/native_ui/editor_tools.rs).

## Verification

On the final source state, 709 Rust unit tests pass (two existing tests remain ignored), nine architecture tests and three infrastructure integration tests pass, and the locked offline all-targets check passes. The frontend has 1,025 passing tests and one skipped test; TypeScript, the production build and dependency-notice consistency checks pass. Rust formatting and diff checks pass.

Earlier in this continuation, the complete AppKit workflow passed after the canvas replacement, all-tool drawing/output tests and output-error recovery were implemented. It verified zero editor WebViews, annotation/output PNG equality, actual eraser pixels, text/style input, rectangle resize, arrow curves, undo/redo, copy retry, refresh and resource cleanup, alongside result, scrolling and pin workflows.

The sandboxed AppKit rerun after handoff removal stalled during macOS initialization with `com.apple.hiservices-xpcservice` and LaunchServices connection errors. Its handles were explicitly interrupted. On 2026-11-07 the desktop rerun succeeded: all annotation tools, crop movement/resizing and Escape restoration, keyboard nudges, dialog suspension, global pin output, copy failure/retry and resource cleanup pass. Follow-up runs also cover the native tray/menu and the original editor keybindings. Visual snapshots are now available in `/tmp/snaplingo-native-ui-consistency/`. No new release package or CPU/RAM comparison is claimed.

Reproduce the current GUI gate with:

    SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR=/tmp/snaplingo-native-editor-canvas \
      cargo test --locked --offline --features tauri/custom-protocol \
      --manifest-path src-tauri/Cargo.toml --test native_ui_workflows -- --surface editor

Run the harness without `--surface editor` for result, capture, scrolling and pin coverage. The harness uses an isolated database, synthetic screenshot source and synthetic clipboard/output, preserving the installed application and system clipboard.

## Remaining migration

1. Complete the final AppKit and visual acceptance gate above, including normal desktop IME use and native editing interactions.
2. Replace the Settings frontend, including provider administration, permissions, Library/history/favorites and the embedded result surface.
3. Replace Tauri application lifecycle, AppHandle/state/event plumbing, main-thread dispatch and packaging. As of 2026-11-07, all Tauri plugins have been removed; the system integrations, tray/menu and async executor use Rust libraries directly. Tauri and tauri-build remain in Cargo.
4. Port the native surfaces to Windows/Linux and verify native packages and actual platform behavior independently.

The updated user goal also requires the UI to remain consistent with the previous application. The toolbar has restored grouped tools, command labels, original icon geometry, rounded panels, original theme colors and its separate properties panel. The native color palette and the other native surfaces still need a complete visual comparison against their frontend counterparts. The goal remains active until the application and build no longer require Tauri, retained desktop functions have been verified, and UI consistency is demonstrated.
