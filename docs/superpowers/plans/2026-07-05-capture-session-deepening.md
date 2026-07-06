# Capture Session Deepening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deepen the Capture Session seams so backend commands and the frontend `ScreenshotSession` shell stop owning workflow choreography directly.

**Architecture:** Expand `CaptureSessionRuntime` into the backend workflow module for capture startup and failure rollback, while adding a frontend `captureHostRuntime.ts` module that owns effect interpretation, host event subscription, reveal timing, and selection persistence. Keep the command layer and React shell thin, and preserve current Capture Mode behavior.

**Tech Stack:** Rust, Tauri, React, TypeScript, Vitest, Cargo tests.

---

## File Structure

### Backend

- Modify: `src-tauri/src/application/services/capture_session_runtime.rs`
  - Expand the runtime interface so it owns capture startup choreography, main-thread dispatch, and rollback.
- Modify: `src-tauri/src/application/services/mod.rs`
  - Re-export any new runtime entrypoints used by commands.
- Modify: `src-tauri/src/commands/capture_session_commands.rs`
  - Reduce to Tauri command adapter behavior; route startup and orchestration into `CaptureSessionRuntime`.

### Frontend

- Create: `src/components/ScreenshotSession/captureHostRuntime.ts`
  - Deep frontend host/runtime module for effect execution, host subscriptions, reveal timing, and selection persistence.
- Modify: `src/components/ScreenshotSession/index.tsx`
  - Thin the React shell so it delegates host/runtime behavior.
- Reuse: `src/components/ScreenshotSession/captureInteractionRuntime.ts`
  - Keep pure effect planning only.
- Reuse: `src/components/ScreenshotSession/captureWindowVisibility.ts`
  - Keep reveal helpers as dependencies of the host runtime.
- Reuse: `src/components/ScreenshotSession/selectionMemory.ts`
  - Keep storage parsing/writing helpers as dependencies of the host runtime.
- Reuse: `src/components/ScreenshotSession/captureCancelRequest.ts`
  - Keep request subscription helpers as dependencies of the host runtime.

### Tests

- Modify: `src-tauri/src/commands/capture_session_commands.rs`
  - Extend command seam tests as the file gets thinner.
- Create or modify: `src-tauri/src/application/services/capture_session_runtime_test.rs`
  - Add focused runtime tests if a dedicated test file is the cleanest option.
- Create: `src/components/ScreenshotSession/captureHostRuntime.test.ts`
  - Focused tests for effect interpretation, host subscriptions, reveal timing, and selection persistence.
- Modify: `src/components/ScreenshotSession/captureInteractionRuntime.test.ts`
  - Keep plan/effect assertions aligned with the new execution seam.
- Modify: `src/components/ScreenshotSession/captureWindowVisibility.test.ts`
  - Verify reveal behavior still matches the host runtime contract.
- Modify: `src/components/ScreenshotSession/selectionMemory.test.ts`
  - Reuse storage helpers through the new host runtime tests where useful.

## Task 1: Lock the Backend Seam With Focused Tests

**Files:**
- Modify: `src-tauri/src/commands/capture_session_commands.rs`
- Create or modify: `src-tauri/src/application/services/capture_session_runtime_test.rs`

- [ ] **Step 1: Identify the backend behaviors that must survive the refactor**

Capture these behaviors as test targets:

- capture startup can begin once and blocks re-entry
- startup success returns after session creation and window open succeed
- startup failure triggers rollback ordering
- render/output/OCR behavior still routes through `CaptureSessionRuntime`

- [ ] **Step 2: Add failing focused tests for the expanded runtime seam**

Add tests that express the intended runtime ownership, such as:

```rust
#[tokio::test]
async fn capture_runtime_rolls_back_when_window_open_fails() {
    // Arrange runtime dependencies with a failing window adapter.
    // Assert restore -> cancel -> end presentation ordering.
}
```

- [ ] **Step 3: Run the focused Rust tests and confirm the new seam is not implemented yet**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_session_runtime
```

Expected: the new runtime-focused assertions fail before implementation.

## Task 2: Deepen the Backend Capture Session Runtime

**Files:**
- Modify: `src-tauri/src/application/services/capture_session_runtime.rs`
- Modify: `src-tauri/src/application/services/mod.rs`

- [ ] **Step 1: Add new runtime entrypoints for capture startup choreography**

Add backend methods that the command layer can call directly, for example:

```rust
pub async fn open_capture_for_mode(
    &self,
    app: &tauri::AppHandle,
    mode: &str,
) -> Result<()>;
```

The exact signature may vary, but the interface must absorb:

- begin capture presentation
- session creation
- main-thread window open
- rollback ordering

- [ ] **Step 2: Move main-thread dispatch and rollback behavior into the runtime**

Move logic currently owned by `capture_session_commands.rs` into the runtime:

- `run_on_main_thread(...)`
- capture presentation begin/end sequencing
- restore hidden windows on failure
- cancel failed session

- [ ] **Step 3: Keep render/output/OCR methods working through the same runtime module**

Preserve these existing runtime responsibilities:

- render PNG/base64 output
- output selection
- recognize selection OCR

Do not change Capture Mode semantics or adapter responsibilities.

- [ ] **Step 4: Run the focused Rust tests again**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_session_runtime
```

Expected: runtime-focused tests pass.

## Task 3: Thin the Backend Command Seam

**Files:**
- Modify: `src-tauri/src/commands/capture_session_commands.rs`

- [ ] **Step 1: Replace command-owned startup choreography with runtime calls**

Reduce command functions so they do only:

- decode inputs
- call `CaptureSessionRuntime`
- map errors to `String`

- [ ] **Step 2: Remove direct orchestration imports from the command module**

After the refactor, this file should no longer directly own most uses of:

- `begin_capture_presentation`
- `end_capture_presentation`
- `open_capture_window_for_session`
- `restore_capture_snapshot_windows`

- [ ] **Step 3: Keep the existing re-entrant shortcut guard test passing**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_shortcut_open_guard_blocks_reentrant_open_until_dropped
```

Expected: PASS

- [ ] **Step 4: Run all command-layer Rust tests that touch capture behavior**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_session_commands
```

Expected: PASS

## Task 4: Lock the Frontend Host Seam With Focused Tests

**Files:**
- Create: `src/components/ScreenshotSession/captureHostRuntime.test.ts`
- Modify: `src/components/ScreenshotSession/captureInteractionRuntime.test.ts`
- Modify: `src/components/ScreenshotSession/captureWindowVisibility.test.ts`
- Modify: `src/components/ScreenshotSession/selectionMemory.test.ts`

- [ ] **Step 1: Add failing tests for effect execution and host subscriptions**

Cover:

- `CaptureRuntimeEffect` execution routing
- `hotkey-triggered` subscription wiring
- native cancel/copy request subscription wiring
- reveal timing path
- selection persistence path

Example shape:

```ts
it('executes translation-window OCR effects through the provided adapters', async () => {
  // Arrange fake adapters and callbacks.
  // Assert OCR -> render/open path ordering.
});
```

- [ ] **Step 2: Run the focused frontend tests and confirm the host runtime does not exist yet**

Run:

```bash
npm test -- captureHostRuntime
```

Expected: FAIL because the new module/tests are not implemented yet.

## Task 5: Create the Frontend Capture Host Runtime

**Files:**
- Create: `src/components/ScreenshotSession/captureHostRuntime.ts`

- [ ] **Step 1: Add a narrow interface for executing capture runtime effects**

Design the module around dependencies passed in, for example:

```ts
export async function executeCaptureRuntimeEffect(
  effect: CaptureRuntimeEffect,
  context: CaptureHostRuntimeContext,
): Promise<void> {
  // ...
}
```

The exact shape may vary, but it must hide concrete adapter ordering from `index.tsx`.

- [ ] **Step 2: Move host event subscription helpers into the new module**

The module should own wiring for:

- `hotkey-triggered`
- native cancel requests
- native copy requests

- [ ] **Step 3: Move reveal timing and selection persistence helpers into the new module**

The module should own:

- reveal timing coordination
- localStorage-backed selection persistence wiring
- restore-from-history wiring

- [ ] **Step 4: Run the focused frontend tests**

Run:

```bash
npm test -- captureHostRuntime captureInteractionRuntime captureWindowVisibility selectionMemory
```

Expected: PASS

## Task 6: Thin the ScreenshotSession React Shell

**Files:**
- Modify: `src/components/ScreenshotSession/index.tsx`

- [ ] **Step 1: Replace direct effect interpretation with calls into `captureHostRuntime.ts`**

Move the current `executeCaptureRuntimeEffect(...)` behavior behind the new seam.

- [ ] **Step 2: Replace direct host event subscription wiring with `captureHostRuntime.ts`**

Move:

- `listenTauriEvent('hotkey-triggered', ...)`
- native cancel request subscription
- native copy request subscription

out of the shell.

- [ ] **Step 3: Replace direct selection persistence wiring with `captureHostRuntime.ts`**

Move shell-owned calls around:

- `saveLastCaptureSelection(window.localStorage, ...)`
- `loadLastCaptureSelection(window.localStorage)`
- `loadCaptureSelectionHistory(window.localStorage)`

behind the host runtime seam.

- [ ] **Step 4: Keep JSX, refs, and state behavior intact**

Do not redesign the component. Keep:

- Capture Mode behavior
- existing state transitions
- current UI rendering

- [ ] **Step 5: Run focused ScreenshotSession tests**

Run:

```bash
npm test -- ScreenshotSession
```

Expected: PASS for the targeted capture-related frontend tests.

## Task 7: Run Full Verification

**Files:**
- Verify only

- [ ] **Step 1: Run full Rust test suite**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS

- [ ] **Step 2: Run full frontend test suite**

Run:

```bash
npm test
```

Expected: PASS

- [ ] **Step 3: Run production frontend build**

Run:

```bash
npm run build
```

Expected: PASS

- [ ] **Step 4: Run whitespace/diff sanity check**

Run:

```bash
git diff --check
```

Expected: no output

## Task 8: Final Review Against the Spec

**Files:**
- Verify only

- [ ] **Step 1: Re-read the approved spec**

Spec:

- `docs/superpowers/specs/2026-07-05-capture-session-deepening-design.md`

- [ ] **Step 2: Verify backend success criteria**

Check:

- command module is thin
- `CaptureSessionRuntime` owns startup choreography
- rollback logic is concentrated in the runtime

- [ ] **Step 3: Verify frontend success criteria**

Check:

- `ScreenshotSession/index.tsx` no longer directly interprets `CaptureRuntimeEffect`
- `ScreenshotSession/index.tsx` no longer directly subscribes to host events
- `ScreenshotSession/index.tsx` no longer directly owns selection persistence wiring

- [ ] **Step 4: Summarize residual risks**

Call out any remaining risks explicitly, especially:

- large remaining file size in `ScreenshotSession/index.tsx`
- helper extraction that improved locality but did not fully deepen every internal seam
