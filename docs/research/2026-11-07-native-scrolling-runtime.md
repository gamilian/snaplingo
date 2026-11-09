# Native scrolling runtime and UI fidelity

The macOS scrolling host and sampler now use injected Rust application services and the AppKit dispatch queue. They no longer import Tauri, resolve state through `AppHandle`, schedule work through its event loop, or use Tauri to create save dialogs. The capture runtime is injected weakly; shell notifier and compatibility callback ownership still need migration, so this does not establish that all framework/state ownership cycles have been removed.

The shared global-hotkey core owns registration, rollback, press/release delivery, main-thread destruction and shutdown independently of the shell. Composition still supplies the Settings hotkey-recording callback. The native scrolling host also receives a completion callback for the retained frontend capture surface; that compatibility event stays at the composition boundary. These callbacks and the remaining application shell mean the full application is not yet Tauri-free.

Windows and Linux keep the existing scrolling WebViews. The shared placement code preserves the frozen display geometry, negative-coordinate positioning, preview availability and overlap checks.

## UI

The previous native controls used AppKit's gray push buttons, different typography and spacing, and no primary Complete button. The revised panel follows the existing `ScrollingCaptureWindow` component's surface and border, rounded corners, title/dimension alignment, font sizes and weights, button spacing, Complete emphasis, preview label expansion, hover colors, tooltips and error retry styling. Dragging is restricted to the header. Unchanged sampling state does not invalidate the panel background.

`native_theme.rs` embeds and reads the same semantic palette blocks in `src/styles/index.css` and `src/styles/themes.css`. The capture editor toolbar uses this palette too, replacing its duplicated color table. Reading these static tokens does not create a WebView or require an npm build. All three presets support light/dark appearance; Settings updates refresh an already open scrolling panel.

Native raster checks verify the actual panel surface and primary button colors for all six preset/appearance combinations. Samples account for Retina backing scale, and PNG snapshots convert the display profile to sRGB; an 8-bit color-space round trip permits one level of channel rounding. They do not establish full visual parity for every surface, font fallback, accessibility behavior or platform. Browser comparison of the rendered React component was not completed: the browser host was unavailable, then automatic approval rejected controlling Microsoft Edge without a further reason. The component source and native raster snapshots remain the evidence for this checkpoint.

## Verification

The final macOS checkpoint passed 727 Rust unit tests (two existing tests ignored), ten architecture checks, the complete native workflow, scrolling controls/overlays, native pinned rendering and the framework-independent UI/shortcut queue harness. Formatting and all-target compilation checks pass. Tests that need macOS services and local mock-server sockets were run outside the sandbox after its permission restrictions prevented the same checks inside it.

`hotkey_recording_scope` passed with real Carbon events sent to its own process: F18 was recorded on release, its action was suppressed only during recording, focus loss restored normal routing and Cmd+C stayed unregistered.

The native UI queue test initializes NSApplication and the production shortcut core without a Tauri builder, handle, WebView or event loop. It verifies worker-thread registration, duplicate rollback, callback release, worker-thread shutdown, manager reinitialization, queued work ordering and cancellation. The architecture gate checks the native scrolling, theme, queue and shortcut sources for Tauri dependencies.

The complete native workflow exercises sampling, undo, preview visibility, save output, sampler stop, pin/result/capture/editor actions and resource cleanup with an isolated database and synthetic capture/clipboard ports. Independent controls and overlay harnesses check action dispatch, stale updates, status recovery, all six palette rasters and repeated closure.

Remaining work is the native capture host's lifecycle bridge, Settings and its embedded result UI, application startup/state/event ownership, packaging, parity checks for the other native surfaces, and native Windows/Linux implementations. No whole-application memory, CPU or size improvement is claimed by this checkpoint.

The capture host and lifecycle bridge are subsequently migrated in [native capture runtime and lifecycle](2026-11-07-native-capture-runtime.md), including editor restoration after a scrolling-window opening failure. The other acceptance and shell work remains.
