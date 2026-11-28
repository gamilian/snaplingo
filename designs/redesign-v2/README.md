# SnapLingo UI redesign v2

- `index.html`: approved standalone design reference, including the latest palette and shortcut adjustments.
- `production.html`: interactive review of the production React components using local sample data and in-memory application ports.

Run `npm run dev` at the repository root, then open:

```text
http://localhost:5173/designs/redesign-v2/production.html
```

The production implementation uses six sidebar destinations, continuously scrolling settings groups, an overview with system/provider status and all shortcuts, icon tiles for editor bindings, a unified Library with history/favorites modes, a compact capture toolbar with its color/stroke/fill bar, a dock below pinned images, and translation/OCR result cards with local brand assets.

Existing advanced settings and provider dialogs remain available. Editor shortcut conflicts require an explicit replacement action. Pinned window geometry reserves room for the dock while clipboard/save output retains image dimensions; resizing preserves the image center, including on high-DPI displays.

Verification:

```sh
npm test
npm run build
```

Browser review covers navigation, scrolling, shortcut recording/conflicts, provider marks, both palettes, capture color controls, editable OCR and translation handoff, and pinned filters/sliders. Native window operations, OS clipboard, permissions and real provider requests need the desktop runtime; the review page uses local ports for these operations.
