# Native quick capture selection on macOS

This continuation follows checkpoint **ddc7d7e** and the [native standalone Result Window](2026-11-05-native-result-window.md). It moves the selection surface for `screenshot-copy`, `screenshot-ocr`, `silent-screenshot-ocr` and `screenshot-translate` to Rust/AppKit. OCR model and runtime resources are unchanged.

## Implementation

- Composition selects the macOS native capture host. Preparation receives the launch mode before capturing frozen pixels, so a quick capture never creates the annotation editor's WebView. A dormant editor is destroyed and its label is released before the native window opens. Normal screenshot editing and Windows/Linux keep their existing frontend.
- The native panel renders the frozen display with NSBitmapImageRep/NSImage. A separate cached desktop layer avoids repainting the background for pointer input. Only selection and magnifier changes invalidate their overlays; there is no recurring render timer.
- Selection state lives in Application and retains window priority, small/full-display candidate exclusions, six-point edge snapping, two-point minimum selection size, Shift square constraints, press/release candidate selection, WASD nudges and stale accessibility-result rejection. Captured drags can continue onto adjacent frozen displays. The visible macOS panel remains on the trigger display, matching the existing separate-display-Spaces restriction.
- Native input retains click/drag completion, Enter, copy/save/quick-save/pin/print shortcuts, full-display selection, F5 refresh, captured-cursor inclusion, history restore/browse, Alt magnification, RGB/HEX copying and format switching. Escape cancels; a right click clears a draft and a subsequent right click cancels. Save dialogs temporarily suspend the panel and resume it on cancellation. Native printing uses NSPrintOperation.
- Accessibility requests keep only the latest queued pointer and apply results to the matching session/revision. Old results cannot replace a new pointer or an active drag.
- Copy/save/pin completion holds the application startup lock, validates the current presentation, records successful selection history and releases the native panel/session. Repeated shortcuts and F5 release the old frozen images and presentation ownership. Closing detaches the content view and drops the image/layer graph.
- OCR uses the existing prepare/complete handoff and background generation checks. Its two-point context expansion is retained; the displayed OCR preview remains cropped to the selected area. The owned job continues after the selection panel closes and delivers to the native Result Window or clipboard without frontend IPC.
- History uses the app identifier's NSUserDefaults suite, retains 20 deduplicated rectangles and is shared with the annotation editor through application ports. The editor imports its existing localStorage history when it next opens; native quick captures subsequently share the same history. R and comma/period follow the current capture mode when completing a restored rectangle.
- Magnifier color copying reads the frozen bitmap's original RGBA channels. AppKit's calibrated NSColor conversion otherwise changes copied RGB values. Display snapshots carry the monitor ICC profile, so render tests check orientation separately from exact source-color sampling.

Sources: [selection state](../../src-tauri/src/application/capture/selection.rs), [native view](../../src-tauri/src/infrastructure/system/capture_window/native_view.rs), [host/actions](../../src-tauri/src/infrastructure/system/capture_window/native_host.rs), [runtime](../../src-tauri/src/application/capture/runtime.rs).

## Validation

- Rust: 683 unit tests pass, two existing tests are ignored; nine architecture tests and three infrastructure integration tests pass.
- All AppKit main-thread harnesses pass: native workflows, pinned rendering, scrolling controls/overlays and result dragging.
- Quick-capture workflows use synthetic frozen images/OCR and an isolated SQLite database/NSUserDefaults suite. Tests exercise actual native event dispatch, negative/Retina coordinates, selected-display decoding, square selection, source colors, cursor inclusion, output shortcuts, copy/save/pin, selection history, all OCR targets, element detection, refresh/replacement, cancellation and image/session cleanup. The native paths assert that no WebView exists.
- Native raster snapshots verify display orientation and scaling, selection/magnifier rendering and print-image rendering. Physical printer output is not exercised.
- The harness waits for the existing OCR delivery/startup exclusion before starting the next scenario. Reentry during this brief handoff retains the runtime's existing behavior.
- Frontend: all 1,013 tests pass, one is skipped; production build and dependency-notice checks pass.

## Quick-selection performance

The isolated release harness renders the same decoded 1280×720 frozen desktop and selection with both adapters. Readiness checks require a visible selection surface, decoded image dimensions and zero/one WebViews respectively. Active OCR, Settings, pinned windows and the Result Window are excluded.

Measurements use the same Apple M1 Pro, 16 GiB host as the prior benchmarks. Values are medians of three runs' means, after four seconds of warmup and eight samples at 750 ms intervals. Physical footprint sums the main process and all attributed helpers. CPU uses macOS rusage counters converted with mach_timebase_info; 100% represents one CPU core.

| Idle visible quick selection | Native | React/WebView |
| --- | ---: | ---: |
| Physical footprint including helpers | 50.8 MiB | 140.8 MiB |
| CPU | 0.018% | 0.235% |
| Helper processes | 0 | 3 |

The native surface uses about 64% less memory and 92% less idle CPU in this benchmark. These measurements cover a frozen quick-selection surface with synthetic pixels, rather than active pointer painting, screen acquisition or OCR inference. Native physical footprint varied between roughly 50.7 and 62.2 MiB across runs; the table uses the stated median. Raw per-run samples and process attribution are saved in [native-quick-capture-metrics.json](2026-11-05-native-quick-capture-metrics.json).

The optimized-build quick-capture and complete native workflow harnesses also pass. Synthetic native keyboard events are sent to the current responder independently of desktop keyboard ownership, matching the standalone native pin harness's background-process policy. Result-window history checks wait for asynchronous database commit after the visible response completes.

## Bundle

The macOS app and DMG build and pass the repository's packaging/signature verification. This is the existing locally signed, unnotarized beta workflow. No installed app is replaced.

Sizes sum regular file bytes and exclude `Contents/Resources/ocr/`:

| macOS arm64 0.1.23 build | Executable | Bundle excluding OCR |
| --- | ---: | ---: |
| Native standalone Result Window | 42,777,008 B | 43,412,359 B |
| Native quick capture selection | 42,960,576 B | 43,595,927 B |

This stage adds 183,568 bytes (about 0.18 MiB, 0.42%) to the non-OCR bundle, which totals about 41.6 MiB. The annotation editor and Settings still need the shared frontend, so this stage does not reduce shipped asset size. Native UI migration reduces the active renderer cost; eliminating the remaining frontend is separate work.

## Reproduction

    cargo test --locked --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test native_ui_workflows -- --surface capture
    cargo test --locked --release --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test native_ui_workflows --no-run
    python3 script/profile-native-ui.py /absolute/path/to/native_ui_workflows-executable docs/research/2026-11-05-native-quick-capture-metrics.json --surface capture

Optional raster output uses `SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR=/tmp/snaplingo-quick-native-snapshots` with the workflow command.

## Remaining scope

The [ordinary screenshot entry continuation](2026-11-05-native-screenshot-entry.md) also moves initial ordinary selection to AppKit and loads the existing annotation editor only after confirmation. The annotation editor, Settings and its embedded result overlay still require the shared frontend. Windows/Linux native UI ports remain separate work. This stage removes WebView creation from the four macOS quick-capture workflows; the whole application still supports WebViews for these remaining surfaces.
