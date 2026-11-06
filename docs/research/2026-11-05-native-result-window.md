# Native standalone Result Window on macOS

The checkpoint commit is **ddc7d7e** (feat: render macOS scrolling capture and pinned images natively). This continuation moves the standalone translation/OCR result window from React/WebView to Rust/AppKit. OCR model and runtime resources were not pruned.

## Implementation

- The existing Result Window runtime still owns request IDs and transfers each current payload once. Its macOS host now creates an AppKit panel and delivers the payload directly to the Rust session. Windows and Linux keep their WebView adapter.
- The application session owns source/language edits, automatic translation policy, per-provider request validity, retries, result/history aggregation, OCR generations and favorite markers. Debounce remains 150 ms for incremental translation and 500 ms otherwise; immediate payloads bypass the automatic-editing preference.
- Native views use NSTextView, NSPopUpButton, NSImageView and system buttons. They retain source/card collapse, language selection and swap, translation/retry, selection/copy, speech, favorites, file OCR, OCR metadata, recommended copy fragments, clear, pin and close.
- IME operations finish before publishing edits. Marked text suppresses automatic translation and survives result updates. Escape used to cancel composition is consumed before window-close handling.
- Edits, payload replacement and close invalidate pending translation work. Deferred manual actions carry the current generation; stale retry, OCR, favorite and error completions cannot affect a newer input. History and auto-copy use the current provider responses, including retries.
- Layout measures native text, preserves the trailing insertion line, respects configured width/height limits, clamps placement to the chosen display and remembers drag position. Native source-collapse choices are stored per origin in NSUserDefaults.
- Window pinning and the 300 ms initial blur grace are retained. Dragging and file dialogs suppress blur dismissal. The normal floating-window level permits native menus and dialogs above the result.
- OCR image strings are shared through Arc; decoded image buffers are released on hide or when showing translation. There is no Rust render loop or recurring result-window polling.

Sources: [session](../../src-tauri/src/application/result_window/session.rs), [native view](../../src-tauri/src/infrastructure/system/result_window/native_view.rs), [native actions](../../src-tauri/src/infrastructure/system/result_window/macos.rs), [copy-fragment detector](../../src-tauri/src/infrastructure/system/result_window/tokens_macos.rs).

## Validation

- Rust: 669 passing unit tests, two ignored tests; nine architecture tests and three infrastructure integration tests pass.
- AppKit main-thread harnesses pass for result workflows, pinned image rendering/transforms, scrolling controls/overlays and result dragging.
- The result harness exercises actual controls and raster rendering, IME Escape/commit, trailing-newline sizing, narrow long text, light/dark OCR presentation, copy callbacks, favorites, speech and hidden-image release. Application actions use a temporary SQLite database, synthetic providers and a fake speech host. The installed app and system clipboard are untouched.
- Standalone pin tests assert desktop key-window state only when their process is active; background processes still exercise visibility, focus requests and input dispatch.
- Frontend: 1,011 tests pass, one is skipped; the production build and dependency-notice check pass.

The native and WebView result benchmark both verify the same source text, synthetic translation and one rendered provider card before sampling. The native path reports zero WebViews; the retained frontend baseline reports one. The release benchmark uses the same backend and excludes active OCR, Capture Workspace and Settings.

## Result-window performance

Measured on the same Apple M1 Pro, 16 GiB host as the prior pin benchmark. Both render the same source and completed provider response. Values are medians of three runs' means, after four seconds of warmup and eight samples at 750 ms intervals.

Physical footprint sums the main process and all attributed helpers. CPU uses macOS rusage counters converted with mach_timebase_info; 100% represents one CPU core.

| Idle visible result window | Native | React/WebView |
| --- | ---: | ---: |
| Physical footprint including helpers | 22.7 MiB | 80.2 MiB |
| CPU | 0.018% | 0.445% |
| Helper processes | 0 | 3 |

The native result surface uses about 72% less memory and 96% less idle CPU in this benchmark. These figures measure this surface with a synthetic provider; they do not measure the whole application's active screenshot/OCR workloads. Raw samples, per-run measurements and process attribution are in [native-result-metrics.json](2026-11-05-native-result-metrics.json).

The optimized native workflow harness also passes.

## Bundle

The app and DMG pass the repository's packaging/signature verification. This is the existing locally signed, unnotarized beta workflow; no installed app was replaced.

Sizes sum regular file bytes and exclude OCR resources:

| macOS arm64 0.1.23 build | Executable | Bundle excluding OCR |
| --- | ---: | ---: |
| Native pin/scrolling checkpoint | 42,475,344 B | 43,110,695 B |
| Native standalone Result Window | 42,777,008 B | 43,412,359 B |

This stage adds 301,664 bytes (about 0.29 MiB, 0.7%) to the non-OCR bundle. The shared frontend is still required by Settings and the other platforms, so this migration alone does not shrink the executable. The previous release-profile optimization remains in place.

## Reproduction

Build the harness with production frontend embedding:

    cargo test --locked --release --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test native_ui_workflows --no-run
    python3 script/profile-native-ui.py /absolute/path/to/native_ui_workflows-executable docs/research/2026-11-05-native-result-metrics.json --surface result

Optional native raster output:

    SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR=/tmp/snaplingo-result-native-snapshots cargo test --locked --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test native_ui_workflows

## Remaining scope

Capture Workspace/editor and Settings still use WebViews. Settings also retains its embedded React result overlay. Native result rendering currently applies to macOS; Windows/Linux native ports and their platform acceptance remain future work.
