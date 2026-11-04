# Rust native UI continuation

## Reference

AirSnip source was checked at commit [5710d3d](https://github.com/bd-dxg/airsnip/tree/5710d3d5acd86ef66dce790f0c46601f2d00eada). Its [manifest](https://github.com/bd-dxg/airsnip/blob/5710d3d5acd86ef66dce790f0c46601f2d00eada/Cargo.toml) identifies a Windows-only Rust application using tiny-skia, Win32/GDI and DirectWrite, licensed GPL-3.0-only. SnapLingo uses the architectural approach; no AirSnip source was copied.

The resting [application loop](https://github.com/bd-dxg/airsnip/blob/5710d3d5acd86ef66dce790f0c46601f2d00eada/src/app/mod.rs) waits for messages. The startup update timer is one-shot. Pin zoom uses a temporary [120 ms refinement timer](https://github.com/bd-dxg/airsnip/blob/5710d3d5acd86ef66dce790f0c46601f2d00eada/src/pin/zoom.rs), and the [pin window](https://github.com/bd-dxg/airsnip/blob/5710d3d5acd86ef66dce790f0c46601f2d00eada/src/pin/window.rs) owns and frees its drawing buffers.

Active long screenshots still use a [90 ms capture timer](https://github.com/bd-dxg/airsnip/blob/5710d3d5acd86ef66dce790f0c46601f2d00eada/src/longshot/window.rs). The [event handler](https://github.com/bd-dxg/airsnip/blob/5710d3d5acd86ef66dce790f0c46601f2d00eada/src/longshot/events.rs) redraws only dirty content and kills the timer when the window is destroyed. Native rendering does not imply that an active capture requires no sampling.

AirSnip's published Windows Private Working Set figures cannot be compared directly with macOS physical footprint or with SnapLingo's larger feature set.

## Implemented on macOS

- Scrolling controls, outline and preview are AppKit windows. The control panel no longer creates a WebView. Rust owns the existing 250 ms sampling interval and the output/error/close lifecycle.
- Buttons retain undo, preview toggle, copy, save in PNG/JPEG/WebP, pin, complete, cancel and retry. Temporary Enter/Escape/Cmd+Z registration and mouse-pass-through preview remain.
- Pins use native image drawing, drag/keyboard movement, zoom, opacity, rotation, flips, grayscale/inversion, thumbnail mode, border, native context menu and toolbar, original-channel color inspection and magnifier.
- Native actions call existing Pinned Image and Settings workflows. Copy/save retain the existing displayed-size export behavior. Hidden pins keep compressed data, geometry and presentation for group/recovery operations while decoded/filter/inspection buffers are released.
- The release profile uses thin LTO, one codegen unit and symbol stripping, retaining Rust's default release speed optimization and panic behavior.

Sources: [controls](../../src-tauri/src/infrastructure/system/scrolling_controls_macos.rs), [sampling/lifecycle](../../src-tauri/src/infrastructure/system/scrolling_native_macos.rs), [pins](../../src-tauri/src/infrastructure/system/pinned_window/native_view.rs), [native actions](../../src-tauri/src/infrastructure/system/pinned_window/macos.rs), [release profile](../../Cargo.toml).

## Validation and bundle size

Frontend: 130 test files, 1,011 passing tests and one skipped test; production build passes. Backend: 657 unit tests plus 12 architecture/integration tests pass, along with AppKit main-thread harnesses for scrolling controls/overlays, pins, native workflow integration and result dragging.

The [native workflow harness](../../src-tauri/tests/native_ui_workflows.rs) uses the real Tauri event loop and adapters with an isolated SQLite database and synthetic capture/clipboard ports. It verifies sampling starts and appends a frame, undo/preview/output, stop after close, keyboard-sized copy, paste replacement, settings changes, hidden buffer release, recovery and groups without touching the installed app or system clipboard. Raster snapshots verify the actual native drawing and rotation/flip directions.

Measured on Apple M1 Pro, 16 GiB RAM. Sizes below sum regular file bytes in the app bundle and exclude OCR resource bytes; they are not RAM measurements.

| macOS arm64 0.1.23 build | Executable | Bundle excluding OCR |
| --- | ---: | ---: |
| Installed official release | 58,078,768 B | 58,714,119 B |
| Previous local build | 54,644,864 B | 55,280,215 B |
| Native UI continuation + release optimization | 42,475,344 B | 43,110,695 B |

The non-OCR bundle is 22.0% smaller than the previous local build and 26.6% smaller than the installed release. This combines the UI change and release optimization; it does not isolate rendering as the cause of executable savings. No OCR models or runtime resources were pruned. The app/DMG built and passed the repository's release/signature verification.

## Remaining scope

Capture Workspace/editor, Result Window and Settings still use WebViews. Windows and Linux retain the prior pin/scrolling frontend. Their native ports and real-platform UI acceptance remain future work. This continuation does not make SnapLingo a fully WebView-free application.

## Pin-window performance

The [profiling script](../../script/profile-native-ui.py) compares native and React pin windows in the same isolated release harness. Both render identical 800×600 images and use the same backend and temporary database. OCR, Capture Workspace, Settings and Result Window are not active. The React run must report a decoded 800-pixel-wide image and three toolbar buttons per window before its measurements are accepted.

The fixture is launched with its own process responsibility. Physical footprint includes the main process and every attributed helper. CPU time uses macOS rusage counters converted through mach_timebase_info; 100% represents one CPU core. Measurements start after four seconds of warmup and take eight samples at 750 ms intervals. Values below are medians of the three runs' means.

| Idle pin windows | Native footprint | React footprint including helpers | Native CPU | React CPU | Helpers: native / React |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 22.7 MiB | 89.6 MiB | 0.006% | 0.099% | 0 / 3 |
| 5 | 64.2 MiB | 300.3 MiB | 0.012% | 0.160% | 0 / 7 |

The five-pin scenario uses about 79% less memory in this benchmark. These results establish a benefit for the migrated pin surface, not the whole application's idle footprint or active screenshot/OCR workloads. Raw samples and process attribution are saved in [native-ui-metrics.json](2026-11-04-native-ui-metrics.json).

Optimized-build native scrolling controls, pinned rendering and the complete native workflow harness also passed. Build the profiling harness with production frontend embedding:

```sh
cargo test --locked --release --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test native_ui_workflows --no-run
python3 script/profile-native-ui.py /absolute/path/to/native_ui_workflows-executable docs/research/native-ui-metrics.json
```

The initial benchmark without custom-protocol had an unloaded React window and was rejected by the readiness check. Its incomplete sample was replaced by the accepted three-run results.
