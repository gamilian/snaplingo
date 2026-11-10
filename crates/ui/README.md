# Shared Rust UI

This package owns portable view state, layout, input and drawing for Settings,
Capture, Result, Pinned Image and Scrolling Capture. Native hosts supply snapshots,
execute actions and present the resulting scene or native control geometry.
No module creates windows, opens a database, or calls an operating-system toolkit.

The default build needs only Unicode text segmentation. Optional Cargo features
keep CPU rendering and AccessKit projection in the same package without making
those dependencies mandatory for layout and interaction:

```toml
snaplingo-ui = { path = "../../crates/ui", features = ["rendering", "accessibility"] }
```

- `foundation/`: geometry, drawing commands, text editing, normalized keys and theme.
- `settings/`: settings models, fields, state/input, drawing, capacity and page routing.
- `capture/`, `result/`, `pinned/`, `scrolling.rs`, `services/`, `library/`: feature views.
- `render/`: optional CPU rasterization and text measurement, accessed through
  `snaplingo_ui::render::{Renderer, RendererTextMetrics, Frame}`.
- `accessibility/`: optional semantic projection, accessed through
  `snaplingo_ui::accessibility`; native AX/UIA adapters remain in the desktop host.

All Rust implementation files live under `src/`. The crate root exposes the small
public entry points; feature state, drawing and tests stay beside their owner.
The existing root exports for common UI values and Settings remain available.

```rust
use snaplingo_ui::{SettingsUi, Snapshot, Size};
let mut ui = SettingsUi::new(Snapshot::default());
let scene = ui.frame(Size { width: 900.0, height: 650.0 }, 1.25);
```

Window sizes and pointer positions use logical DIP. The interface scale is the
user's 100%/125% setting. Both drawing commands and hit regions contain that same
scale; the renderer applies the monitor's device pixel ratio independently.
Text commands carry a wrapping/clipping rectangle, font family, size and weight.
Clips are nested and hit regions are intersected with the corresponding clip.

Map OS input to `Event` and dispatch the returned actions in order. Complete
backend work by delivering `Event::Snapshot` with the committed projection.
Set `Snapshot::key_style` to `Windows` for Windows Ctrl/Win display and storage
semantics; the default is macOS. Begin/end recording actions let the host capture
already registered global shortcuts, which it returns as `RecordedShortcut`.

The shared pages include General (six sections), OCR (seven fields, three sections),
Translation (15 fields, four sections) and Screenshot (three sections), including
text fields, shortcut rows, sliders, theme cards and light/dark appearances.
General's permission, log, update and about state comes from `GeneralRuntime`;
its system commands require a host. Navigation among these four pages changes
the active page in the shared state; other routes remain host actions.
Other navigation items emit `Navigate` to the host. They do not replace the page
with incomplete content. Capacity, shortcut reset and conflict actions likewise
remain host actions. The product host owns business workflows and window lifetime.

`Event::Activate`, `Focus` and `SetValue` reuse the same validated interaction
paths for accessibility. Pointer-down/move/up drive slider gestures. Inspect
`page()` before executing hotkey actions to select its Screenshot, OCR or Translation category.

`text_edit()` exposes committed text, UTF-8 selection and a separate preedit draft
for native input-method adapters. `TextSelection`, `ImePreedit`, `ImeCommit` and
`ImeDisabled` update the same editor used for clipboard and keyboard input.
The host supplies shaped caret metrics and converts native text-service offsets.

`result.rs` also owns the Result window's presentation, actions and logical layout.
Both AppKit controls and the Windows scene consume it; text measurement and
native text services stay in their adapters. AppKit geometry is the reference
for header, source card, provider cards, OCR controls and copy-fragment popup.

`capture/presentation.rs` owns the editor toolbar's placement, control geometry,
enabled/selected state, style ranges and error rows, plus palette geometry and
mutation availability. AppKit controls and the shared scene consume the same
presentation. It also owns selection labels, masks, magnifier geometry and sample
rectangles. Hosts supply frozen pixels and measured text, retain native text and
color input, and convert the returned intents into Core editor actions. Native
coordinates are normalized to top-left logical coordinates before using it.
