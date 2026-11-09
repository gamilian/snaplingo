# Shared Settings UI

This crate owns the portable General, OCR, Translation and Screenshot Settings pages’ state, layout, display list and
hit testing. It has no windowing, platform, font, raster or backend dependencies.
The host supplies `Snapshot` and executes `Action`; the renderer draws `Scene`.

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
remain host actions. This is a migration component, not the complete app UI.

`Event::Activate`, `Focus` and `SetValue` reuse the same validated interaction
paths for accessibility. Pointer-down/move/up drive slider gestures. Inspect
`page()` before executing hotkey actions to select its Screenshot, OCR or Translation category.

`text_edit()` exposes committed text, UTF-8 selection and a separate preedit draft
for native input-method adapters. `TextSelection`, `ImePreedit`, `ImeCommit` and
`ImeDisabled` update the same editor used for clipboard and keyboard input.
The host supplies shaped caret metrics and converts native text-service offsets.
