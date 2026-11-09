# Shared capture toolbar and navigation counts

The capture toolbar uses the retained AppKit editor as its independent visual
reference. Test-only exporters on the original toolbar and style views record
their actual `NSView` frames alongside `capture-native-editor-controls.png`,
`capture-native-editor-text-controls.png` and
`capture-native-editor-colors.png`. The original editor workflow passed after
adding these exporters; the production AppKit behavior was not changed.

The shared toolbar preserves the two control rows, grouped shape and arrow
menus, separate system color well and palette menu, stroke/fill/round controls,
and the text font/bold/italic/size row. It uses the original alpha-template PNG
icons, native control spacing and matching enabled/selected state. The font
popup includes its inset rounded bezel and paired arrows. Selection hints use
real font measurement, and the initial frozen image stays undimmed until the
selection state requests tinting.

The checked-in `native-capture-toolbar-layout.json` is independent of renderer
output. `capture_toolbar_layout` compares 207 actual control rectangles at
interface scales 1, 1.25 and 2. Five shared interaction tests cover grouped-tool
selection, palette/system-color separation, disabled actions, toolbar-background
hit testing, long font menu scrolling, committed slider edits and initial
selection behavior. Raster examples under `target/shared-capture` support
comparison with the original PNGs; font antialiasing and native bevels can still
differ between platforms. These results do not establish Windows GUI runtime
acceptance.

Navigation capacity counts now come from the existing `LibraryIndex`. Both the
portable `PreferencesSession` and macOS `SharedFeaturePage` share the same
asynchronous cache. The UI reads cached totals, and history/favorite/screenshot
favorite events trigger a refresh. The existing index clamps page limits to one,
so the lightweight query requests one key without loading record bodies. No
usage fields are added to persisted General settings.

Each refresh receives its revision before spawning. A late older response or a
response after disposal cannot overwrite current totals. A real SQLite test
inserts and deletes history, regular favorites and screenshot favorites and
verifies event-driven totals; a delayed-query test checks revision and disposal
handling. The SettingsWindow integration test also verifies the seeded history
and favorites totals reach a regular settings snapshot.

## Windows pin coordinates

The Windows pin host previously divided physical capture origins by the primary
monitor DPI, then passed a logical position to winit, allowing a second monitor's
DPI to rescale the desktop origin. It also centered only on the primary monitor
and left cascaded pins unclamped. The extracted production geometry reproduced
four failures before the fix.

The host now uses physical window positions. Logical capture origins are
converted once through the core `PrimaryMonitorScale` policy; physical origins
remain exact, including negative coordinates. Centering reuses the shared
virtual-desktop union and bounded 20-DIP cascade. It measures actual window
pixels before reveal and again after a DPI-changing move, preserving the
existing logical size and renderer scaling while restoring the physical origin
or center. Seven geometry cases and five existing presentation/scene cases pass;
the Windows native library cross-check passes. Windows GUI runtime behavior
still requires native-runner acceptance.
