# Capture Module Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete the obsolete parallel Capture path so all product screenshot workflows flow through the existing deep Capture Session module.

**Architecture:** Keep `CaptureSessionRuntime`, `CaptureSessionService`, `CaptureOutputService`, and the platform `ScreenshotBackend` adapters as the active Capture path. Remove the shallow `CaptureService` module, its unused Tauri commands, its frontend adapter export, and related state residue. Preserve `trigger_screenshot` and `ScreenshotBackend::capture_region` because the Advanced Settings smoke entrypoint and Capture Session snapshot hydration still use them.

**Tech Stack:** Rust, Tauri 2, React, TypeScript, Vitest, Cargo tests.

---

## Scope

In scope:

- Delete the legacy `CaptureService` module.
- Delete `capture_full_screen`, `capture_region`, and `save_screenshot` Tauri commands.
- Remove the unused frontend `captureFullScreen` adapter export.
- Remove `CaptureService` from Application exports, Application Composition, and `AppState`.
- Remove the dead `ScreenshotBackend::capture_full_screen` interface and platform implementations.
- Remove the unused `ScreenshotState` runtime state.
- Delete the stale placeholder capture integration test.
- Preserve the Capture Session command names and frontend-visible payloads.

Out of scope:

- Do not change menu or Hotkey action dispatch.
- Do not move the Screenshot contract out of Infrastructure.
- Do not refactor Capture Workspace.
- Do not rename Capture Session Tauri commands.
- Do not remove `ScreenshotBackend::capture_region`.
- Do not validate Windows or Linux runtime behavior in this plan.

## Assumptions

- Execute in a dedicated clean worktree created from a commit that already contains the complete Capture IPC type migration.
- The Capture IPC migration spans `src/domain/capture.ts`, `src/domain/capture.test.ts`, the Capture/Pinned/OCR consumers under `src/components/`, `src/tauri/`, `src/stores/appStore.ts`, and the cleanup in `src/types/index.ts`. All of these changes belong to the user and must be preserved.
- If that migration is still uncommitted, stop and ask the user to finish or authorize a separate baseline commit. Do not stash, partially copy, or absorb it into the consolidation commits.
- No external client depends on the three legacy Tauri command names; repository search shows no active consumer.
- Historical files under `docs/superpowers/plans/` are execution records and must not be rewritten merely because they mention the old shape.

## Success Criteria

- `triggerScreenshot()` still invokes `trigger_screenshot`.
- The Advanced Settings screenshot test still opens the Capture Session workflow.
- `capture_region` remains available only as the internal Screenshot backend capability used by Capture Session.
- Active source contains no `CaptureService`, `capture_full_screen`, `save_screenshot`, `captureFullScreen`, `ScreenshotState`, or `screenshot_state` references.
- Frontend tests/build and Rust tests pass.

## File Structure

Delete:

- `src-tauri/src/application/services/capture_service.rs` — shallow legacy Capture module.
- `src-tauri/src/commands/capture_commands.rs` — unused legacy Tauri commands.
- `src-tauri/tests/capture_integration_test.rs` — placeholder test with no behavioral coverage.

Modify:

- `src/tauri/captureSession.ts` — remove the unused `captureFullScreen` export; retain `triggerScreenshot`.
- `src/tauri/__tests__/captureSession.test.ts` — characterize the retained `triggerScreenshot` adapter.
- `src-tauri/src/application/services/mod.rs` — stop declaring and exporting `capture_service`.
- `src-tauri/src/application/mod.rs` — stop re-exporting `CaptureService`.
- `src-tauri/src/commands/mod.rs` — stop declaring and exporting `capture_commands`.
- `src-tauri/src/app_state.rs` — remove `CaptureService` and `ScreenshotState` from runtime state.
- `src-tauri/src/composition/capture_runtime.rs` — construct only the Capture Session path.
- `src-tauri/src/composition.rs` — remove legacy Capture and screenshot-state wiring.
- `src-tauri/src/lib.rs` — remove legacy command registration and stale state re-export.
- `src-tauri/src/infrastructure/system/screenshot/backend.rs` — remove the dead full-screen method.
- `src-tauri/src/infrastructure/system/screenshot/macos.rs` — remove the dead full-screen implementation.
- `src-tauri/src/infrastructure/system/screenshot/windows.rs` — remove the dead full-screen implementation.
- `src-tauri/src/infrastructure/system/screenshot/linux.rs` — remove the dead full-screen implementation.
- `src-tauri/src/infrastructure/system/screenshot/xcap_common.rs` — remove the dead full-screen helper.
- `src-tauri/src/application/services/capture_session_runtime.rs` — remove the obsolete method from the test backend.
- `src-tauri/src/application/services/capture_session_service_test.rs` — remove the obsolete method from the test backend.

### Task 0: Establish a Clean Baseline

**Files:** none.

- [ ] **Step 1: Audit the complete Capture IPC migration before creating a worktree**

Run in the current user worktree:

```bash
git status --short
git diff --name-only
git ls-files --others --exclude-standard
```

Expected: the audit includes the complete migration, especially:

- `src/domain/capture.ts`
- `src/domain/capture.test.ts`
- `src/components/ScreenshotSession/types.ts`
- `src/components/ScreenshotSession/windowMode.ts`
- `src/components/PinnedImageWindow/index.tsx`
- `src/components/PinnedImageWindow/pinActions.ts`
- `src/components/PinnedImageWindow/pinActions.test.ts`
- `src/components/ResultWindow/ocrFileWorkflow.ts`
- `src/tauri/captureSession.ts`
- `src/tauri/__tests__/captureSession.test.ts`
- `src/tauri/ocr.ts`
- `src/tauri/pinnedImage.ts`
- `src/stores/appStore.ts`
- `src/types/index.ts`

If any of these changes are uncommitted, stop. The user must finish or authorize a separate Capture IPC baseline commit before this plan starts.

- [ ] **Step 2: Verify the baseline commit contains the migration**

After the migration has its own commit, run:

```bash
git log --name-status --format="commit %H %s" -n 20 -- src/domain src/components/ScreenshotSession/types.ts src/components/ScreenshotSession/windowMode.ts src/components/PinnedImageWindow src/components/ResultWindow/ocrFileWorkflow.ts src/tauri src/stores/appStore.ts src/types/index.ts
```

Expected: committed history reachable from `HEAD` contains the complete migration file set above. No migration file may exist only in a dirty worktree.

- [ ] **Step 3: Start from an isolated worktree**

Use `superpowers:using-git-worktrees` to create a worktree from that baseline commit or branch.

- [ ] **Step 4: Inspect the isolated baseline**

Run:

```bash
git status --short
```

Expected: clean worktree. The complete Capture IPC migration is already present in committed history.

- [ ] **Step 5: Run the focused frontend baseline**

Run:

```bash
npm test -- src/tauri/__tests__/captureSession.test.ts
```

Expected: PASS.

- [ ] **Step 6: Run the backend baseline**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS. Stop and record any pre-existing failure before editing.

### Task 1: Protect the Retained Capture Session Entrypoint

**Files:**

- Modify: `src/tauri/__tests__/captureSession.test.ts`
- Verify: `src/tauri/captureSession.ts`
- Verify: `src/components/SettingsWindow/Advanced/AdvancedPage.tsx`

- [ ] **Step 1: Add a characterization test**

Add:

```typescript
it('keeps the Advanced Settings screenshot entrypoint on Capture Session', async () => {
  const { triggerScreenshot } = await import('../captureSession');
  invoke.mockResolvedValueOnce(undefined);

  await triggerScreenshot();

  expect(invoke).toHaveBeenCalledWith('trigger_screenshot');
});
```

This protects the retained path before neighboring legacy exports are removed.

- [ ] **Step 2: Run the focused test**

Run:

```bash
npm test -- src/tauri/__tests__/captureSession.test.ts
```

Expected: PASS.

- [ ] **Step 3: Confirm the UI uses only the retained entrypoint**

Run:

```bash
rg -n "triggerScreenshot|captureFullScreen" src/components src/tauri
```

Expected:

- `AdvancedPage.tsx` imports and calls `triggerScreenshot`.
- `captureFullScreen` is only defined in `src/tauri/captureSession.ts`.

- [ ] **Step 4: Commit**

```bash
git add src/tauri/__tests__/captureSession.test.ts
git commit -m "test: protect capture session trigger adapter"
```

### Task 2: Remove the Frontend Legacy Capture Export

**Files:**

- Modify: `src/tauri/captureSession.ts`
- Test: `src/tauri/__tests__/captureSession.test.ts`

- [ ] **Step 1: Delete the unused export**

Remove:

```typescript
export async function captureFullScreen() {
  return invoke<string>('capture_full_screen');
}
```

Keep:

```typescript
export async function triggerScreenshot() {
  return invoke<void>('trigger_screenshot');
}
```

- [ ] **Step 2: Verify no frontend consumer remains**

Run:

```bash
rg -n "captureFullScreen|capture_full_screen" src
```

Expected: no output.

- [ ] **Step 3: Run focused tests and type checking**

Run:

```bash
npm test -- src/tauri/__tests__/captureSession.test.ts
npm run build
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/tauri/captureSession.ts src/tauri/__tests__/captureSession.test.ts
git commit -m "refactor: remove legacy capture frontend adapter"
```

### Task 3: Delete the Parallel Rust Capture Module

**Files:**

- Delete: `src-tauri/src/application/services/capture_service.rs`
- Delete: `src-tauri/src/commands/capture_commands.rs`
- Modify: `src-tauri/src/application/services/mod.rs`
- Modify: `src-tauri/src/application/mod.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/composition/capture_runtime.rs`
- Modify: `src-tauri/src/composition.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Remove module declarations and exports**

Delete these declarations/exports:

```rust
pub mod capture_service;
pub use capture_service::CaptureService;
mod capture_commands;
pub use capture_commands::*;
```

Remove `CaptureService` from the `crate::application` re-export list.

- [ ] **Step 2: Remove the legacy runtime field**

Change `CaptureRuntimeState` from:

```rust
pub struct CaptureRuntimeState {
    pub capture: Arc<CaptureService>,
    pub sessions: Arc<CaptureSessionService>,
    // existing fields
}
```

to:

```rust
pub struct CaptureRuntimeState {
    pub sessions: Arc<CaptureSessionService>,
    // existing fields unchanged
}
```

Remove the `CaptureService` import.

- [ ] **Step 3: Simplify Capture composition**

Change the construction flow to pass the platform backend directly to `CaptureSessionService`:

```rust
let screenshot_backend = get_screenshot_backend();
let capture_session_service =
    Arc::new(CaptureSessionService::new(screenshot_backend));
```

Remove:

- `CaptureRuntimeParts::capture_service`
- `CaptureService::new(...)`
- `capture_service` from the returned parts
- `capture: capture_runtime.capture_service` from `composition.rs`

- [ ] **Step 4: Remove legacy Tauri command registration**

Remove from `tauri::generate_handler!`:

```rust
commands::capture_full_screen,
commands::capture_region,
commands::save_screenshot,
```

Do not remove `commands::trigger_screenshot` or any Capture Session command.

- [ ] **Step 5: Delete the two legacy files**

Delete:

- `src-tauri/src/application/services/capture_service.rs`
- `src-tauri/src/commands/capture_commands.rs`

- [ ] **Step 6: Format and compile**

Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 7: Verify the active Rust path**

Run:

```bash
rg -n "CaptureService|capture_service|commands::capture_full_screen|commands::capture_region|commands::save_screenshot|\.capture\.capture\b" src-tauri/src
```

Expected: no output.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src
git commit -m "refactor: remove parallel capture module"
```

### Task 4: Shrink the Screenshot Backend Interface

**Files:**

- Modify: `src-tauri/src/infrastructure/system/screenshot/backend.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/macos.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/windows.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/linux.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/xcap_common.rs`
- Modify: `src-tauri/src/application/services/capture_session_runtime.rs`
- Modify: `src-tauri/src/application/services/capture_session_service_test.rs`

- [ ] **Step 1: Remove the dead trait method**

Delete:

```rust
async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError>;
```

Keep:

```rust
async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError>;
```

`capture_region` is required by `CaptureSessionService::capture_selection_snapshots`.

- [ ] **Step 2: Remove platform implementations**

Delete `capture_full_screen` from:

- `MacOSScreenshotBackend`
- `WindowsScreenshotBackend`
- `LinuxScreenshotBackend`

Do not alter monitor snapshot, window candidate, cursor, or region capture behavior.

- [ ] **Step 3: Remove the unused XCap full-screen helper**

Delete `capture_full_screen_png()` from `xcap_common.rs`.

Keep `get_primary_monitor()` because `capture_region_png()` still uses it. Do not remove `rgba_image_to_png()`; monitor snapshot capture also uses it.

- [ ] **Step 4: Update test backends**

Delete only the obsolete `capture_full_screen` implementations from the fake `ScreenshotBackend` implementations in:

- `capture_session_runtime.rs`
- `capture_session_service_test.rs`

Do not change their `capture_region` behavior.

- [ ] **Step 5: Run focused residue checks**

Run:

```bash
rg -n "capture_full_screen|capture_full_screen_png" src-tauri/src
rg -n "capture_region" src-tauri/src/application/services/capture_session_service.rs src-tauri/src/infrastructure/system/screenshot
```

Expected:

- First command: no output.
- Second command: Capture Session and platform backend references remain.

- [ ] **Step 6: Format and test**

Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src
git commit -m "refactor: shrink screenshot backend interface"
```

### Task 5: Remove Capture State Residue

**Files:**

- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/composition/capture_runtime.rs`
- Modify: `src-tauri/src/composition.rs`
- Modify: `src-tauri/src/lib.rs`
- Delete: `src-tauri/tests/capture_integration_test.rs`

- [ ] **Step 1: Remove the unused screenshot state**

Delete `ScreenshotState`:

```rust
#[derive(Default)]
pub struct ScreenshotState {
    pub data: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}
```

Remove:

- `CaptureRuntimeState::screenshot_state`
- `CaptureRuntimeParts::screenshot_state`
- its construction and composition wiring
- `pub use app_state::{AppState, ScreenshotState};` in favor of exporting only `AppState`
- now-unused `parking_lot::Mutex` imports in these files

- [ ] **Step 2: Delete the placeholder integration test**

Delete `src-tauri/tests/capture_integration_test.rs`. It contains only `assert!(true)` and describes the removed `capture_service` path.

- [ ] **Step 3: Verify residue is gone**

Run:

```bash
rg -n "ScreenshotState|screenshot_state|capture_service|capture_screen" src-tauri/src src-tauri/tests
```

Expected: no output.

- [ ] **Step 4: Run backend tests**

Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src src-tauri/tests
git commit -m "refactor: remove legacy screenshot state"
```

### Task 6: Full Verification

**Files:** none.

- [ ] **Step 1: Run architecture residue searches**

Run:

```bash
rg -n "CaptureService|capture_service|capture_full_screen|save_screenshot|captureFullScreen|ScreenshotState|screenshot_state" src src-tauri/src src-tauri/tests
```

Expected: no output.

- [ ] **Step 2: Confirm retained paths**

Run:

```bash
rg -n "trigger_screenshot|triggerScreenshot" src src-tauri/src
rg -n "capture_region" src-tauri/src/application/services/capture_session_service.rs src-tauri/src/infrastructure/system/screenshot
```

Expected:

- `trigger_screenshot` remains registered and used by `triggerScreenshot`.
- `capture_region` remains inside Capture Session and platform adapters.

- [ ] **Step 3: Run frontend verification**

Run:

```bash
npm test
npm run build
```

Expected: PASS.

- [ ] **Step 4: Run backend verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 5: Check patch integrity**

Run:

```bash
git diff --check
git status --short
```

Expected: no whitespace errors; only intentional changes remain.

- [ ] **Step 6: Manual Tauri smoke test**

Run:

```bash
npm run tauri:dev
```

In Settings → Advanced, click the screenshot test action.

Expected:

- Capture overlay opens through `trigger_screenshot`.
- A selection can be made and cancelled.
- Save/copy/OCR paths still use Capture Session.
- No call attempts any removed legacy command.

Stop the dev process after the smoke test.

## Final Review Checklist

- The diff deletes more code than it adds.
- Capture Session is the only product Capture workflow module.
- No new abstraction is introduced.
- No Capture Session IPC payload changes.
- No unrelated formatting or refactor.
- User-owned Capture IPC type changes are preserved.
