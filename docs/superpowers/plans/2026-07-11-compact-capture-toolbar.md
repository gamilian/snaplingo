# Compact Capture Toolbar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the oversized Capture Workspace toolbar with the approved compact Claude-style single-row toolbar while preserving all existing tools and actions.

**Architecture:** Keep toolbar behavior in `captureEditorToolbar.tsx`, presentation tokens in `capturePresentation.ts`, and placement dimensions in `useCaptureWorkspaceController.ts`. Tests assert the compact visual contract and updated positioning dimensions without coupling to workflow internals.

**Tech Stack:** React, TypeScript, Tailwind utility classes, Vitest, Vite

---

### Task 1: Lock the compact presentation contract

**Files:**
- Modify: `src/components/ScreenshotSession/capturePresentation.test.ts`
- Modify: `src/components/ScreenshotSession/captureWorkspaceDerived.test.ts`

- [ ] **Step 1: Write failing presentation assertions**

Assert that toolbar classes use `h-[42px]`, compact padding/radius, 28 px icon buttons, and 11 px compact text actions rather than the current 56 px/36 px controls.

- [ ] **Step 2: Write failing placement fixture updates**

Change Capture Workspace derived-state fixtures from `{ width: 1220, height: 56 }` to the approved compact dimensions `{ width: 640, height: 42 }`.

- [ ] **Step 3: Run focused tests and verify RED**

Run: `npm test -- capturePresentation captureWorkspaceDerived`

Expected: presentation assertions fail because current toolbar classes remain large.

### Task 2: Implement compact toolbar presentation and SVG actions

**Files:**
- Modify: `src/components/ScreenshotSession/capturePresentation.ts`
- Modify: `src/components/ScreenshotSession/captureEditorToolbar.tsx`
- Modify: `src/components/ScreenshotSession/useCaptureWorkspaceController.ts`

- [ ] **Step 1: Compact shared presentation classes**

Set the toolbar to approximately 42 px high with smaller radius, gap, padding, shadow, icon buttons, dividers, and command controls. Preserve existing active, disabled, and primary states.

- [ ] **Step 2: Replace command labels with the approved controls**

Render Cancel as `ESC`, OCR as `OCR`, Copy as an SVG copy icon, Save as an SVG save icon, and Finish as a blue SVG check icon. Preserve titles, aria labels, callbacks, modifier-click quick save behavior, and keyboard shortcuts.

- [ ] **Step 3: Compact color and size controls**

Reduce color swatch, range width, and fill control sizes while retaining their existing values and event handling.

- [ ] **Step 4: Synchronize placement dimensions**

Set `TOOLBAR_SIZE` to `{ width: 640, height: 42 }` so derived positioning matches the rendered toolbar.

- [ ] **Step 5: Run focused tests and verify GREEN**

Run: `npm test -- capturePresentation captureWorkspaceDerived captureActions`

Expected: all focused tests pass.

### Task 3: Verify and commit

**Files:**
- Verify all modified files

- [ ] **Step 1: Run full frontend tests**

Run: `npm test`

Expected: 85 test files and 602 tests pass.

- [ ] **Step 2: Run production build**

Run: `npm run build`

Expected: TypeScript and Vite build pass; the existing chunk-size warning may remain.

- [ ] **Step 3: Check the diff**

Run: `git diff --check`

Expected: no whitespace errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/ScreenshotSession/capturePresentation.test.ts \
  src/components/ScreenshotSession/captureWorkspaceDerived.test.ts \
  src/components/ScreenshotSession/capturePresentation.ts \
  src/components/ScreenshotSession/captureEditorToolbar.tsx \
  src/components/ScreenshotSession/useCaptureWorkspaceController.ts \
  docs/superpowers/plans/2026-07-11-compact-capture-toolbar.md
git commit -m "refine compact capture toolbar"
```
