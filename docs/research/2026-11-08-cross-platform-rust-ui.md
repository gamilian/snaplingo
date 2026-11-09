# Shared Rust desktop drawing on macOS and Windows

Investigated on 2026-11-08 against first-party source. The requested direction is one Rust layout, drawing and interaction implementation on macOS and Windows, with native window/input/presentation adapters. The existing AppKit preference controls are migration work already present in this checkout; adding independent Windows versions of those controls would duplicate the UI and would not establish shared drawing.

## Recommendation

Use a portable UI crate containing layout, theme tokens, hit testing, editing state and CPU rendering. `tiny-skia` 0.11.4 supplies shapes/images; `cosmic-text` 0.14.2 supplies shaping, fallback, layout and CPU glyph rendering. `winit` 0.30.12 can own the normal desktop window/event loop on both required platforms. `softbuffer` 0.4.8 is suitable for **opaque** window presentation, including Settings, but its pixel format does not support per-pixel alpha. Keep a small native alpha presenter for transparent overlay/pinned windows. These are source-supported implementation choices, not measurements or completed platform acceptance.

This follows AirSnip's useful separation—software pixels plus native window mechanics—without adopting its Windows-only text/layout code or claiming that its published resource figures apply to SnapLingo.

## What AirSnip actually implements

The audited `main` revision is [`7d4178fa7283b10d8f5123a2981c16daaa4d85fd`](https://github.com/bd-dxg/airsnip/tree/7d4178fa7283b10d8f5123a2981c16daaa4d85fd). Its [manifest](https://github.com/bd-dxg/airsnip/blob/7d4178fa7283b10d8f5123a2981c16daaa4d85fd/Cargo.toml) declares v0.9.0, `windows` 0.62.2 and `tiny-skia` 0.12.0. Thus the proposed tiny-skia 0.11 version is a deliberate SnapLingo dependency choice, not the version in current AirSnip.

- Its [README](https://github.com/bd-dxg/airsnip/blob/7d4178fa7283b10d8f5123a2981c16daaa4d85fd/README.md) explicitly targets Windows 10+ x64 and describes tiny-skia plus DirectWrite. Its size, idle memory and latency figures are measurements of its own application. Mixed-DPI real-hardware validation remains a documented limitation.
- The [Settings renderer](https://github.com/bd-dxg/airsnip/blob/7d4178fa7283b10d8f5123a2981c16daaa4d85fd/src/settings/panel/draw.rs) draws navigation, rows, toggles and text into a pixmap. Its [window adapter](https://github.com/bd-dxg/airsnip/blob/7d4178fa7283b10d8f5123a2981c16daaa4d85fd/src/settings/panel/window.rs) owns HWND creation, DPI sizing and the presentation DIB. This is self-drawn UI, not a native control toolkit.
- Its [main application loop](https://github.com/bd-dxg/airsnip/blob/7d4178fa7283b10d8f5123a2981c16daaa4d85fd/src/app/mod.rs) blocks in `GetMessageW`, followed by `TranslateMessage`/`DispatchMessageW`. Update checking uses a delayed timer and worker notification; this is not an always-redrawing render loop. The [window procedure](https://github.com/bd-dxg/airsnip/blob/7d4178fa7283b10d8f5123a2981c16daaa4d85fd/src/app/wndproc.rs) receives hotkeys and timers.
- Its [overlay presenter](https://github.com/bd-dxg/airsnip/blob/7d4178fa7283b10d8f5123a2981c16daaa4d85fd/src/overlay/surface.rs) converts pixmap regions from RGBA to DIB BGRA and calls `BitBlt` for local damage. Its [longshot presenter](https://github.com/bd-dxg/airsnip/blob/7d4178fa7283b10d8f5123a2981c16daaa4d85fd/src/longshot/present.rs) instead uses premultiplied BGRA and `UpdateLayeredWindow` with alpha. Opaque and alpha presentation already require different mechanics in the reference architecture.
- Its [text renderer](https://github.com/bd-dxg/airsnip/blob/7d4178fa7283b10d8f5123a2981c16daaa4d85fd/src/render/text.rs) uses DirectWrite bitmap render targets and memory DIBs. Its [IME adapter](https://github.com/bd-dxg/airsnip/blob/7d4178fa7283b10d8f5123a2981c16daaa4d85fd/src/overlay/ime.rs) associates Windows IME contexts. Neither implementation is portable to AppKit.

AirSnip's manifest and README specify **GPL-3.0-only**. This MIT repository should reference the architecture and public behavior; do not copy its implementations, rendering paths, custom widgets, or text/IME code. The observations above were made by reading source, not importing it into SnapLingo.

## Verified candidate capabilities and limits

| Component | Verified capability | Boundary SnapLingo must implement |
| --- | --- | --- |
| winit 0.30.12 | macOS/Windows window and event backends; explicit redraw; wait-based loop; keyboard, pointer, focus, IME and DPI events | Widgets, editing semantics, application action routing, accessibility, platform-specific alpha presentation |
| softbuffer 0.4.8 | CPU-written pixel buffers; AppKit and Win32 backends; raw-window-handle 0.6 integration | RGBA conversion; background composition; no per-pixel-alpha output; AppKit presentation copies pixels |
| tiny-skia 0.11.4 | CPU-only shape rasterization, clipping, gradients, image blending; premultiplied RGBA8888 pixmaps | No text engine, widget layout, input handling or resource cache |
| cosmic-text 0.14.2 | Rust shaping/fallback/bidi/line layout, CPU swash rendering, cursor/selection/editor actions; documented full macOS/Windows/Linux feature support | System IME host, clipboard, focus/shortcut policy, undo grouping, accessibility, UI widgets |

The winit [event loop source](https://github.com/rust-windowing/winit/blob/v0.30.12/src/event_loop.rs) defaults to `ControlFlow::Wait`; workers can wake it through `EventLoopProxy::send_event`. Its [window source](https://github.com/rust-windowing/winit/blob/v0.30.12/src/window.rs) supplies visibility, transparency requests and cursor hit testing. A transparency request controls the window configuration; it does not change the pixel format accepted by a separate presenter.

The softbuffer [README](https://github.com/rust-windowing/softbuffer/blob/v0.4.8/README.md) lists AppKit and Win32 and states that no GPU/hardware graphics stack is required. Its [buffer contract](https://github.com/rust-windowing/softbuffer/blob/v0.4.8/src/lib.rs) requires `0x00RRGGBB`: the high byte is zero, not alpha. Its [AppKit/CoreGraphics backend](https://github.com/rust-windowing/softbuffer/blob/v0.4.8/src/backends/cg.rs) creates the CGImage with `CGImageAlphaInfo::NoneSkipFirst`; its [Win32 backend](https://github.com/rust-windowing/softbuffer/blob/v0.4.8/src/backends/win32.rs) uses `BitBlt`. Therefore writing RGBA bytes into a softbuffer buffer, or enabling a transparent winit window, cannot be treated as a valid alpha solution. The documented AppKit `present()` path blocks while copying image data.

The tiny-skia [README](https://github.com/RazrFalcon/tiny-skia/blob/v0.11.4/README.md) explicitly excludes text and GPU rendering. Its [pixmap source](https://github.com/RazrFalcon/tiny-skia/blob/v0.11.4/src/pixmap.rs) defines premultiplied RGBA storage. This supports a platform-independent rendering result: opaque presenters can flatten/convert it; alpha presenters preserve premultiplication and convert channel order only as required.

The cosmic-text [README](https://github.com/pop-os/cosmic-text/blob/0.14.2/README.md) documents rustybuzz shaping, swash rendering, bidirectional layout, fallback and full feature support on macOS/Windows/Linux. Its [manifest](https://github.com/pop-os/cosmic-text/blob/0.14.2/Cargo.toml) requires Rust 1.75 and uses tiny-skia 0.11.2 in development, which is evidence that the requested rendering generation is a practical pairing. Its [buffer](https://github.com/pop-os/cosmic-text/blob/0.14.2/src/buffer.rs) exposes size/metrics, layout runs, hit testing and CPU pixel callbacks; its [editor](https://github.com/pop-os/cosmic-text/blob/0.14.2/src/edit/editor.rs) draws selection/cursor, while the [editing interface](https://github.com/pop-os/cosmic-text/blob/0.14.2/src/edit/mod.rs) provides string insertion, selection copying, navigation and pointer actions. Clipboard transfer remains a host responsibility. Font fallback still depends on fonts available to the configured FontSystem; for reproducible UI references, load the same bundled UI font deliberately rather than relying entirely on each OS's default font.

The inspected manifests license [winit](https://github.com/rust-windowing/winit/blob/v0.30.12/Cargo.toml) as Apache-2.0, [softbuffer](https://github.com/rust-windowing/softbuffer/blob/v0.4.8/Cargo.toml) and [cosmic-text](https://github.com/pop-os/cosmic-text/blob/0.14.2/Cargo.toml) as MIT OR Apache-2.0, and [tiny-skia](https://github.com/RazrFalcon/tiny-skia/blob/v0.11.4/Cargo.toml) as BSD-3-Clause. Generated third-party notices still need updating when these become production dependencies.

## IME and DPI are explicit contracts

Winit's [IME event definition](https://github.com/rust-windowing/winit/blob/v0.30.12/src/event.rs) separates `Enabled`, `Preedit(String, Option<(usize, usize)>)`, `Commit(String)` and `Disabled`. Preedit selection offsets are **UTF-8 byte offsets**. The [window API](https://github.com/rust-windowing/winit/blob/v0.30.12/src/window.rs) requires explicit `set_ime_allowed(true)` while a text field has focus, and `set_ime_cursor_area` to position the candidate window. IME is disabled by default; on macOS it must also be enabled for composed dead-key text.

The [Windows backend](https://github.com/rust-windowing/winit/blob/v0.30.12/src/platform_impl/windows/ime.rs) reads IMM composition and result strings and sets composition/candidate positions. The [AppKit backend](https://github.com/rust-windowing/winit/blob/v0.30.12/src/platform_impl/macos/view.rs) implements `NSTextInputClient` and emits the shared preedit/commit events. It also exposes a material limitation: `selectedRange` returns NSNotFound, attributed surrounding text is unavailable, and replacement ranges in `setMarkedText`/`insertText` have TODOs and are ignored. Ordinary composition support is not evidence of all the reconversion/surrounding-text behavior supplied by an NSTextView.

The shared editor should own committed text, selection and a separate composition draft. Draw preedit and its selection without recording each preedit update as an edit; insert the final committed string once. While composing, Enter/Esc and navigation must reach the IME before capture completion, page submission, close or shortcut handlers. Cancel composition on focus loss/disposal and disable IME during shortcut recording. A native text-input adapter may later be needed for richer system text services, but it must feed the same Rust editing model and caret layout.

The [DPI event](https://github.com/rust-windowing/winit/blob/v0.30.12/src/event.rs) includes the new device scale and `InnerSizeWriter`; moving across monitors or changing display scale can trigger it. Keep device scale separate from SnapLingo's configured interface scale. Convert pointer/caret/layout coordinates consistently, recreate physical-size buffers on scale/resize, and position candidate windows from the final transformed caret. The [macOS extensions](https://github.com/rust-windowing/winit/blob/v0.30.12/src/platform/macos.rs) support Accessory activation policy, disabling the default menu and Retina configuration; existing tray/menu behavior can remain native.

## Smallest useful shared boundary

Create a workspace crate such as `snaplingo-ui` with no AppKit/Win32/Tauri types in its public model. Start with one real Settings surface and reuse the original semantic tokens and reference fixtures.

Its responsibilities should be concrete:

1. `Viewport` holds logical dimensions, device scale and interface scale. Layout and hit testing consume one logical coordinate system.
2. `UiInput` normalizes pointer/wheel/key/focus and text/preedit/commit input. State transitions produce typed application intents; business services remain in the existing Application modules.
3. `Frame` holds dimensions/stride and premultiplied RGBA pixels plus damage rectangles. The same renderer creates both on-screen frames and snapshot artifacts.
4. Text widgets share cosmic-text state, selection, caret geometry and composition handling across platforms. Widget visuals use the same scene/layout functions as labels and controls.
5. A narrow host consumes frames, requests redraws, updates the cursor and IME caret/enablement, and connects clipboard and application intents. The host does not choose page layout, button metrics or colors.

For normal opaque windows, winit plus softbuffer can implement this host on both macOS and Windows. On macOS there must be one owner of the NSApplication loop—do not start an independent `NSApplication::run` alongside winit's event loop. Existing native windows can be retained during migration, but their lifetime and main-thread dispatch must be coordinated by that one shell. For transparent surfaces, the same shared frame goes through an AppKit CGImage/layer presenter or Win32 layered-window presenter. This keeps platform code restricted to mechanisms and avoids recreating every widget twice.

Keeping entirely hand-written AppKit/Win32 hosts is also feasible and gives direct IME/window control, but would require maintaining event normalization, DPI transitions, focus, keyboard dispatch and wakeups twice. Winit already implements those mechanisms; replacing it should be justified by a demonstrated integration gap. Conversely, using softbuffer for every window despite its alpha contract would be an incorrect simplification.

## Required acceptance before removing the retained renderer

- Compare real original Settings references with shared-renderer snapshots for Native/Studio/Paper, light/dark, narrow/normal widths and configured scale. Check controls, scrolling, clipping, fonts, focus, selection and hit targets, not just blank frames.
- Exercise Chinese/Japanese IME preedit, commit, cancellation, Enter/Esc, dead keys, surrogate/emoji sequences, selection replacement and candidate tracking while scrolling/resizing on both operating systems. Record the winit surrounding-text limitation honestly.
- Run actual Windows and macOS startup, tray, single-instance, close-to-hide, shortcuts, capture, alpha/pinned windows, native dialogs and permission/restart workflows. Building on one platform does not establish acceptance on the other.
- Measure event-driven idle use and capture/drag/typing frames on fixed hardware. A shared 1800×1300 RGBA buffer alone occupies about 8.9 MiB; extra cached frames and presentation copies are real costs. AirSnip's much smaller feature set and its own measurements are reference context, not promised SnapLingo results.

This investigation added no UI dependency and replaced no window. The existing Library index, History and Favorites application services can feed either shared drawing host; their pagination and action behavior should remain application-owned.

## Native accessibility continuation

The subsequent desktop implementation uses `accesskit_winit` 0.34.2 and `accesskit` 0.25.2. The [official manifest](https://github.com/AccessKit/accesskit/blob/9095a7a1d5b818ff355a7a90b0a809a4a6772b87/adapters/winit/Cargo.toml) declares winit `^0.30.5`, compatible with the resolved winit 0.30.13, and native `accesskit_macos` 0.27.2 / `accesskit_windows` 0.35.2 adapters. Their Rust 1.87 requirement fits this workspace's Rust 1.88 minimum. The adapter is enabled with `default-features = false, features = ["rwh_06"]`; no Linux async runtime is added for the two required targets.

The [winit adapter API](https://github.com/AccessKit/accesskit/blob/9095a7a1d5b818ff355a7a90b0a809a4a6772b87/adapters/winit/src/lib.rs) requires construction before the window is shown and `process_event` before application event handling. `with_mixed_handlers` synchronously returns a cached complete semantic tree on activation and sends native actions through the existing EventLoopProxy. This avoids an empty activation tree and does not add idle polling. The official [mixed-handler example](https://github.com/AccessKit/accesskit/blob/9095a7a1d5b818ff355a7a90b0a809a4a6772b87/adapters/winit/examples/mixed_handlers.rs) demonstrates this pattern. The [AccessKit coordinate contract](https://github.com/AccessKit/accesskit/blob/9095a7a1d5b818ff355a7a90b0a809a4a6772b87/accesskit/src/lib.rs) requires final node coordinates in physical pixels relative to the native container, so the new projection multiplies Scene geometry by the device scale exactly once; interface zoom is already present in Scene.

`snaplingo-accessibility` supplies navigation and section selection, Switch toggled state, ComboBox values/expanded popups, Slider values/bounds/steps, hotkey values/recording hints, disabled reset/clear controls, scrolling and status announcements. It takes bounds from the same Scene geometry/hit regions as rendering, including controls that can be focused and scrolled into view. Native actions map to the shared UI's validated Focus/Activate/SetValue/key/scroll events and then existing backend dispatch. Late actions for closed popups or another page are rejected. Host adapters cache the full projection and publish changed nodes.

The macOS adapter [subclasses the actual NSView](https://github.com/AccessKit/accesskit/blob/9095a7a1d5b818ff355a7a90b0a809a4a6772b87/adapters/winit/src/platform_impl/macos.rs); the Windows adapter [subclasses the actual HWND](https://github.com/AccessKit/accesskit/blob/9095a7a1d5b818ff355a7a90b0a809a4a6772b87/adapters/winit/src/platform_impl/windows.rs). These are native accessibility adapters, not invisible visual controls. Tests verify geometry/DPI, action routing, focus without activation, disabled controls, popup lifecycle, range edits and stale-request rejection. They do not establish real VoiceOver/Narrator acceptance or full text-editor accessibility; those remain platform acceptance work.

## Implementation validation on 2026-11-08

The subsequent implementation now contains real Core, Storage, UI, Render, Accessibility, Settings, Assets and Desktop crates. Core and Storage contain the moved production workflows and persistence code; the Settings adapter connects shared OCR, Translation and Screenshot preference pages to those workflows. The standalone desktop example uses a temporary SQLite database and a recording-only shortcut registrar. The workspace resolves winit 0.30.13 under the declared 0.30.12 requirement. This records the implemented checkpoint separately from the candidate research above.

On the available macOS arm64 host, this command completed successfully:

```sh
cargo check --locked -p snaplingo-ui -p snaplingo-render -p snaplingo-desktop --target x86_64-pc-windows-gnu --lib --offline
```

After installing the official Homebrew MinGW-w64 toolchain, the broader Windows checks also completed:

```sh
cargo check --locked -p snaplingo-ui -p snaplingo-render -p snaplingo-desktop --target x86_64-pc-windows-gnu --all-targets --offline
cargo check --locked -p snaplingo --no-default-features --features native-app --target x86_64-pc-windows-gnu --lib --offline
```

The SQLite/WebP build scripts now pass with `x86_64-w64-mingw32-gcc`. The application check includes the native Windows result, capacity and notices hosts and the shared Settings backend. Cross-checking does not establish executable linking, packaging or Windows runtime behavior. `.github/workflows/native-ui.yml` remains the native-runner path; its existence is not a report of a completed CI run. The default release command still packages the retained application through Tauri.

The standalone Screenshot slice adds shared text editing for paths, custom file names and maximum image height. Committed text and UTF-8 selection remain separate from the preedit draft; the renderer supplies caret geometry and the desktop host routes IME/clipboard events. This is not acceptance of complete system text services. The installed winit 0.30.13 source retains the byte-offset and pre-commit empty-preedit contracts and the AppKit surrounding-text/replacement-range limitations described above. Chinese/Japanese IME and assistive-technology behavior still require real macOS and Windows acceptance.

## Production macOS preference continuation

The retained production Settings window first embedded shared OCR and Translation pages through `shared_view_macos.rs` and `shared_page_macos.rs`. Each page has one AppKit drawing view and no per-control AppKit subviews. Retained navigation passes content bounds and interface zoom; the view crops the same shared scene, paints its premultiplied RGBA directly into an NSBitmapImageRep, and routes normalized input back to SettingsUi. The AppKit accessibility adapter uses the same cropped semantic projection, with backing scale applied once and host zoom supplied by the NSView transform. A backing-properties change redraws the cached raster.

These pages use the existing production AppState, SettingsConfiguration, SettingsApplication and HotkeyRuntime rather than opening a second database. Desktop event subscriptions refresh current snapshots on the main thread; recording completion requires the owning session token, and leaving the focused window cancels it. Page disposal releases subscriptions, local event monitoring, accessibility state and its own recording session. At that checkpoint General retained AppKit controls and Screenshot was a standalone slice. The continuation below replaces both production pages with the shared renderer.

The real `native_ui_workflows -- --surface settings` harness passed all seven retained navigation routes, OCR/Translation persistence and remount, Native/Studio/Paper resizing across interface scales 1/1.25/1.5/1.75/2, direct raster dimensions, zero per-control subviews, stale recording rejection, valid recording persistence, focus cancellation and capacity-dialog hide/restore. This exercised the actual macOS window server with the retained Vite frontend running. It does not establish VoiceOver/Narrator acceptance, an actual Windows runtime, or removal of the remaining Tauri application/build shell.


## General and Screenshot continuation (2026-11-08)

The production macOS route now mounts General and Screenshot through the same
`SharedFeaturePage`/`SharedFeatureView` used by OCR and Translation. General
projects its existing `GeneralController` into shared runtime state and routes
permissions, logs, history, notices and updates back to that controller. Its six
sections, twelve editable fields, theme cards, number inputs and disclosures are
shared Rust drawing. Destructive General commands retain native confirmation
sheets; disposed pages invalidate late update results.

Screenshot now uses the application's existing SettingsConfiguration and
HotkeyRuntime, including all five global shortcuts and local annotation/pin
shortcuts. Native directory and color dialogs only save confirmed choices. The
AppKit canvas implements NSTextInputClient against the shared TextEdit, converts
UTF-16 ranges, returns surrounding text and shaped caret geometry, separates
marked text from committed values, and routes clipboard commands. Partial
replacement of marked text retains its unselected text. Candidate confirmation
and cancellation do not submit the field or leak a draft into another field.
The shared longshot number arrows follow captured WebKit stepUp/stepDown behavior
and retain the original save-on-blur semantics.

New fixtures come from actual retained WebKit pages: 72 General scenes (2,520
rectangles) and 36 Screenshot scenes (1,836 rectangles), alongside the existing
84 OCR/Translation scenes. They cover Native/Studio/Paper, light/dark and normal/
compact widths. Screenshot reference capture has a reproducible `screenshot`
entry point; its conditional JPEG/custom filename rows and annotation grid have
interaction coverage rather than fabricated DOM geometry. Geometry agreement
within 0.5 DIP does not prove identical font rasterization, shadows or icons.

The shared desktop host and the no-Tauri Windows application library pass GNU
cross-checks, including the real Settings/Services/Library adapters, result window,
capacity dialog and notices. Actual Windows UI, VoiceOver/Narrator, and
Chinese/Japanese input-method acceptance remain required. The AppKit protocol
regression verifies real canvas callbacks and persistence, not a complete system
IME keyboard session.

A standalone macOS native entry now owns the NSApplication loop, tray, shortcuts,
single instance, seven-page Settings canvas and lifecycle. Its real AppKit smoke
passed. `npm run native:build` builds and signs an independent native `.app`;
`npm run native:bundle -- --debug` passed resource, identity, architecture,
hardened-runtime and macOS deployment checks locally with explicit ad-hoc mode.
The remaining work includes Services visual acceptance, Windows capture/pin/
scrolling and application startup acceptance, release packaging for Windows and
switching the formal default away from Tauri.

### Verified production continuation

The final `native_ui_workflows --surface settings` run passed with shared General,
Screenshot, OCR and Translation. It exercised all seven navigation entries,
three themes and five configured scales for General and Translation, resizing,
About-section scrolling, the retained capacity-modal hide/restore, real shortcut
recording, Screenshot NSTextInputClient callbacks, and persisted edits across
page disposal/remount. An unchanged-scene regression also verifies bitmap reuse.
The host now applies shell width and frame size together and reuses equal scenes
at the same device/zoom scale; this avoids redundant CPU rasterization during
focus and repeated SettingsChanged events. DOM probes use WKWebView completion
callbacks and allow 30 seconds for unoptimized native redraws, while geometry and
persistence assertions retain their original tolerances.
