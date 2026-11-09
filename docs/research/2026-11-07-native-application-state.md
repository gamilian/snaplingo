# Rust application state and desktop notifications

`build_app_state` now takes `CompositionOptions`: bundle identity, version, OCR/cache paths and an injected selected-text service. Constructing the macOS application services no longer requires AppHandle or a Tauri builder. The retained shell supplies these inputs through `composition/tauri_bridge.rs`; Windows/Linux also supply their existing window runtime adapters through the same boundary.

Settings, hotkey, provider, history, favorite and screenshot-favorite notifiers now publish typed Rust `DesktopEvent` values. `DesktopEvents` stores weak subscriptions, releases its registry lock before callbacks, supports nested delivery and stops subsequent callbacks when a subscription is dropped. Native result/pin preferences, scrolling themes and menu shortcuts subscribe with weak service references. Native pin preferences and scrolling completion no longer retain shell handles in their callbacks.

Window requests go through a live controller and return its error. This preserves Settings-opening and restart failure reporting; an ordinary notification observer cannot make a missing window controller appear available. Persistent changes still notify after their database write, and a failed UI notification does not roll that write back.

`NativeHotkeyRegistrar` receives a category/action callback and uses the framework-independent shortcut core. Its callback publishes a Rust event instead of resolving application state through AppHandle. The required-permission adapter likewise receives the bundle identifier and a restart callback; installation/translocation guards, selected-service reset arguments and existing permission UI messages remain.

## Compatibility and UI

`TauriDesktopBridge` owns the retained frontend subscription and forwards the original six event names with the same null payload. It also handles the remaining Settings, restart, hotkey-action and capture-completion requests. Its subscription is explicitly stopped during shell exit. These changes preserve the current UI renderer and its live data updates.

This checkpoint does not remove all Tauri dependencies or prove that every application ownership cycle is gone. The subsequent [selection and desktop-action migration](2026-11-07-native-selection-and-desktop-actions.md) removes AppHandle from the real macOS selected-text service and recording filter, and makes application actions call Rust runtimes directly. The independent construction/lifetime test now supplies that actual native selection service. Settings rendering, window-route delivery, application startup/event loop and packaging still retain Tauri.

## Verification

The checkpoint passes 733 Rust unit tests (two existing tests ignored), ten architecture checks, the full native workflow, independent pin/scrolling checks and the framework-independent UI/shortcut harness. Formatting and all-target compilation checks pass.

The state test constructs the actual Rust service graph without initializing Tauri, checks Settings notification delivery and verifies that the capture runtime is released when its state owner is dropped. Event tests cover worker delivery, receiver ownership release, reentrant delivery, self-unsubscription and window-controller failure propagation.

The native workflow attaches real Tauri listeners for all six compatibility events, verifies the original null payload and visibility of committed Settings, then stops a temporary bridge and confirms that only the remaining bridge delivers. Real Carbon events sent only to the test process route through the Rust registrar, event publisher and controller to the native result window exactly once on press. Capture/editor, OCR/translation, pin, scrolling failure recovery and resource cleanup workflows continue to pass.

Remaining work is native Settings and its embedded result/library UI, the retained window controller and application startup/event loop, packaging without Tauri, full visual acceptance and native Windows/Linux implementations. The original goal remains active.
