# SnapLingo UI redesign v2

- `index.html`: approved standalone design reference, including the latest palette and shortcut adjustments.
- `production.html`: interactive review of the production React components using local sample data and in-memory application ports.

Run `npm run dev` at the repository root, then open:

```text
http://localhost:5173/designs/redesign-v2/production.html
```

The production implementation uses seven sidebar destinations (Overview & Shortcuts, Capture, Translation, OCR, Services, General, and Library), with translation/OCR behavior on separate pages and all translation/OCR/speech provider and model configuration together in Services, continuously scrolling settings groups, an overview with system/provider status and all shortcuts, icon tiles for editor bindings, a unified Library with history/favorites modes, a compact capture toolbar with its color/stroke/fill bar, a dock below pinned images, and translation/OCR result cards with local brand assets.

Translation and OCR services use separate cards with an 8px gap and shared drag animations: the selected card follows the pointer at its original size with a soft shadow while nearby cards make room, then eases into place on drop. Dragging uses open space without a placeholder frame; mouse interaction keeps the handle neutral. Handles also support Up/Down keys. OCR order is persisted independently of the selected engine; translation ordering keeps active and inactive providers in their respective groups.

Existing advanced settings and provider dialogs remain available. Editor shortcut conflicts require an explicit replacement action. Pinned window geometry reserves room for the dock while clipboard/save output retains image dimensions; resizing preserves the image center, including on high-DPI displays.

Verification:

```sh
npm test
npm run build
```

Browser review covers navigation, scrolling, shortcut recording/conflicts, provider marks, both palettes, capture color controls, editable OCR and translation handoff, and pinned filters/sliders. Native window operations, OS clipboard, permissions and real provider requests need the desktop runtime; the review page uses local ports for these operations.
