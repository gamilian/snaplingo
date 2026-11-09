# Native Settings first-use permission guide

The standalone AppKit Settings migration had omitted the first-use guide.
The retained React `RequiredPermissionsGate` wrapped only the old Settings
application branch, while `NativeSettingsWindow` mounted the seven shared pages
directly. The existing General permissions/recovery controls were already wired
to the Rust controller; the missing behavior was the guide and its lifetime.

An isolated `native_app_smoke --permissions` scenario reproduced the omission:
missing Screen Recording opened native Settings without an attached guide. It
now passes against the actual AppKit window with a fake permission host and
temporary database. It checks initial presentation without a request, duplicate
request suppression, error/retry, return refresh, optional Accessibility,
temporary-installation blocking, recovery navigation and dismissal across
Settings hide/show. It never requests or resets the user's TCC permissions.

`PermissionPromptSession` owns that Settings lifetime's dismissal, busy state,
request generation and permission projection. Reading status never requests a
grant. Only the explicit Authorize action requests Screen Recording, rechecking
the current grant and installation context first. Late work after hide/disposal
does not update the new presentation. The native sheet refreshes on activation
and every 750 ms while visible, and closes once Screen Recording is granted.
Accessibility stays optional. Recovery navigates to the existing General
permissions section, retaining its explicit refresh, repair confirmation and
restart controls.

The guide uses shared Rust drawing in a borderless AppKit sheet and an AccessKit
modal tree. Buttons, focus and keyboard actions use the same typed model; Tab
cycles inside the guide and Escape/Later restore the previous responder. The
temporary-copy state only offers acknowledgement and installation instructions.
There is no macOS permission guide on Windows.

The independent WebKit reference mounts the actual retained component using an
isolated permission port. Its 24 recorded scenes cover all three interface
themes, light/dark appearance, initial/attempted/error/installation states.
`permission_prompt_layout` checks all cards and 54 action rectangles within
0.5 DIP. `permission_prompt_snapshot` produces corresponding review PNGs under
`target/shared-permissions`. These checks establish layout and native lifecycle,
not VoiceOver acceptance or identical system font antialiasing.

The native UI workflow harness also now owns AppKit directly, using an explicit
`NativeTestApp` with `Arc<AppState>` and the main dispatch queue. It retains real
capture/editor/result/pin/scrolling/Carbon assertions with synthetic capture and
clipboard ports. WebView-only comparisons and retired WK Settings mounting
tests were removed; the standalone application smoke covers the seven native
routes, capacity and permission sheets. The profiler targets only native
pin/result/capture fixtures and no longer needs Vite or a Tauri event loop.
