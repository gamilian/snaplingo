# SnapLingo Architecture Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve SnapLingo architecture navigability by making the frontend/backend seam explicit, deepening Capture Session and Provider configuration modules, and moving application composition out of `lib.rs`.

**Architecture:** Keep the Tauri-standard top-level names `src/` and `src-tauri/`. Add explicit frontend Tauri adapters so UI modules stop calling raw command strings, then deepen the backend modules whose interfaces currently leak ordering and configuration details across callers. Preserve current behavior first, then change module shape behind tests.

**Tech Stack:** Tauri 2, Rust 2021, React 18, TypeScript, Zustand, Vitest, Cargo tests.

---

## Scope Check

This plan covers several related architecture improvements, but each task is independently testable. Execute in order.

Recommended first implementation wave:

1. Task 1: Baseline and stale-file audit.
2. Task 2: Frontend Tauri adapter seam.
3. Task 3: Capture Session frontend adapter.
4. Task 4: Capture Session backend runtime module.

Defer Tasks 5-7 if the first wave exposes behavior risk. Do not rename `src/` or `src-tauri/`; those names are Tauri ecosystem convention.

## File Structure

### Files to Create

- `src/tauri/translation.ts`  
  Frontend adapter for translation commands.

- `src/tauri/providers.ts`  
  Frontend adapter for provider listing, activation, credentials, custom translation providers, and ordering.

- `src/tauri/history.ts`  
  Frontend adapter for history commands.

- `src/tauri/captureSession.ts`  
  Frontend adapter for Capture Session commands.

- `src/tauri/pinnedImage.ts`  
  Frontend adapter for pinned image commands.

- `src/tauri/__tests__/translation.test.ts`  
  Tests command-name and payload mapping for translation.

- `src/tauri/__tests__/providers.test.ts`  
  Tests command-name and payload mapping for provider operations.

- `src/tauri/__tests__/captureSession.test.ts`  
  Tests Capture Session command mapping and optional parameter handling.

- `src-tauri/src/application/services/capture_session_runtime.rs`  
  Deep module that coordinates Capture Session rendering, OCR, output, and output outcome decisions.

- `src-tauri/src/application/providers/configuration.rs`  
  Module for Provider configuration lifecycle: credential validation, storage, runtime reconfiguration, and custom provider definitions.

- `src-tauri/src/composition.rs`  
  Application composition module for AppState construction and startup wiring.

- `docs/architecture/runtime-map.md`  
  Human-readable map of frontend runtime, backend runtime, command seam, and current deep modules.

### Files to Modify

- `src/hooks/useTranslate.ts`  
  Replace raw `invoke` usage with `src/tauri/translation.ts`.

- `src/stores/providerStore.ts`  
  Replace raw `invoke` usage with `src/tauri/providers.ts`.

- `src/stores/historyStore.ts`  
  Replace raw `invoke` usage with `src/tauri/history.ts`.

- `src/components/ScreenshotSession/index.tsx`  
  Replace raw Capture Session command calls with `src/tauri/captureSession.ts`.

- `src/components/ScreenshotSession/captureActions.ts`  
  Replace raw Capture Session command calls with `src/tauri/captureSession.ts`.

- `src/components/ScreenshotSession/captureSessionLifecycle.ts`  
  Replace raw Capture Session command calls with `src/tauri/captureSession.ts`.

- `src/components/ScreenshotSession/captureWindowVisibility.ts`  
  Replace raw Capture Session command calls with `src/tauri/captureSession.ts`.

- `src/components/PinnedImageWindow/index.tsx`  
  Replace raw pinned-image command calls with `src/tauri/pinnedImage.ts`.

- `src/components/PinnedImageWindow/pinActions.ts`  
  Replace raw pinned-image command calls with `src/tauri/pinnedImage.ts`.

- `src-tauri/src/application/services/mod.rs`  
  Export `capture_session_runtime`.

- `src-tauri/src/application/mod.rs`  
  Export `CaptureSessionRuntime`.

- `src-tauri/src/commands/capture_session_commands.rs`  
  Call `CaptureSessionRuntime` instead of manually passing several services.

- `src-tauri/src/application/providers/ocr/coordinator.rs`  
  Support runtime Provider reconfiguration consistently with Translation.

- `src-tauri/src/application/providers/ocr/coordinator_test.rs`  
  Add reconfiguration tests.

- `src-tauri/src/lib.rs`  
  Move AppState construction helpers into `composition.rs`.

- `src-tauri/src/application/providers/mod.rs`  
  Export Provider configuration module.

- `CONTEXT.md`  
  Update domain vocabulary if new stable module names are introduced.

### Files to Audit Before Deleting or Moving

- `src-tauri/src/history.rs`
- `src-tauri/src/utils.rs`
- `src/App.PROTOTYPE.tsx`
- `src/App.PROTOTYPE_UI.tsx`
- `src/App.UIDOC.tsx`
- `public/screenshot.html`

Do not delete untracked files without explicit confirmation. Current untracked files observed during planning:

- `screenshot.html`
- `screenshot-test.html`
- `script/check-screenshot-flow.mjs`
- `src-tauri/src/lib.rs.bak`

---

## Task 1: Baseline and Stale-File Audit

**Files:**
- Create: `docs/architecture/runtime-map.md`
- Possibly remove or relocate after audit: `src-tauri/src/history.rs`
- Possibly remove or relocate after audit: `src-tauri/src/utils.rs`
- Possibly relocate after audit: `src/App.PROTOTYPE.tsx`
- Possibly relocate after audit: `src/App.PROTOTYPE_UI.tsx`
- Possibly relocate after audit: `src/App.UIDOC.tsx`
- Possibly relocate after audit: `public/screenshot.html`

- [x] **Step 1: Record current test baseline**

Run:

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: record PASS/FAIL for each command in the task notes. Do not fix unrelated failures in this task.

Current status: PASS. `npm test` passed 228 tests in 20 files. `npm run build` passed. `cargo test --manifest-path src-tauri/Cargo.toml --lib` passed 191 tests with pre-existing warnings.

- [x] **Step 2: Verify stale Rust files are not compiled**

Run:

```bash
rg "mod history|mod utils|HistoryManager|detect_language" src-tauri/src src-tauri/Cargo.toml Cargo.toml -n
```

Expected: no `mod history` or `mod utils` references. `HistoryManager` and `detect_language` should only appear in the stale files themselves.

Current status: confirmed. `src-tauri/src/history.rs` and `src-tauri/src/utils.rs` were not compiled by the Rust crate and were removed.

- [x] **Step 3: Verify prototype files are not imported**

Run:

```bash
rg "App\\.PROTOTYPE|App\\.UIDOC|public/screenshot\\.html" src index.html public docs -n
```

Expected: either no references, or references clearly inside documentation/prototype instructions.

Current status: confirmed. Prototype references were documentation-only. Tracked prototypes moved from `src/` to `designs/prototypes/`, and ADR/PRD references were updated.

- [x] **Step 4: Create runtime map doc**

Create `docs/architecture/runtime-map.md`:

```markdown
# SnapLingo Runtime Map

## Frontend Runtime

`src/` is the React/Vite frontend. Window modules render Settings Window, Capture Window, Result Window, and Pinned Image Window.

## Backend Runtime

`src-tauri/` is the Tauri/Rust backend runtime. `src-tauri/src/commands/` is the frontend-facing adapter seam. `application/` owns workflow modules, `domain/` owns shared domain types, and `infrastructure/` owns OS, storage, HTTP, window, and event adapters.

## Frontend/Backend Seam

Frontend code calls backend behavior only through `src/tauri/*` adapters. Those adapters call Tauri commands declared under `src-tauri/src/commands/`.

## Deep Modules

- Provider Coordinators: provider activation, persistence, and execution.
- Capture Session: frozen desktop, selection rendering, output, and OCR handoff.
- Pinned Image: in-memory pinned image state and window adapter behavior.
```

- [x] **Step 5: Remove or relocate confirmed stale tracked files**

If Steps 2-3 show the tracked files are not part of runtime, choose one path:

- Delete stale implementation files that fail the deletion test: `src-tauri/src/history.rs`, `src-tauri/src/utils.rs`.
- Move prototype app files to `designs/` if they are still useful as design artifacts.

Run:

```bash
git status --short
```

Expected: only the audited files and `docs/architecture/runtime-map.md` changed.

- [x] **Step 6: Re-run baseline commands**

Run:

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: same PASS/FAIL status as Step 1. No new failures.

Current status: PASS. `npm test` passed 228 tests in 20 files. `npm run build` passed. `cargo test --manifest-path src-tauri/Cargo.toml --lib` passed 191 tests with the same warning class as baseline.

- [x] **Step 7: Commit**

```bash
git add docs/architecture/runtime-map.md src-tauri/src/history.rs src-tauri/src/utils.rs src/App.PROTOTYPE.tsx src/App.PROTOTYPE_UI.tsx src/App.UIDOC.tsx public/screenshot.html
git commit -m "chore(architecture): document runtime map and remove stale shadows"
```

---

## Task 2: Add Frontend Tauri Adapters for Translation, Providers, and History

**Files:**
- Create: `src/tauri/translation.ts`
- Create: `src/tauri/providers.ts`
- Create: `src/tauri/history.ts`
- Create: `src/tauri/__tests__/translation.test.ts`
- Create: `src/tauri/__tests__/providers.test.ts`
- Modify: `src/hooks/useTranslate.ts`
- Modify: `src/stores/providerStore.ts`
- Modify: `src/stores/historyStore.ts`

- [ ] **Step 1: Add failing translation adapter test**

Create `src/tauri/__tests__/translation.test.ts`:

```ts
import { describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('translation tauri adapter', () => {
  it('maps auto source language to null for translate_text_v2', async () => {
    const { translateText } = await import('../translation');
    invoke.mockResolvedValueOnce([{ provider_id: 'google', translated_text: '你好' }]);

    await translateText({ text: 'hello', sourceLang: 'auto', targetLang: 'zh-CN' });

    expect(invoke).toHaveBeenCalledWith('translate_text_v2', {
      request: {
        text: 'hello',
        source_lang: null,
        target_lang: 'zh-CN',
      },
    });
  });
});
```

- [ ] **Step 2: Run test and verify it fails**

Run:

```bash
npm test -- --run src/tauri/__tests__/translation.test.ts
```

Expected: FAIL because `src/tauri/translation.ts` does not exist.

- [ ] **Step 3: Implement translation adapter**

Create `src/tauri/translation.ts`:

```ts
import { invoke } from '@tauri-apps/api/core';
import type { TranslationResult } from '../types';

export interface TranslateTextInput {
  text: string;
  sourceLang: string;
  targetLang: string;
}

export async function translateText(input: TranslateTextInput) {
  return invoke<TranslationResult[]>('translate_text_v2', {
    request: {
      text: input.text,
      source_lang: input.sourceLang === 'auto' ? null : input.sourceLang,
      target_lang: input.targetLang,
    },
  });
}
```

- [ ] **Step 4: Replace `useTranslate` raw invoke**

In `src/hooks/useTranslate.ts`, remove the direct `invoke` import and call:

```ts
const results = await translateText({
  text: textToTranslate,
  sourceLang: fromLang,
  targetLang: toLang,
});
```

- [ ] **Step 5: Add Provider adapter tests**

Create `src/tauri/__tests__/providers.test.ts` with at least these tests:

```ts
import { describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('providers tauri adapter', () => {
  it('activates translation provider with backend parameter name', async () => {
    const { activateTranslationProvider } = await import('../providers');
    invoke.mockResolvedValueOnce(undefined);

    await activateTranslationProvider('deepl');

    expect(invoke).toHaveBeenCalledWith('activate_translation_provider', {
      providerId: 'deepl',
    });
  });

  it('saves translation credentials as a credentials map', async () => {
    const { configureTranslationProviderCredentials } = await import('../providers');
    invoke.mockResolvedValueOnce(undefined);

    await configureTranslationProviderCredentials('baidu-translate', {
      app_id: 'app',
      secret_key: 'secret',
    });

    expect(invoke).toHaveBeenCalledWith('configure_translation_provider_credentials', {
      providerId: 'baidu-translate',
      credentials: { app_id: 'app', secret_key: 'secret' },
    });
  });
});
```

- [ ] **Step 6: Implement Provider and History adapters**

Create `src/tauri/providers.ts` with functions matching current backend commands:

- `listTranslationProviders`
- `activateTranslationProvider`
- `deactivateTranslationProvider`
- `reorderActiveTranslationProviders`
- `getProviderCredentialSchema`
- `configureTranslationProvider`
- `configureTranslationProviderCredentials`
- `addCustomTranslationProvider`
- `removeCustomTranslationProvider`
- `listOcrProviders`
- `activateOcrProvider`
- `configureOcrProvider`

Create `src/tauri/history.ts` with functions matching current backend commands:

- `getTranslationHistory`
- `getOcrHistory`
- `searchHistory`
- `deleteHistory`
- `clearAllHistory`

- [ ] **Step 7: Replace store raw invokes**

Modify `src/stores/providerStore.ts` and `src/stores/historyStore.ts` so they import from `src/tauri/providers.ts` and `src/tauri/history.ts`.

Expected: no direct `invoke(` remains in these two store files.

- [ ] **Step 8: Run frontend tests and build**

Run:

```bash
npm test -- --run src/tauri/__tests__/translation.test.ts src/tauri/__tests__/providers.test.ts src/components/ResultWindow/translationInput.test.ts
npm run build
```

Expected: PASS.

- [ ] **Step 9: Inspect direct invoke usage**

Run:

```bash
rg "invoke<|invoke\\(" src -n
```

Expected: remaining direct invocations only in Capture Session, Pinned Image, legacy prototypes, or files scheduled for later tasks.

- [ ] **Step 10: Commit**

```bash
git add src/tauri src/hooks/useTranslate.ts src/stores/providerStore.ts src/stores/historyStore.ts
git commit -m "refactor(frontend): add typed tauri adapters for providers and history"
```

---

## Task 3: Add Frontend Tauri Adapters for Capture Session and Pinned Image

**Files:**
- Create: `src/tauri/captureSession.ts`
- Create: `src/tauri/pinnedImage.ts`
- Create: `src/tauri/__tests__/captureSession.test.ts`
- Modify: `src/components/ScreenshotSession/index.tsx`
- Modify: `src/components/ScreenshotSession/captureActions.ts`
- Modify: `src/components/ScreenshotSession/captureSessionLifecycle.ts`
- Modify: `src/components/ScreenshotSession/captureWindowVisibility.ts`
- Modify: `src/components/PinnedImageWindow/index.tsx`
- Modify: `src/components/PinnedImageWindow/pinActions.ts`

- [ ] **Step 1: Add failing Capture Session adapter tests**

Create `src/tauri/__tests__/captureSession.test.ts`:

```ts
import { describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('capture session tauri adapter', () => {
  it('omits includeCursor when false', async () => {
    const { renderCaptureOutput } = await import('../captureSession');
    invoke.mockResolvedValueOnce('base64');

    await renderCaptureOutput({
      sessionId: 'capture-1',
      rect: { x: 0, y: 0, width: 10, height: 10 },
      annotations: [],
      includeCursor: false,
    });

    expect(invoke).toHaveBeenCalledWith('render_capture_output', {
      sessionId: 'capture-1',
      rect: { x: 0, y: 0, width: 10, height: 10 },
      annotations: [],
    });
  });

  it('passes pin action to output_capture', async () => {
    const { outputCapture } = await import('../captureSession');
    invoke.mockResolvedValueOnce(undefined);

    await outputCapture({
      sessionId: 'capture-1',
      rect: { x: 1, y: 2, width: 3, height: 4 },
      annotations: [],
      includeCursor: true,
      action: { type: 'pin' },
    });

    expect(invoke).toHaveBeenCalledWith('output_capture', {
      sessionId: 'capture-1',
      rect: { x: 1, y: 2, width: 3, height: 4 },
      annotations: [],
      includeCursor: true,
      action: { type: 'pin' },
    });
  });
});
```

- [ ] **Step 2: Run test and verify it fails**

Run:

```bash
npm test -- --run src/tauri/__tests__/captureSession.test.ts
```

Expected: FAIL because `src/tauri/captureSession.ts` does not exist.

- [ ] **Step 3: Implement Capture Session adapter**

Create `src/tauri/captureSession.ts` with functions:

- `openCaptureWindow`
- `createCaptureSession`
- `getCaptureSession`
- `cancelCaptureSession`
- `restoreCaptureSnapshotWindowsForSession`
- `renderCaptureOutput`
- `defaultCaptureSavePath`
- `quickCaptureSavePath`
- `outputCapture`
- `runCaptureOcr`

Use existing frontend types from `src/components/ScreenshotSession/types.ts` at first. Do not create new shared type systems in this task.

- [ ] **Step 4: Implement Pinned Image adapter**

Create `src/tauri/pinnedImage.ts` with functions:

- `getPinnedImage`
- `copyPinnedImage`
- `replacePinnedImageFromClipboard`
- `savePinnedImage`
- `closePinnedImage`
- `removePinnedImage`
- `togglePinnedImagesVisibility`
- `switchPinnedImageGroup`
- `movePinnedImageToNextGroup`
- `hidePinnedImageGroup`
- `destroyPinnedImageGroup`

- [ ] **Step 5: Replace raw invokes in Capture Session files**

Replace direct calls in:

- `src/components/ScreenshotSession/index.tsx`
- `src/components/ScreenshotSession/captureActions.ts`
- `src/components/ScreenshotSession/captureSessionLifecycle.ts`
- `src/components/ScreenshotSession/captureWindowVisibility.ts`

Expected: these files import adapter functions instead of `invoke` from `@tauri-apps/api/core`.

- [ ] **Step 6: Replace raw invokes in Pinned Image files**

Replace direct calls in:

- `src/components/PinnedImageWindow/index.tsx`
- `src/components/PinnedImageWindow/pinActions.ts`

Expected: these files import adapter functions from `src/tauri/pinnedImage.ts`.

- [ ] **Step 7: Run focused frontend tests**

Run:

```bash
npm test -- --run \
  src/tauri/__tests__/captureSession.test.ts \
  src/components/ScreenshotSession/captureActions.test.ts \
  src/components/ScreenshotSession/captureSessionLifecycle.test.ts \
  src/components/ScreenshotSession/captureWindowVisibility.test.ts \
  src/components/PinnedImageWindow/pinActions.test.ts
```

Expected: PASS.

- [ ] **Step 8: Verify command seam is explicit**

Run:

```bash
rg "invoke<|invoke\\(" src -n
```

Expected: direct runtime command calls are concentrated under `src/tauri/`. Direct invokes in prototype files are acceptable only if those files were intentionally kept as design artifacts.

- [ ] **Step 9: Commit**

```bash
git add src/tauri src/components/ScreenshotSession src/components/PinnedImageWindow
git commit -m "refactor(frontend): centralize capture and pin tauri adapters"
```

---

## Task 4: Add Backend Capture Session Runtime Module

**Files:**
- Create: `src-tauri/src/application/services/capture_session_runtime.rs`
- Modify: `src-tauri/src/application/services/mod.rs`
- Modify: `src-tauri/src/application/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands/capture_session_commands.rs`
- Test: `src-tauri/src/application/services/capture_session_service_test.rs`

- [ ] **Step 1: Add failing runtime test**

Add a focused test to `src-tauri/src/application/services/capture_session_service_test.rs` or create `capture_session_runtime_test.rs` if the existing file becomes too large:

```rust
#[tokio::test]
async fn runtime_recognizes_selection_text_through_one_interface() {
    // Arrange a CaptureSessionService with a frozen session and a mock OCR coordinator/provider.
    // Use existing test fixtures from capture_session_service_test where possible.
    //
    // Act:
    // let result = runtime.recognize_selection_text(&session_id, &selection_rect).await.unwrap();
    //
    // Assert:
    // assert_eq!(result.text, "recognized");
    //
    // This test should initially fail because CaptureSessionRuntime does not exist.
}
```

Expected failure: missing `CaptureSessionRuntime`.

- [ ] **Step 2: Run test and verify it fails**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_session_runtime --lib
```

Expected: FAIL because the runtime module does not exist.

- [ ] **Step 3: Implement runtime module**

Create `src-tauri/src/application/services/capture_session_runtime.rs`:

```rust
use std::sync::Arc;

use crate::application::providers::ocr::OcrCoordinator;
use crate::application::services::{
    CaptureOutputService, CaptureSessionOutput, CaptureSessionService, ImageCompositionService,
};
use crate::domain::capture::{
    AnnotationCommand, CaptureOutputAction, CaptureSessionId, LogicalRect,
};
use crate::domain::ocr::OcrResult;
use crate::Result;

pub struct CaptureSessionRuntime {
    sessions: Arc<CaptureSessionService>,
    image_composition: Arc<ImageCompositionService>,
    output: Arc<CaptureOutputService>,
    ocr: Arc<OcrCoordinator>,
}

impl CaptureSessionRuntime {
    pub fn new(
        sessions: Arc<CaptureSessionService>,
        image_composition: Arc<ImageCompositionService>,
        output: Arc<CaptureOutputService>,
        ocr: Arc<OcrCoordinator>,
    ) -> Self {
        Self {
            sessions,
            image_composition,
            output,
            ocr,
        }
    }

    pub fn render_png_base64(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        include_cursor: bool,
    ) -> Result<String> {
        self.sessions.render_png_base64(
            &self.image_composition,
            session_id,
            rect,
            annotations,
            include_cursor,
        )
    }

    pub async fn recognize_selection_text(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
    ) -> Result<OcrResult> {
        self.sessions
            .recognize_selection_text(&self.image_composition, &self.ocr, session_id, rect)
            .await
    }

    pub async fn output_selection(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        include_cursor: bool,
        action: CaptureOutputAction,
    ) -> Result<CaptureSessionOutput> {
        self.sessions
            .output_selection(
                &self.image_composition,
                &self.output,
                session_id,
                rect,
                annotations,
                include_cursor,
                action,
            )
            .await
    }
}
```

- [ ] **Step 4: Export runtime module**

Modify `src-tauri/src/application/services/mod.rs`:

```rust
pub mod capture_session_runtime;
pub use capture_session_runtime::CaptureSessionRuntime;
```

Modify `src-tauri/src/application/mod.rs` export list to include `CaptureSessionRuntime`.

- [ ] **Step 5: Add runtime to AppState**

Modify `src-tauri/src/lib.rs`:

- Add `pub capture_session_runtime: Arc<CaptureSessionRuntime>` to `AppState`.
- Construct it after `capture_session_service`, `image_composition_service`, `capture_output_service`, and `ocr_coordinator`.

Expected construction shape:

```rust
let capture_session_runtime = Arc::new(CaptureSessionRuntime::new(
    capture_session_service.clone(),
    image_composition_service.clone(),
    capture_output_service.clone(),
    ocr_coordinator.clone(),
));
```

- [ ] **Step 6: Thin capture session commands**

Modify `src-tauri/src/commands/capture_session_commands.rs`:

- `render_capture_output` calls `state.capture_session_runtime.render_png_base64(...)`.
- `output_capture` calls `state.capture_session_runtime.output_selection(...)`.
- `run_capture_ocr` calls `state.capture_session_runtime.recognize_selection_text(...)`.

Keep pinned image window opening in the command module for now because it is Tauri window adapter behavior.

- [ ] **Step 7: Run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_session --lib
```

Expected: PASS.

- [ ] **Step 8: Inspect command Module shrinkage**

Run:

```bash
git diff -- src-tauri/src/commands/capture_session_commands.rs src-tauri/src/application/services/capture_session_runtime.rs
```

Expected: command module no longer passes `image_composition_service`, `capture_output_service`, and `ocr_coordinator` separately for selection operations.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/application src-tauri/src/commands/capture_session_commands.rs src-tauri/src/lib.rs
git commit -m "refactor(capture): add capture session runtime module"
```

---

## Task 5: Deepen Provider Configuration Lifecycle

**Files:**
- Create: `src-tauri/src/application/providers/configuration.rs`
- Modify: `src-tauri/src/application/providers/mod.rs`
- Modify: `src-tauri/src/commands/provider_commands.rs`
- Modify: `src-tauri/src/commands/ocr_commands.rs`
- Modify: `src-tauri/src/application/providers/ocr/coordinator.rs`
- Modify: `src-tauri/src/application/providers/ocr/coordinator_test.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add failing OCR reconfiguration test**

In `src-tauri/src/application/providers/ocr/coordinator_test.rs`, extend `MockOcrProvider` so it supports `reconfigure_credentials` and records received credentials:

```rust
fn reconfigure_credentials(
    &mut self,
    credentials: &std::collections::HashMap<String, String>,
) -> crate::Result<()> {
    self.text_to_return = credentials
        .get("text_to_return")
        .cloned()
        .unwrap_or_else(|| self.text_to_return.clone());
    Ok(())
}
```

Add test:

```rust
#[test]
fn test_reconfigure_provider_updates_runtime_provider() {
    let config = Arc::new(ConfigFile::new_temp());
    let coordinator = OcrCoordinator::new(config);
    coordinator
        .register(MockOcrProvider::with_text("mock", "Mock OCR", "before"))
        .unwrap();

    let mut credentials = std::collections::HashMap::new();
    credentials.insert("text_to_return".to_string(), "after".to_string());

    coordinator.reconfigure_provider("mock", &credentials).unwrap();

    // Follow-up async recognize assertion may be added if the mock stores mutable text.
}
```

Expected failure: current `OcrCoordinator::reconfigure_provider` returns the fixed restart-only error.

- [ ] **Step 2: Run failing test**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml test_reconfigure_provider_updates_runtime_provider --lib
```

Expected: FAIL with restart-only error.

- [ ] **Step 3: Update OcrCoordinator provider storage**

Modify `src-tauri/src/application/providers/ocr/coordinator.rs` so OCR mirrors Translation's mutable provider seam:

- Store providers as `Arc<RwLock<dyn OcrProvider>>` or another safe mutable adapter.
- `get_active` returns a cloned handle without holding the provider map lock.
- `recognize` reads the provider for the duration of the provider call.
- `reconfigure_provider` gets a write lock and calls `provider.reconfigure_credentials(credentials)`.

Keep the public Coordinator Interface stable unless a test proves a change is needed.

- [ ] **Step 4: Add Provider configuration module shell**

Create `src-tauri/src/application/providers/configuration.rs` and move these definitions out of `src-tauri/src/lib.rs`:

- `CustomTranslationProviderDef`
- `create_llm_translation_provider`

Export them through `src-tauri/src/application/providers/mod.rs`.

Update imports in `lib.rs` and `provider_commands.rs`.

- [ ] **Step 5: Move credential validation helpers**

Move repeated credential validation from `provider_commands.rs` and `ocr_commands.rs` into `configuration.rs`:

```rust
pub fn validate_required_credentials(
    fields: &[CredentialField],
    credentials: &std::collections::HashMap<String, String>,
) -> crate::Result<()> {
    for field in fields {
        let value = credentials.get(&field.name).ok_or_else(|| {
            crate::AppError::Other(format!("Missing required field: {}", field.label))
        })?;
        if value.trim().is_empty() {
            return Err(crate::AppError::Other(format!(
                "Field cannot be empty: {}",
                field.label
            )));
        }
    }
    Ok(())
}
```

- [ ] **Step 6: Update commands to use Provider configuration module**

Modify:

- `configure_translation_provider_credentials`
- `add_custom_translation_provider`
- `remove_custom_translation_provider`
- `configure_ocr_provider`

Expected: commands stay as Tauri adapters; validation and provider construction knowledge moves to the Provider configuration Module.

- [ ] **Step 7: Run Provider tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml provider --lib
cargo test --manifest-path src-tauri/Cargo.toml ocr::coordinator --lib
cargo test --manifest-path src-tauri/Cargo.toml translation::coordinator --lib
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/application/providers src-tauri/src/commands src-tauri/src/lib.rs
git commit -m "refactor(providers): centralize provider configuration lifecycle"
```

---

## Task 6: Move Application Composition Out of `lib.rs`

**Files:**
- Create: `src-tauri/src/composition.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create composition module**

Create `src-tauri/src/composition.rs` with functions that build runtime dependencies but do not run Tauri:

```rust
use std::path::PathBuf;

use tauri::AppHandle;

use crate::AppState;

pub fn build_app_state(config_path: PathBuf, app: AppHandle) -> AppState {
    AppState::new(config_path, app)
}
```

This starts as a pass-through. Later steps move construction internals out of `AppState::new`.

- [ ] **Step 2: Move Provider registration helpers**

Move Provider registration blocks from `AppState::new` into private functions in `composition.rs`:

- translation Provider registration
- custom LLM Provider restoration
- OCR Provider registration
- event bus subscription helper if it can move without Tauri runtime coupling

Keep `AppState` struct definition in `lib.rs` for now.

- [ ] **Step 3: Update `run()` setup hook**

Modify `src-tauri/src/lib.rs` setup:

```rust
let app_state = composition::build_app_state(config_path, app.handle().clone());
```

- [ ] **Step 4: Run Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: PASS.

- [ ] **Step 5: Inspect `lib.rs` diff**

Run:

```bash
wc -l src-tauri/src/lib.rs src-tauri/src/composition.rs
git diff -- src-tauri/src/lib.rs src-tauri/src/composition.rs
```

Expected: `lib.rs` loses provider construction detail and keeps Tauri builder, command registration, and AppState shape.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/composition.rs
git commit -m "refactor(app): move runtime composition out of lib"
```

---

## Task 7: Deepen Settings Window Navigation

**Files:**
- Create: `src/components/SettingsWindow/navigationModel.tsx`
- Modify: `src/components/SettingsWindow/index.tsx`
- Modify: `src/stores/settingsStore.ts`
- Test: optional `src/components/SettingsWindow/navigationModel.test.tsx`

- [ ] **Step 1: Add navigation model**

Create `src/components/SettingsWindow/navigationModel.tsx`:

```tsx
import { HotkeysPage as ScreenshotHotkeysPage } from './Screenshot/HotkeysPage';
import { SaveSettingsPage } from './Screenshot/SaveSettingsPage';
import { EditorPage as ScreenshotEditorPage } from './Screenshot/EditorPage';
import { FavoritesPage as ScreenshotFavoritesPage } from './Screenshot/FavoritesPage';
// Continue importing existing pages here.

export type MainTab = 'screenshot' | 'translation' | 'ocr' | 'services' | 'general' | 'advanced';

export interface SecondaryNavItem {
  key: string;
  label: string;
  render: () => JSX.Element;
}

export interface SettingsSection {
  key: MainTab;
  label: string;
  secondary?: SecondaryNavItem[];
  render?: () => JSX.Element;
}

export const settingsSections: SettingsSection[] = [
  {
    key: 'screenshot',
    label: '截图',
    secondary: [
      { key: 'hotkeys', label: '快捷键', render: () => <ScreenshotHotkeysPage /> },
      { key: 'save-settings', label: '保存设置', render: () => <SaveSettingsPage /> },
      { key: 'editor', label: '编辑器', render: () => <ScreenshotEditorPage /> },
      { key: 'favorites', label: '收藏夹', render: () => <ScreenshotFavoritesPage /> },
    ],
  },
  // Add translation, ocr, services, general, advanced.
];
```

- [ ] **Step 2: Update settings store types**

Modify `src/stores/settingsStore.ts` to import `MainTab` from `navigationModel.tsx`. Replace `as any` call sites by typed helper functions.

- [ ] **Step 3: Refactor SettingsWindow render**

Modify `src/components/SettingsWindow/index.tsx`:

- Read `settingsSections`.
- Find the active section.
- Render `SecondaryNav` from `section.secondary`.
- Render active page via `render()`.

Expected: repeated nav item arrays and tab if chains disappear.

- [ ] **Step 4: Run TypeScript build**

Run:

```bash
npm run build
```

Expected: PASS and no `as any` in `src/components/SettingsWindow/index.tsx`.

- [ ] **Step 5: Commit**

```bash
git add src/components/SettingsWindow src/stores/settingsStore.ts
git commit -m "refactor(settings): centralize settings navigation model"
```

---

## Task 8: Documentation and Final Verification

**Files:**
- Modify: `CONTEXT.md`
- Modify: `ARCHITECTURE.md`
- Possibly create: `docs/adr/0005-runtime-seams-and-composition.md`

- [ ] **Step 1: Update domain language**

Update `CONTEXT.md` if these Module names become stable:

- Frontend Tauri Adapter
- Capture Session Runtime
- Provider Configuration Module
- Application Composition

Keep wording consistent with existing Capture Session and Provider language.

- [ ] **Step 2: Update architecture doc**

Update `ARCHITECTURE.md` to reflect the actual current directories:

- `src/tauri/` as frontend adapter seam.
- `src-tauri/src/commands/` as backend Tauri adapter seam.
- `src-tauri/src/composition.rs` as runtime composition.
- Capture Session Runtime as Application module.

- [ ] **Step 3: Add ADR if a durable decision was made**

If Task 5 chooses runtime OCR reconfiguration as required behavior, create `docs/adr/0005-runtime-provider-reconfiguration.md`:

```markdown
# ADR 0005: Runtime Provider Reconfiguration

## Status
Accepted

## Context
Provider configuration commands save credentials while the UI expects the Provider to be usable immediately.

## Decision
Provider configuration must update the runtime Provider instance when the Provider is already registered.

## Consequences
Provider Coordinators need a mutable Provider seam. Configuration tests target the Coordinator Interface instead of command internals.
```

- [ ] **Step 4: Run full verification**

Run:

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --tests
```

Expected: PASS, or documented pre-existing failures only.

- [ ] **Step 5: Inspect final architecture diff**

Run:

```bash
git diff --stat
rg "invoke<|invoke\\(" src -n
rg "Runtime reconfiguration requires restart|TODO: Implement SQLite storage|TODO: Implement with whatlang" src-tauri/src -n
```

Expected:

- Runtime command calls are concentrated under `src/tauri/`.
- The OCR restart-only configuration message is gone if Task 5 was implemented.
- Stale TODOs from deleted files no longer appear.

- [ ] **Step 6: Commit**

```bash
git add CONTEXT.md ARCHITECTURE.md docs/adr docs/architecture
git commit -m "docs(architecture): document runtime seams and deep modules"
```

---

## Execution Notes

- Keep commits task-sized.
- Do not reformat unrelated files.
- Do not rename `src/` or `src-tauri/`.
- Do not move Capture Session UI state before Tasks 2-4 make the seam explicit.
- If a test fails for an unrelated pre-existing reason, record the exact failure and continue only when the current task's behavior is protected by a focused test.
- Before implementing this plan, use `superpowers:subagent-driven-development` for task-by-task execution or `superpowers:executing-plans` for inline execution.
