# Compact Capture Toolbar Design

## Goal

Make the Capture Workspace editor toolbar compact enough to remain visible around small selections without removing existing tools or actions.

## Visual Direction

- Keep the previously approved Claude-style visual language: light neutral surface, restrained shadow, rounded corners, thin rounded SVG strokes, and a blue primary action.
- Reduce the toolbar height from 56 px to approximately 42 px.
- Use 28 px tool buttons with approximately 17 px SVG icons.
- Reduce horizontal gaps, dividers, color swatch size, slider width, and outer padding.
- Keep the toolbar in a single row.

## Actions

- Cancel is a compact `ESC` text button.
- OCR is a compact `OCR` text button.
- Copy remains an SVG copy icon.
- Save remains an SVG save/download icon.
- Finish remains a blue SVG check icon.
- Every icon-only action keeps a descriptive `title` and `aria-label`; shortcuts remain available through tooltips and keyboard handling rather than visible button text.

## Layout Contract

- Update the toolbar dimensions used by `useCaptureWorkspaceController` so placement calculations match the rendered toolbar.
- Keep a stable explicit toolbar width rather than measuring the DOM at runtime.
- Preserve the current above/below selection placement behavior and viewport clamping.

## Scope

- Update `captureEditorToolbar.tsx` markup and SVG icon presentation.
- Update compact toolbar classes in `capturePresentation.ts`.
- Update toolbar width and height constants in `useCaptureWorkspaceController.ts`.
- Update focused presentation and positioning tests where class or size expectations change.
- Do not change capture workflow behavior, keyboard shortcuts, annotation state, or output actions.

## Verification

- Run Capture Workspace and presentation tests.
- Run the complete frontend test suite.
- Run the production frontend build.
- Visually verify that the toolbar remains single-line, has no text overflow, and uses the compact dimensions at small and normal selection sizes.
