# Native capture runtime and lifecycle

The macOS capture host now receives Rust services instead of an AppHandle. Selection, annotation rendering, font loading, styles, save/print, favorites, scrolling capture, OCR handoff and error recovery use those services and the AppKit main queue directly. The runtime and screenshot-favorite workflow are held weakly to avoid creating direct ownership cycles with their hosts. Settings notifiers still retain shell dependencies; this does not prove that all application ownership cycles are gone.

Composition passes the user-defaults suite identifier as a string, so constructing the macOS capture runtime no longer requires a Tauri handle. The selection history key, frozen session geometry, original image compositor, tool bindings, annotation controls and output settings remain unchanged.

## Lifecycle

`presentation_macos.rs` owns overlapping presentations, prior foreground application tracking, activation suppression and cursor cleanup with AppKit. Capture windows identify themselves as `capture`; the process's native window inventory determines whether a result, pin or settings window currently owns keyboard focus. Capture completion preserves that window's focus instead of activating an earlier application.

Hiding suspends the native view without discarding its frozen pixels or annotations. Completion/cancellation releases the view after presentation cleanup, and replacement targets only the old session. If scrolling-window creation fails, the retained editor is revealed again with its original selection and annotations. The resource check waits for both application-session removal and queued AppKit destruction.

The previous macOS WebView window adapter, window runtime and editor IPC toolbar are compiled only by the test/reference harness. Windows/Linux keep their previous adapters. The macOS startup no longer runs the capture WebView prewarming loop. The application still has a Tauri shell, Settings WebView and IPC boundary, including the retained capture-image protocol; the whole application has not been migrated.

## Verification

The native workflow passes selection, negative/Retina coordinates, capture keys, copy/save/pin, OCR/translation, control inspection, refresh, annotation tools, IME/text, styles, crop manipulation, undo/redo, frozen output and resource cleanup. It now also injects a scrolling-window opening failure into the real workflow and verifies that the native editor, annotations, session ownership and output pixels survive without a WebView. Independent pinned-window checks pass.

The UI-thread harness initializes NSApplication without a Tauri builder, handle, WebView or event loop. It checks nested capture presentations, unchanged activation policy at startup, final release, cancellation and shortcut lifecycle. `capture_shortcut_scope` passes with no global capture-editing key registration during idle, startup, overlapping presentations or completion. The architecture gate covers the native host, lifecycle, OCR effects and renderer as well as scrolling, and rejects direct Tauri APIs and the old local shell adapters.

This checkpoint passes 727 Rust unit tests (two existing tests ignored), ten architecture checks, the native workflow and framework-independent UI queue tests. All-target compilation and formatting checks pass. These checks establish this runtime migration, not complete visual parity of every window or a whole-application CPU/RAM/size reduction.

Remaining work includes Settings and its embedded result UI, application startup/state/notifier ownership, packaging without Tauri, visual acceptance of every migrated surface, and native Windows/Linux implementations with real-platform checks. The original goal remains active.

Continued in [Rust application state and desktop notifications](2026-11-07-native-application-state.md): the state builder accepts Rust inputs, all six durable-change notifiers use weakly subscribed native events, and the retained frontend forwarding has an independent, explicitly stopped subscription. Selected-text/window, Settings and shell work remains.
