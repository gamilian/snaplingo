# Cross-Platform Architecture Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move platform-specific code into infrastructure seams, remove obsolete screenshot workflow code, and make frontend native calls easier to isolate for cross-platform work.

**Architecture:** Application modules keep provider/coordinator interfaces. Platform implementations live under infrastructure modules and are selected by composition. Frontend UI modules should call focused adapter modules instead of learning Tauri or browser-native details directly.

**Tech Stack:** Rust/Tauri backend, React/TypeScript frontend, Vitest, Cargo tests.

---

## File Structure

- Create `src-tauri/src/infrastructure/system/ocr/mod.rs`: exposes a platform OCR engine factory.
- Create `src-tauri/src/infrastructure/system/ocr/backend.rs`: defines the OCR engine interface used by the provider adapter.
- Create `src-tauri/src/infrastructure/system/ocr/macos.rs`: owns macOS Vision and objc2 implementation details.
- Modify `src-tauri/src/infrastructure/system/mod.rs`: exports the OCR infrastructure module.
- Modify `src-tauri/src/application/providers/ocr/impls/system_ocr.rs`: keep only the application provider adapter and delegate OCR work to infrastructure.
- Modify `src-tauri/src/application/providers/ocr/impls/mod.rs`: continue exposing `SystemOcrProvider` only on macOS.
- Modify `src-tauri/src/composition/provider_runtime.rs`: keep provider registration behavior unchanged.
- Delete `src/components/ScreenshotWorkflow/index.tsx`, `src/components/ScreenshotCapture/index.tsx`, and `src/components/ScreenshotEditor/index.tsx` if no runtime imports remain.
- Create or extend frontend adapter modules under `src/tauri/` for window, event, and clipboard seams after the backend cleanup.

## Task 1: Move System OCR platform implementation behind infrastructure seam

**Files:**
- Create: `src-tauri/src/infrastructure/system/ocr/backend.rs`
- Create: `src-tauri/src/infrastructure/system/ocr/macos.rs`
- Create: `src-tauri/src/infrastructure/system/ocr/mod.rs`
- Modify: `src-tauri/src/infrastructure/system/mod.rs`
- Modify: `src-tauri/src/application/providers/ocr/impls/system_ocr.rs`
- Test: existing provider tests and composition tests

- [x] **Step 1: Write/adjust tests for the provider adapter**

Keep existing behavior checks:

```rust
#[test]
fn system_ocr_provider_is_local_and_ready_without_credentials() {
    let provider = SystemOcrProvider::new();

    assert_eq!(provider.id(), "system-ocr");
    assert_eq!(provider.name(), "System OCR");
    assert!(provider.is_configured());
    assert!(!provider.requires_api_key());
}
```

- [x] **Step 2: Run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml application::providers::ocr::impls::system_ocr
```

Expected: pass before and after, proving behavior is preserved.

- [x] **Step 3: Introduce the infrastructure OCR engine interface**

Add a small interface in `backend.rs`:

```rust
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::Result;

pub trait SystemOcrEngine: Send + Sync {
    fn recognize(&self, request: &OcrRequest) -> Result<OcrResult>;
}
```

- [x] **Step 4: Move macOS Vision implementation**

Move Vision and objc2 imports/functions from the provider adapter into `macos.rs`. The macOS module owns language mapping, observation conversion, and Vision request execution.

- [x] **Step 5: Make `SystemOcrProvider` delegate**

`SystemOcrProvider` should keep provider identity and `OcrProvider` implementation, but delegate recognition to `get_system_ocr_engine()`.

- [x] **Step 6: Run verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
npm test
```

Expected: all pass.

## Task 2: Delete legacy screenshot workflow path

**Files:**
- Delete: `src/components/ScreenshotWorkflow/index.tsx`
- Delete: `src/components/ScreenshotCapture/index.tsx`
- Delete: `src/components/ScreenshotEditor/index.tsx`

- [x] **Step 1: Confirm no imports remain**

Run:

```bash
rg -n "ScreenshotWorkflow|ScreenshotCapture|ScreenshotEditor" src
```

Expected: only deletion candidates appear, plus settings editor page alias that is unrelated.

- [x] **Step 2: Delete unused modules**

Remove the obsolete runtime files. Do not touch the Settings screenshot editor page.

- [x] **Step 3: Verify frontend**

Run:

```bash
npm run build
npm test
```

Expected: all pass.

## Task 3: Deepen frontend native adapter modules

**Files:**
- Create or modify: `src/tauri/window.ts`
- Create or modify: `src/tauri/events.ts`
- Create or modify: `src/tauri/clipboard.ts`
- Modify: `src/components/ResultWindow/ResultWindow.tsx`
- Modify: `src/components/ScreenshotSession/index.tsx`
- Modify: `src/components/PinnedImageWindow/index.tsx`
- Modify: `src/App.tsx`

- [x] **Step 1: Add frontend adapter tests where behavior can be isolated**

Test clipboard and event wrapper behavior with Vitest mocks for `@tauri-apps/api/*`.

- [x] **Step 2: Move direct clipboard writes to `src/tauri/clipboard.ts`**

Expose one function:

```ts
export function writeClipboardText(text: string): Promise<void> {
  return navigator.clipboard.writeText(text);
}
```

- [x] **Step 3: Move direct window/event calls behind adapter modules**

Keep the interface small. Avoid wrapping every Tauri method unless a UI module uses it now.

- [x] **Step 4: Replace UI imports**

UI modules should import from `src/tauri/*`, not directly from `@tauri-apps/api/*`, except adapter modules and tests.

- [x] **Step 5: Verify adapter seam**

Run:

```bash
rg -n "@tauri-apps/api|navigator\\.clipboard" src
npm run build
npm test
```

Expected: direct native imports remain only in `src/tauri/*`, tests, or explicitly documented exceptions.

## Task 4: Architecture residue check

**Files:**
- Inspect: `src-tauri/src/application/providers/ocr/`
- Inspect: `src-tauri/src/infrastructure/system/`
- Inspect: `src/tauri/`
- Inspect: `src/components/ResultWindow/`

- [x] **Step 1: Run architecture residue searches**

Run:

```bash
rg -n "objc2|objc2_vision|VNRecognizeTextRequest" src-tauri/src/application
rg -n "@tauri-apps/api|navigator\\.clipboard" src/components src/App.tsx
rg -n "ScreenshotWorkflow|ScreenshotCapture|ScreenshotEditor" src
```

Expected: no platform OCR implementation in application; no obsolete screenshot workflow; no avoidable native calls in UI modules.

- [x] **Step 2: Run full verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
npm test
git diff --check
```

Expected: all pass.

## Notes

- Do not extract a full OCR Result Window module in this pass unless the current edits make `ResultWindow.tsx` materially worse.
- Do not move network OCR providers into infrastructure; Baidu and Tesseract provider code are provider adapters, not platform system adapters.
- Preserve the current OCR UI changes and do not revert user-facing spacing, tooltip, scrollbar, or language selector work.
