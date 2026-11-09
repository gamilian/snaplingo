# Native Rust system integration

The application no longer depends on Tauri plugins. The remaining `tauri` and `tauri-build` dependencies still supply the desktop window/event/IPC shell and package build; removing the plugins alone does not finish the native application migration.

## Replacements

| Capability | Rust implementation | Preserved behavior |
| --- | --- | --- |
| Logging | `infrastructure/logging.rs`, fern and SQLite repository | UTC record format, Debug ceiling, durable settings filtering, retention cleanup |
| External opening | `system/external_open.rs`, open/ShellExecute | HTTP(S), mail and phone links; downloaded installers use a separate file entry |
| Global shortcuts | `system/shortcut.rs`, global-hotkey | Key press/release timing, recorder routing, rollback on registration failure, scrolling keys and UI-thread cleanup |
| Login startup | `system/start_on_boot.rs`, auto-launch | Existing SnapLingo entry identity, macOS LaunchAgent, Windows Run entry, Linux AppImage target and settings rollback |
| Dialogs | `system/dialogs.rs`, rfd | File filters, save format order and suffixes, canceled dialogs, caller parent window and native error buttons |
| Single instance | `system/single_instance.rs`, OS file lock and authenticated loopback notification | Second launch exits before setup and activates Settings; pending notifications survive startup; crash releases ownership |
| Tray/menu | `app_shell.rs`, tray-icon/muda and AppKit activation | Original menu order/text/actions, template icon, configured accelerators, OCR title/tooltip and Accessory activation |
| Async work | `infrastructure/runtime.rs`, Tokio | Application-owned worker pool, cancellation, blocking tasks and timers; residual Tauri IPC shares this handle |

Plugin initialization, capabilities, Rust dependencies and the npm dialog plugin have been removed. Cargo.lock and package-lock.json were updated, and third-party notices regenerated. Direct tray-icon disables default features to preserve the previous dependency feature set.

## Verification

The integration checkpoint passed 727 Rust unit tests (two existing tests ignored), nine architecture tests and three infrastructure integration tests. The complete AppKit workflow passed; targeted capture-editor workflows repeatedly passed after toolbar and keybinding updates. The frontend has 1,029 passing tests and one skipped test, and its production build and dependency notices check pass.

`single_instance_scope` passes with real child processes, including secondary activation before setup and takeover after a primary exits without Rust cleanup. `hotkey_recording_scope` dispatches actual Carbon events to this test process, confirming recorded release delivery, action suppression only during recording, focus-loss recovery and no global Cmd+C registration. `capture_shortcut_scope` verifies capture editing keys stay unregistered through overlapping presentations.

The AppKit menu verification creates the native tray/menu, checks item labels and accelerator modifiers, triggers a real menu action, verifies the OCR status title/tooltip and releases UI resources. It is run by the native workflow harness.

## UI fidelity and remaining work

The user requires the native UI to match the previous UI. System integration replacements preserve existing titles, menu actions and native-dialog behavior. The capture toolbar now uses the original frontend SVG icon shapes and has restored grouped tools, command labels, primary Copy, properties panel and semantic theme colors. Its native color palette and every other native surface must still be compared against the actual prior frontend rendering. Passing functional tests does not establish visual parity.

Remaining work includes Settings and its embedded result surface, the Tauri window/event/state bridge, native UI-thread dispatch and lifecycle, packaging, and native Windows/Linux implementations with independent platform checks. The goal remains active.

Continued in [native scrolling runtime and UI fidelity](2026-11-07-native-scrolling-runtime.md): the macOS scrolling host, sampler, save dialog and shortcut core now use Rust services/AppKit dispatch directly; the scrolling panel follows the previous semantic palette and typography. Settings recording and legacy capture completion remain composition callbacks, and the full application still depends on Tauri.
