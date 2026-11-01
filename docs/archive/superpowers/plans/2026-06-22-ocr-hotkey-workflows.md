# OCR Hotkey Workflows Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement OCR `silent-screenshot-ocr`, `file-ocr`, and `show-window` hotkey workflows.

**Architecture:** Keep global shortcut registration shallow and dispatch OCR workflows through existing frontend/backend seams. Reuse Capture Session for screenshot OCR, reuse `recognize_image`/`OcrCoordinator` for file OCR, and make `ResultWindow` explicitly support `translation` and `ocr` modes.

**Tech Stack:** React, Zustand, Tauri v2, Vitest, Rust, Cargo tests.

---

## Files

- Modify: `src/stores/appStore.ts` for result window mode and OCR status state.
- Modify: `src/components/ResultWindow/ResultWindow.tsx` for translation/OCR mode rendering.
- Create: `src/components/ResultWindow/ocrFileWorkflow.ts` for image file OCR helper logic.
- Test: `src/components/ResultWindow/ocrFileWorkflow.test.ts`.
- Modify: `src/App.tsx` for OCR hotkey events and OCR mode opening.
- Modify: `src/tauri/captureSession.ts` only if result-window adapter names need clarification.
- Modify: `src/tauri/ocr.ts` if a typed adapter for `recognize_image` does not already exist.
- Modify: `src/components/ScreenshotSession/types.ts`, `captureActions.ts`, `captureInteractionModel.ts`, and `index.tsx` for `silent-screenshot-ocr`.
- Modify tests: `src/components/ScreenshotSession/captureActions.test.ts`, `captureInteractionModel.test.ts`.
- Modify: `src-tauri/src/startup_shortcuts.rs` to enable and dispatch OCR actions.
- Modify: `src-tauri/src/commands/mod.rs` for OCR result/window events if needed.
- Modify: `src-tauri/src/lib.rs` and `src-tauri/capabilities/default.json` only if Tauri dialog/fs plugins are introduced.

## Task 1: Result Window OCR Mode

- [ ] Add failing tests for pure OCR file workflow helper behavior.
- [ ] Run `npm test -- src/components/ResultWindow/ocrFileWorkflow.test.ts` and confirm failure.
- [ ] Extend `appStore` with `resultWindowMode: 'translation' | 'ocr'`, `ocrText`, `isOcrRunning`, and `ocrError`.
- [ ] Add store actions: `showTranslationWindow`, `showOcrWindow`, `setOcrText`, `setOcrRunning`, `setOcrError`.
- [ ] Split `ResultWindow` rendering by mode while preserving current translation behavior.
- [ ] In OCR mode, show upload button, status, error, and recognized text.
- [ ] Run the focused frontend tests.
- [ ] Checkpoint/self-review: no translation behavior regression; OCR mode is explicit.

## Task 2: File OCR Workflow

- [ ] Add failing tests for file OCR helper: cancel is no-op, selected image calls recognizer, errors update state.
- [ ] Run focused test and confirm failure.
- [ ] Add typed frontend adapter for `recognize_image`.
- [ ] Add `@tauri-apps/plugin-dialog` and `@tauri-apps/plugin-fs` if needed for Tauri v2 file selection/read.
- [ ] Register plugins and capabilities if the Tauri APIs require explicit permissions.
- [ ] Wire OCR upload button to file selection, file read, `recognize_image`, and OCR state updates.
- [ ] Run focused frontend tests and `npm run build`.
- [ ] Checkpoint/self-review: file picker cancel does not show an error; file read/OCR failures are visible.

## Task 3: Silent Screenshot OCR

- [ ] Add failing tests for `silent-screenshot-ocr` mode mapping to OCR with no result window and session finish.
- [ ] Run focused capture tests and confirm failure.
- [ ] Extend `CaptureMode` and `isCaptureMode` with `silent-screenshot-ocr`.
- [ ] Map `silent-screenshot-ocr` to a silent OCR selection flow/completion plan.
- [ ] Add a frontend Tauri command adapter or backend command for copying recognized text to clipboard.
- [ ] In capture completion, run OCR, copy text, finish session, and do not open result window.
- [ ] Run focused capture tests.
- [ ] Checkpoint/self-review: no visible OCR window opens for silent flow; capture session cleanup still runs.

## Task 4: Hotkey Dispatch

- [ ] Add/update Rust unit tests around implemented OCR actions and rejected unknown actions.
- [ ] Run the focused Rust test and confirm failure if coverage is missing.
- [ ] Mark all three OCR actions implemented in `startup_shortcuts.rs`.
- [ ] Dispatch `silent-screenshot-ocr` to `open_capture_window_from_shortcut(app, "silent-screenshot-ocr")`.
- [ ] Dispatch `file-ocr` to a main-window event that opens OCR mode and starts file OCR.
- [ ] Dispatch `show-window` to a main-window event that opens OCR mode without automatically selecting a file.
- [ ] Wire `App.tsx` listeners for the OCR events.
- [ ] Run focused Rust and frontend tests.
- [ ] Checkpoint/self-review: configured non-empty OCR hotkeys no longer fail registration.

## Task 5: Full Verification and Commit

- [ ] Run `npm test`.
- [ ] Run `npm run build`.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml --lib`.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml --tests`.
- [ ] Review `git diff --stat` and changed files for unrelated edits.
- [ ] Commit with `feat(ocr): implement OCR hotkey workflows`.
