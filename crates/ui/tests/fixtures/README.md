The OCR and Translation fixtures comes from the actual existing React page rendered in WKWebView,
using `crates/app/examples/settings_reference_snapshot.rs`. It includes
all card and control bounding rectangles in 84 scenes: three interface themes,
light/dark appearance, 900/700 DIP widths and three OCR or four Translation section routes. Disabled and
offscreen controls are included. No rectangles are derived from shared Rust UI.

To regenerate, start Vite on localhost port 5194, then run the reference example
with `features`, `features dark`, `features compact` and `features dark compact`.
Each run writes actual DOM metrics to `target/native-settings/reference-*.json`.
`layout_report` and `compare-react-layout.mjs` compare those live references to the
shared Rust display list. When updating this retained fixture, copy only x, y,
width and height from the actual reference JSON's card frames and row controls;
keep the original row labels and source names so provenance remains inspectable.

`react-screenshot-layout.json` preserves 36 Screenshot scenes from the same
WKWebView harness: all three themes, light/dark appearance, 900/700 DIP widths,
and Hotkeys/Output/Capture Interface routes. `screenshot_react_layout` compares
1,836 content/card/control rectangles, including all five global shortcuts,
the pin-border shortcut, save path/browse, longshot height, switches, sliders,
selects and all preset color swatches.

The source collector only includes `.settings-control-row` descendants. Its
Screenshot metrics therefore do not contain the annotation shortcut grid,
its restore button, or conditional JPEG/custom-filename rows. The transparent
native color inputs are collected, but their internal frames differ from the
clickable palette labels; the regression checks color swatches instead of
equating those invisible input frames with the shared palette hit targets.
This fixture verifies geometry, not text rasterization or system IME behavior.

The renderer's exact-font React geometry tests for Library, Services,
Permissions and Result use macOS WKWebView references. Their measured text widths
and wrapping depend on macOS fonts; those tests are explicitly ignored on other
platforms. Production continues to use each platform's system fonts, including
Segoe UI on Windows. Portable UI state/input tests, fixed-geometry tests, renderer
text tests, Result theme/panel checks and Result scrolling/hit testing still run
on both platforms. The scrolling test compares WKWebView token coordinates only
on macOS. A Windows-native reference capture is needed for Windows exact-font
geometry acceptance; the macOS fixture is not that baseline.

`react-general-layout.json` contains 72 General scenes: all three themes,
light/dark appearance, 900/700 DIP widths and all six section routes. The actual
React page supplies the card, theme-picker and preference/action rectangles.
`general_react_layout` compares 2,520 rectangles, including the disabled reset
control and all numeric increment/decrement buttons. Run the reference example
with no arguments, `dark`, `compact` and `dark compact` to regenerate
`reference-*-general-*.json`. The General baseline has granted permissions, no
logs and no available update; recovery, populated logs and update states are
separately exercised through the shared model and production controller.

To recapture Screenshot references, start Vite on port 5194 and run
`cargo run --locked --manifest-path crates/app/Cargo.toml --example settings_reference_snapshot -- screenshot`
with no suffix, `dark`, `compact`, and `dark compact`. This writes the same
36 `reference-*-screenshot-*.json` files and corresponding PNGs under
`target/native-settings` without opening the installed application database.

The Screenshot run also records the retained WebKit number field's actual
`stepUp()`/`stepDown()` results in `reference-*-screenshot-number-steps.json`.
`react-screenshot-number-steps.json` retains those independent results for the
shared Longshot height spinner's draft, boundary and empty-input regression.
The field persists its rounded/clamped value only on blur, as the React page does.

`react-library-layout.json` contains 60 actual React Library scenes captured by
`settings_reference_snapshot library [dark] [compact]`: all three interface
themes, both appearances and widths, History translation/OCR and Favorites
translation/OCR/screenshot details. The synthetic records and PNG are owned by
`settings_library_fixture.ts` and `library-preview.png`; no installed records
are read. The collector also exercises pagination, filter reset and a search
with no results, writing 24 extra `search-empty` DOM reports.

`snaplingo-render/tests/library_react_layout.rs` uses the real shared font
measurer and compares 2,016 content/list/control rectangles against that fixture.
Disabled controls are included. Body text wraps through `TextBlock`, image
previews use the same actual PNG pixels, and multiline notes retain line breaks.
Run `cargo run --locked -p snaplingo-render --example library_snapshot` for the
matching 60 shared raster artifacts under `target/shared-library`.
This gate establishes geometry for these fixtures; it does not claim identical
system font antialiasing, all hover states or platform accessibility acceptance.

`react-services-layout.json` preserves 108 actual Services scenes: three themes,
both appearances and widths, OCR/translation/TTS lists, OCR and translation
credentials, DeepLX and standard DeepL, a custom Responses form and its multiline
prompt after scrolling 650 points. Rectangles are compact `[x,y,width,height]`
arrays. The collector mounts the actual `SettingsWindow` and Services components
with `settings_services_fixture.ts`; no installed credentials or settings are read.

Run `settings_reference_snapshot services [dark] [compact]` in all four variants,
then `node script/record-services-layout.mjs` to replace only the recorded DOM
geometry and enabled flags. The recording script does not read renderer output.
`cargo test --locked -p snaplingo-render --test services_react_layout --offline`
uses actual `RendererTextMetrics` and independently renders the paired provider,
voice/schema/prompt DTOs; it checks 1,788 content/card/control frames and button
enabled state without depending on `target/`. Prompt-save foreground/background
colors are also retained: the original button is clickable (the collector calls
the real save handler), despite its raised/faint appearance. The same DTO fixture powers
`cargo run --locked -p snaplingo-render --example services_snapshot` for raster QA.

`react-permission-guide-layout.json` contains 24 retained React
`RequiredPermissionsGate` scenes: Native/Studio/Paper, light/dark, initial,
attempted authorization, failed authorization and temporary installation.
`settings_reference_snapshot permissions [dark]` mounts the actual component
with an isolated permission port. It records its card/action rectangles and
computed font metrics without reading or requesting real OS permissions.
`permission_prompt_layout` compares the shared card and controls within 0.5 DIP;
`permission_prompt_snapshot` renders the corresponding 24 shared PNGs. The native
AppKit sheet lifecycle is separately covered by `native_app_smoke --permissions`.

`native-capture-toolbar-layout.json` records actual AppKit `NSView` frames from
the retained capture editor, including its rectangle style row, text style row
and color palette. The test-only collector writes these beside the original
toolbar PNGs when `native_ui_workflows --surface editor` runs with
`SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR` set. The recorded JSON comes from
`EditorToolbar::test_geometry` and `EditorStyle::test_geometry`, never shared UI
coordinates. Its `text` member is the independently captured text-mode scene.

`cargo test --locked -p snaplingo-render --test capture_toolbar_layout --offline`
compares 207 actual control frames at interface scales 1, 1.25 and 2, including
disabled and selected state. `cargo run --locked -p snaplingo-render --example
capture_toolbar_snapshot --offline` writes shared toolbar, text-style, palette
and grouped-tool PNGs under `target/shared-capture`. These checks preserve the
original controls' geometry; platform font antialiasing and native bevels are
inspected separately in the paired rasters.
