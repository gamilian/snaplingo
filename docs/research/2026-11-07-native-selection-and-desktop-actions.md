# Native selection, recording and desktop actions

The actual macOS selection provider is constructed from the application identifier. Neither it nor its selection methods hold AppHandle. Synthetic copy dispatch uses the process main queue, preserving modifier-release waiting and the existing clipboard transaction.

Self-selection reads the focused AppKit window. A first-responder NSTextView supplies its UTF-16 selected range, including native field editors. Otherwise the reader finds a WKWebView in that window's content hierarchy and evaluates the same `window.getSelection()` script used by the previous Settings adapter. WebKit returns NSString directly, so quotes and newlines are not passed through an extra JSON decoder. The persisted `SelfWebview` method/source identity and method ordering remain unchanged. An inactive application or missing focused window fails the attempt and allows the existing fallback scheme to continue.

The recording state can be cloned without duplicating its session. The macOS global-shortcut filter owns that state and a Rust delivery callback. AppKit requires an active application and a visible key window whose native identifier matches the recording destination. Settings assigns its existing label as that identifier. Both key edges are consumed during recording; release publishes the same `recordingId`/`hotkey` payload through DesktopEvents and the retained frontend bridge. Focus loss cancels only the observed session and restores ordinary shortcut routing.

Application actions receive Rust services rather than AppHandle. Capture, selection translation, result windows and pin actions clone only the runtime needed by their asynchronous work. Settings/History/About, permission fallback and Quit publish window requests. The retained bridge still delivers those window routes and saves Settings geometry before exit. Menu callbacks and second-instance activation publish Rust action events; macOS dispatch and activation policy no longer use the Tauri event-loop API. The menu labels, ordering, accelerators and lazy Settings behavior from ADR 0006/0009 remain unchanged.

## Verification and limits

The native selection harness initializes NSApplication and real NSTextView/WKWebView controls, without a Tauri builder, AppHandle or Tauri event loop. It verifies UTF-16 selection of Chinese and emoji, empty selection, nested WebView traversal, quotes/newlines and rejection after the window is closed. Its control-level checks intentionally do not assert that a locked desktop grants focus.

Rust action tests construct actual services without a framework owner and verify Settings/History/About/Quit routing. Denied screen-recording and accessibility requests still route to the Permissions section, without performing OS resets or starting the blocked workflow. The application-state lifetime test now uses the actual native selected-text service. The native workflow verifies original notification payloads and recording payloads with real Tauri listeners, and tests Carbon hotkey/menu delivery to the native result window. Architecture guards cover the selection provider, recording core and application actions.

The strict `hotkey_recording_scope` probe initially found `com.apple.loginwindow` as the frontmost application: AppKit activation returned false and both native and Tauri focus remained false. Alternative activation calls, activation policies and an application-bundle launch did not change that result. Those temporary experiments were removed. The probe preserves its focus assertion and now catches failures/timeouts and exits, instead of leaving the event loop running after a worker panic.

Once desktop focus became available, the final production recording path passed the same probe:

```text
cargo run --quiet --locked --manifest-path src-tauri/Cargo.toml --example hotkey_recording_scope
PASS: native F18 recorded on release; its action was suppressed only while recording; cancellation and focus loss restore normal routing; Cmd+C remains unregistered
```

The recording result requires the real AppKit focus predicate to accept the window: a fake focused-window callback is not injected into this probe. The checkpoint passes 735 Rust unit tests (two existing ignored), ten architecture checks, the full native UI workflow and the independent selection/main-queue harnesses. Formatting, all-target compilation, dependency-notice validation and the retained frontend production build also pass.

This migration keeps the current Settings frontend; it does not yet replace Settings, its embedded result/library UI, the main Tauri event loop or packaging. Cargo still requires `tauri` and `tauri-build`. Windows/Linux window and recording adapters remain framework-backed. The full native-Rust migration goal remains active.
