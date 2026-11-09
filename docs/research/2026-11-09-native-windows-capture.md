# Windows capture host and shared presentation

The no-Tauri Windows application presents a single capture HWND on the existing
winit event loop. `capture_window/windows.rs` implements the existing core
runtime host; `windows/view.rs` owns its window lifetime and adapts shared
selection/editor state to `snaplingo-ui::capture`. Input, accessibility, text
geometry and raster assets are separate small adapter modules. Output actions
continue through `CaptureSessionRuntime`, including annotated copy/save, pin,
OCR/translation, screenshot favorites and scrolling capture.

The desktop source normalizes all monitor bounds to the primary monitor scale.
The spanning HWND therefore uses physical position/size, pointer input divides
by that desktop scale, and rendering applies that scale. The HWND's current
monitor DPI is kept separate. This preserves negative monitor origins and
alignment between frozen pixels, selection and annotation coordinates.
`LayeredPresenter` supplies premultiplied alpha through `UpdateLayeredWindow`.

Core `CaptureEditor` still owns annotation gestures, stacking, selected handles,
undo/redo and styles. Preview requests coalesce and reject superseded revisions.
Moving annotations retain their stacking order; editing text replaces its
committed annotation in the preview. Text input uses the shared grapheme/IME
model, and its caret positions use the same selected font, fallback glyphs,
physical font-size rounding and glyph advances as final annotation output.
The host supports mouse selection, word/document navigation, clipboard,
multiline text and text undo/redo. Accessibility exposes session-scoped stable
buttons, sliders and editable text with actions through the same UI model.

Save/print dialogs suspend the overlay and preserve the editor on cancellation
or error. Printing uses the owner's native Windows print dialog and GDI rather
than depending on an image filename association. Scrolling completion closes
the corresponding overlay; cancellation restores it. System-color cancellation
and asynchronous errors only update the matching current session.

Verification performed locally:

- `cargo check --locked -p snaplingo --no-default-features --features native-app --target x86_64-pc-windows-gnu --lib --bins --test native_app_smoke --offline`
- Five portable capture presentation tests cover mixed-DPI negative coordinates,
  cursor hotspots, selection/editor movement/cancellation, annotation stacking,
  text drafts, stale preview results and grapheme-safe IME caret geometry.
- Shared capture interaction tests and the independent native-toolbar geometry
  fixture cover grouped tools, styles, palettes, disabled/busy controls, popup
  lifecycle and access to long font lists.
- The original AppKit capture/editor workflows were executed with synthetic
  frozen pixels. Their actual PNGs are in `target/native-capture-reference`.
  `cargo run --locked -p snaplingo-render --example capture_selection_snapshot --offline`
  reproduces paired shared desktop/selection/lens PNGs in `target/shared-capture`.
  `capture_toolbar_snapshot` supplies toolbar, palette and grouped-tool PNGs.

This establishes compilation, portable behavior and the inspected layout. It
does not establish actual Windows capture/printing/device-driver behavior,
Windows Chinese/Japanese IME acceptance or Narrator acceptance. AppKit and the
shared renderer retain differences in system font shaping and antialiasing;
these checks do not claim pixel-identical text across platforms.
