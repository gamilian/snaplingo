# Capture toolbar icons

These SVGs preserve the icon paths from `src/views/CaptureWorkspace/captureEditorToolbar.tsx`. The adjacent PNGs render the same geometry at 68 × 68 pixels, four times the 17-point display size. AppKit uses their alpha channel as a tintable template so the native toolbar keeps the frontend's shapes in either appearance.

The SVGs are the editable source. Regenerate a PNG from its SVG with an SVG rasterizer at 68 × 68 pixels after changing an icon; preserve transparent backgrounds. No external icon set or generated artwork is used.
