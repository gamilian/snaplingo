# Native ordinary screenshot entry on macOS

This continuation follows the [native quick-capture selection](2026-11-05-native-quick-capture.md). The initial `screenshot` selection now uses the same Rust/AppKit renderer. The existing annotation editor is created after a selection is confirmed. OCR model and runtime resources are unchanged.

## Behavior and ownership

- Ordinary screenshot startup opens a native selection panel without a WebView. Dragging a rectangle, clicking a candidate, restoring selection history or selecting the display opens the annotation editor. Enter copies the hovered selection directly; copy/save/quick-save/pin/print shortcuts retain the native output paths.
- The editor receives the existing session ID, logical rectangle and captured-cursor setting through an application command. It uses the same frozen pixels, enters its selected preview and restores the existing annotation-tool behavior before revealing. The transfer does not capture the desktop again.
- The native panel remains visible while the editor loads. The frontend's reveal acknowledgment transfers ownership; a dedicated host operation then closes the native panel and releases its decoded image/layer graph without canceling the shared capture session.
- Startup failure, an unrevealed editor closing or the eight-second handoff timeout discards the failed WebView and retains the native session for retry. Canceled or stale handoffs cannot reveal an editor. Escape can interrupt editor startup: cancellation rejects the acknowledgment before waiting for the startup lock.
- Editor output and cancellation release the frozen session and presentation, and destroy the editor when no capture presentation remains. The window destruction listener distinguishes an editor that owns the session from an editor whose handoff never completed.
- F5 inside the existing editor refreshes its frozen desktop through the legacy preparation path. It retains the same editor window while replacing the old capture session. Native startup preparation is reserved for a new capture launch; it would otherwise destroy the editor that is requesting the refresh.
- The normal editor's annotation tools, reselection, embedded result workflows and frontend remain available. Windows/Linux retain their existing capture UI.

Sources: [handoff coordinator](../../src-tauri/src/application/capture/editor_handoff.rs), [capture runtime](../../src-tauri/src/application/capture/runtime.rs), [native host](../../src-tauri/src/infrastructure/system/capture_window/native_host.rs), [native selection](../../src-tauri/src/infrastructure/system/capture_window/native_view.rs), [frontend adoption](../../src/application/capture-workspace/runtime.ts).

## Validation

- Rust: 689 unit tests pass, two existing tests are ignored; nine architecture tests and three infrastructure integration tests pass.
- Six handoff tests cover reveal acknowledgment, frozen-session ownership, failure/retry, cancellation during startup, stale sessions, unrevealed-window destruction and timeout recovery. Cancellation calls the public operation while startup holds its lock and must finish within one second.
- The real AppKit/Tauri workflow starts ordinary selection with zero WebViews, drags a rectangle, waits for the editor to reveal and checks that the native decoded images are released. Clicking the actual editor copy button produces byte-identical PNG output from the original frozen crop; the session and editor window are then released.
- A subsequent ordinary selection copies a candidate with Enter while retaining zero WebViews. Another editor handoff tests F5, verifies the replacement desktop has decoded 1280×720 pixels, waits for the host's decode/paint and actual AppKit reveal, checks that the editor's native window is unchanged and cancels it with Escape. DOM image readiness precedes native reveal and is checked separately.
- The complete native workflow passes in debug and optimized release builds, including this editor handoff after scrolling, pinned-image, result and quick-capture scenarios. Its host uses the application's implicit-exit policy so closing the last WebView cannot bypass the remaining assertions.
- Native pinned rendering, scrolling controls/overlays and result dragging harnesses pass. Native raster output includes the ordinary screenshot entry and its shortcut hint.
- Frontend: 1,016 tests pass, one is skipped; TypeScript, production build and dependency-notice checks pass.

## Rendering cost

Ordinary initial selection now uses the renderer measured in the [quick-selection benchmark](2026-11-05-native-quick-capture.md#quick-selection-performance), and does not initialize the annotation editor while the user is choosing a region. That benchmark measured 50.8 MiB versus 140.8 MiB physical footprint including helpers, and 0.018% versus 0.235% idle CPU, with synthetic frozen pixels. These are the previously measured quick-selection values, not new measurements of ordinary screenshot editing.

The annotation editor still loads a WebView when needed. This stage reduces renderer initialization before editing and releases the editor after capture completion; it does not establish the whole application's RAM/CPU usage or WebKit helper lifetime after editing. The shipped frontend remains necessary for the annotation editor and Settings.

## Bundle

The macOS arm64 0.1.23 app and DMG build and pass the repository's packaging/signature verification. They use the existing locally signed, unnotarized beta workflow. The installed app is unchanged.

Sizes sum regular file bytes and exclude `Contents/Resources/ocr/`:

| macOS arm64 0.1.23 build | Executable | Bundle excluding OCR |
| --- | ---: | ---: |
| Native quick capture selection | 42,960,576 B | 43,595,927 B |
| Native ordinary screenshot entry | 42,994,352 B | 43,629,703 B |

This handoff adds 33,776 bytes (about 33.0 KiB, 0.08%) to the non-OCR bundle, which remains about 41.6 MiB. The combined native migration and release-profile optimization produce a bundle 25.7% smaller than the installed release's 58,714,119 bytes excluding OCR. This stage alone does not reduce shipped frontend bytes: the annotation editor and Settings still need them. No OCR model or runtime resources were pruned.

## Reproduction

    cargo test --locked --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test native_ui_workflows -- --surface editor
    cargo test --locked --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --lib --test architecture_dependency_test --test infrastructure_integration_test --test native_ui_workflows
    cargo test --locked --release --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test native_ui_workflows --no-run

Optional raster output uses `SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR=/tmp/snaplingo-screenshot-native-snapshots` with the workflow command.

## Remaining scope

The [native toolbar continuation](2026-11-05-native-editor-toolbar.md) moves tool selection and capture commands to AppKit. The annotation canvas, style controls and in-editor reselection remain frontend-rendered, as do Settings and its embedded result overlay. Replacing those surfaces and the application shell is needed to remove the shared frontend and Tauri. Windows/Linux native UI ports and their real-platform acceptance remain separate work.
