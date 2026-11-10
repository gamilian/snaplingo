# SnapLingo runtime map

Both binaries under `apps/desktop/src` enter the native application. The Cargo
package remains `snaplingo`; executable, installation and persisted-data identities
are unchanged by the module reorganization.

```text
shell/macos.rs or shell/windows.rs
  -> bootstrap constructs AppState, adapters and feature dependencies
  -> shell actions route menu/global-key/single-instance intent
  -> Core feature workflow -> Platform/Storage/integration contract
  -> desktop feature presenter/controller -> UI snapshot/intents
  -> native host consumes presentation and applies OS effects
```

AppKit owns the macOS loop; winit owns the Windows loop. Desktop integrations
marshal work to the correct loop and own worker execution. Shell lifecycle owns
startup, tray residency and shutdown. Closing Settings hides its window while the
resident application continues. Shutdown cancels/drains work before releasing
native owners.

Bootstrap is the only code that resolves the complete application into feature
`Services` and session factories. Native features do not import AppState or call
bootstrap. Notifications carry typed updates; subscriber lifetime belongs to the
consumer and does not create a browser event bridge.

`features/settings/presenter` owns one Settings session shared by both hosts:
navigation, edit/save ordering, refresh, stale-result rejection and disposal.
Bootstrap supplies real settings/provider/library workflows and native effects.
The Settings host consumes `ui/settings` plus the optional UI renderer and
accessibility projector. AppKit supplies its native text, dialog and accessibility
adapters; Windows uses the feature's winit host. Saved interface zoom and device
DPI are independent.

Capture startup is serialized by `core/capture`. A session freezes pixels,
monitor geometry and the coordinate convention together. Platform supplies screen
acquisition; Core owns selection, editing and output policy. Native Capture hosts
render that frozen state and send typed editing intent. Windows retains its
primary-desktop-scale policy. Both native hosts consume `ui::capture::presentation` for toolbar/palette
geometry and state, selection labels and magnifier sampling/placement. The Mac
adapter retains native controls and text composition; its actual frames and state
match the preserved AppKit baseline.

Result request validity, automatic translation and stale-result policy live in
Core. The desktop Result controller coordinates product effects; UI owns shared
presentation and native-control geometry. AppKit retains TextKit/IME/selection;
Windows supplies shared-renderer measurements and native text events.

Pinned Image owns portable transforms/input in UI, business state in Core, and
decoding/placement in the desktop feature. Mac image controls and Windows alpha
presentation consume shared state. Scrolling Capture owns one shared sampler and
cancellation-safe lifecycle, observing Core stitching/output through revisioned
previews. Its native controls, pass-through preview and viewport outline retain
explicit window owners.

Provider coordination, settings/hotkey durability, history/favorite policy,
selected-text translation, permission ordering and speech policy remain Core
features. SQLite repositories and image persistence live in Storage. Platform
supplies system OCR, speech, selection, screenshot acquisition, clipboard, fonts,
shortcuts, paths, login registration and single-instance ownership. Application
integrations provide HTTP/LLM transport, packaged local OCR inference, updates,
logs and native-window helpers.

`reference/web` is an isolated React reference harness; `tools/reference` supplies
independent collectors. Neither runs during native startup nor ships as a browser
fallback. Native acceptance, visual references, cross compilation and installer
checks are separate evidence, recorded in the refactor verification record and release
verification documents.
