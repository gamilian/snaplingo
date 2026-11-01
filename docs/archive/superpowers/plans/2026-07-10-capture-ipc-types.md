# Capture IPC Types Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move Capture Session, OCR, and Pinned Image frontend IPC shapes out of `ScreenshotSession` so the Frontend Tauri Adapter no longer depends on a UI module.

**Architecture:** Create one small frontend domain type module for capture-related IPC payloads and geometry. Keep `ScreenshotSession` as a consumer of those shapes, not their owner. Replace the stale `src/types/index.ts` bucket with the existing focused type homes rather than expanding it.

**Tech Stack:** TypeScript, React, Vite, Vitest.

---

## Scope

In scope:
- Move shared `LogicalRect`, `Point`, `CaptureSessionView`, `CaptureMode`, `OcrResult`, `PinnedImageView`, and annotation command shapes into a focused frontend domain module.
- Update Frontend Tauri Adapter modules and window modules to import shared IPC shapes from the domain module.
- Keep Screenshot Workspace-specific state and runtime modules in `src/components/ScreenshotSession`.
- Remove or empty the stale `src/types/index.ts` bucket if no imports remain.

Out of scope:
- No backend Rust type changes.
- No IPC command rename or payload shape change.
- No Capture Workspace controller split.
- No Pinned Image runtime refactor.
- No Tesseract adapter move.

## File Structure

- Create: `src/domain/capture.ts`  
  Owns frontend capture-related IPC/domain shapes and `CAPTURE_MODES`.
- Create: `src/domain/capture.test.ts`  
  Verifies shared Capture mode values use current IPC strings.
- Modify: `src/components/ScreenshotSession/types.ts`  
  Re-export capture domain shapes for a narrow compatibility step, or remove after imports are migrated.
- Modify: `src/components/ScreenshotSession/windowMode.ts`  
  Use `CAPTURE_MODES` from the domain module instead of a hand-coded predicate.
- Modify: `src/tauri/captureSession.ts`, `src/tauri/ocr.ts`, `src/tauri/pinnedImage.ts`  
  Import shared types from `src/domain/capture`.
- Modify: `src/components/PinnedImageWindow/*`, `src/components/ResultWindow/ocrFileWorkflow.ts`  
  Import shared cross-window types from `src/domain/capture`.
- Delete or reduce: `src/types/index.ts`  
  Remove stale Capture-related definitions if no current imports depend on them.

## Task 1: Add Capture Domain Type Test

**Files:**
- Create: `src/domain/capture.test.ts`

- [x] **Step 1: Write the failing test**

```ts
import { describe, expect, it } from 'vitest';
import { CAPTURE_MODES } from './capture';

describe('capture domain types', () => {
  it('uses backend IPC capture mode strings', () => {
    expect(CAPTURE_MODES).toEqual([
      'screenshot',
      'screenshot-copy',
      'screenshot-ocr',
      'silent-screenshot-ocr',
      'screenshot-translate',
    ]);
    expect(CAPTURE_MODES).not.toContain('Screenshot');
    expect(CAPTURE_MODES).not.toContain('OcrTranslate');
  });
});
```

- [x] **Step 2: Run test to verify it fails**

Run: `npm test -- src/domain/capture.test.ts`

Expected: FAIL because `src/domain/capture.ts` does not exist.

## Task 2: Create Capture Domain Type Module

**Files:**
- Create: `src/domain/capture.ts`
- Modify: `src/components/ScreenshotSession/types.ts`

- [x] **Step 1: Implement the minimal module**

Move shared capture/OCR/pinned shapes from `src/components/ScreenshotSession/types.ts` into `src/domain/capture.ts`. Export `CAPTURE_MODES` as a readonly tuple and derive `CaptureMode` from it.

- [x] **Step 2: Keep compatibility**

For this narrow migration, make `src/components/ScreenshotSession/types.ts` re-export from `../../domain/capture` so existing internal ScreenshotSession imports keep working until explicitly migrated.

- [x] **Step 3: Run domain test**

Run: `npm test -- src/domain/capture.test.ts`

Expected: PASS.

## Task 3: Update Adapter and Cross-Window Imports

**Files:**
- Modify: `src/tauri/captureSession.ts`
- Modify: `src/tauri/ocr.ts`
- Modify: `src/tauri/pinnedImage.ts`
- Modify: `src/tauri/__tests__/captureSession.test.ts`
- Modify: `src/components/PinnedImageWindow/index.tsx`
- Modify: `src/components/PinnedImageWindow/pinActions.ts`
- Modify: `src/components/PinnedImageWindow/pinActions.test.ts`
- Modify: `src/components/ResultWindow/ocrFileWorkflow.ts`

- [x] **Step 1: Replace imports**

Change cross-window and adapter imports from `components/ScreenshotSession/types` to `domain/capture`.

- [x] **Step 2: Verify no adapter imports UI types**

Run: `rg -n "components/ScreenshotSession/types|../ScreenshotSession/types|../../components/ScreenshotSession/types" src/tauri src/components/PinnedImageWindow src/components/ResultWindow`

Expected: No output for adapter/cross-window modules.

## Task 4: Use Shared Capture Modes in Window Routing

**Files:**
- Modify: `src/components/ScreenshotSession/windowMode.ts`
- Test: `src/components/ScreenshotSession/windowMode.test.ts`

- [x] **Step 1: Use `CAPTURE_MODES`**

Replace the hard-coded `isCaptureMode` chain with `CAPTURE_MODES.includes(...)`.

- [x] **Step 2: Run focused tests**

Run: `npm test -- src/domain/capture.test.ts src/components/ScreenshotSession/windowMode.test.ts src/tauri/__tests__/captureSession.test.ts`

Expected: PASS.

## Task 5: Remove Stale Type Bucket If Unused

**Files:**
- Delete or modify: `src/types/index.ts`
- Verify imports under `src/`

- [x] **Step 1: Check current imports**

Run: `rg -n "from ['\"]\\.\\.?/types|from ['\"].*/types" src`

Expected: only intentional local type imports remain, plus `src/types` imports if still current.

- [x] **Step 2: Remove stale Capture definitions**

If `src/types/index.ts` still contains only stale shapes or is unused, delete it. If current translation/history types remain in use, leave only those current exports.

- [x] **Step 3: Run TypeScript build**

Run: `npm run build`

Expected: PASS.

## Task 6: Final Verification

**Files:** none.

- [x] **Step 1: Run targeted frontend tests**

Run: `npm test -- src/domain/capture.test.ts src/components/ScreenshotSession/windowMode.test.ts src/tauri/__tests__/captureSession.test.ts src/components/PinnedImageWindow/pinActions.test.ts`

Expected: PASS.

- [x] **Step 2: Inspect diff**

Run: `git diff -- src/domain src/components/ScreenshotSession src/tauri src/components/PinnedImageWindow src/components/ResultWindow src/types docs/superpowers/plans/2026-07-10-capture-ipc-types.md`

Expected: diff only covers the plan and Capture IPC type locality migration.
