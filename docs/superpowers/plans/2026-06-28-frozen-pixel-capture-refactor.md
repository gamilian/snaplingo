# Frozen Pixel Capture Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every active screenshot entry freeze screen pixels before the capture overlay is revealed, while treating Settings, Main, and pinned windows as ordinary capture subjects.

**Architecture:** Keep `CaptureSessionService` as the frozen pixel owner and keep the `capture` webview as the only special window. Startup returns lightweight session metadata to the frontend; preview, copy, save, pin, OCR, and print render from backend-frozen pixels instead of requiring full monitor base64 hydration before overlay reveal.

**Tech Stack:** Tauri 2, Rust, macOS AppKit/CoreGraphics screenshot backend, React 18, TypeScript, Vitest, Cargo tests.

---

## Scope And Success Criteria

- Shortcut path freezes pixels before opening/revealing the capture overlay.
- Direct capture session creation also creates a frozen session; it is not layout-only.
- Frontend reveals the overlay after metadata and canvas surface are ready; full monitor base64 hydration is not a reveal precondition.
- Output/OCR/preview commands render from backend-frozen pixels and do not force frontend snapshot hydration first.
- Settings, Main, and `pin-*` windows are not hidden by screenshot startup. Only a previously visible `capture` overlay may be hidden before a new freeze.
- Magnifier/color sampling may use lazy hydration and may be unavailable until pixels arrive.
- Legacy screenshot-window removal is a follow-up phase, not part of the first implementation slice.

## File Structure

- Modify `src-tauri/src/application/services/capture_session_service.rs`
  Keep frozen sessions as the primary service behavior. Retain lazy hydration only as an optional frontend pixel hydration API.
- Modify `src-tauri/src/application/services/capture_session_service_test.rs`
  Lock service semantics for frozen sessions, metadata-only views, and optional hydration.
- Modify `src-tauri/src/commands/capture_session_commands.rs`
  Make `create_capture_session` use frozen session creation and make shortcut startup use one frozen session path without frontend image payload.
- Modify `src-tauri/src/infrastructure/system/capture_window/backend.rs`
  Keep only `capture` in the hide set and update tests/names to reflect the ordinary-window semantics.
- Modify `src/components/ScreenshotSession/captureWindowVisibility.ts`
  Reveal when a session exists and status is selectable/previewable; remove image-readiness from reveal permission.
- Modify `src/components/ScreenshotSession/captureWindowVisibility.test.ts`
  Replace hydration-gated reveal tests with metadata-gated reveal tests.
- Modify `src/components/ScreenshotSession/index.tsx`
  Stop hydrating full monitor images during startup and before output/preview/OCR. Keep lazy hydration only for optional pixel UI such as magnifier/color sampling.
- Modify `src/components/ScreenshotSession/magnifier.ts`
  Keep magnifier gated by hydrated pixels so it degrades gracefully.
- Modify `src/tauri/__tests__/captureSession.test.ts`
  Keep adapter coverage for optional hydration command.

---

### Task 1: Lock Overlay Reveal Semantics

**Files:**
- Modify: `src/components/ScreenshotSession/captureWindowVisibility.test.ts`
- Modify: `src/components/ScreenshotSession/captureWindowVisibility.ts`

- [x] **Step 1: Write the failing test**

Replace the hydration-gated reveal assertion with:

```ts
it('reveals the capture window once metadata is ready even before image hydration', () => {
  expect(
    shouldRevealCaptureWindow({
      status: 'selecting',
      hasSession: true,
      hasCaptureImagesReady: false,
      hasRevealed: false,
    }),
  ).toBe(true);
});
```

- [x] **Step 2: Run the focused test and verify it fails**

Run: `npm test -- src/components/ScreenshotSession/captureWindowVisibility.test.ts`

Expected: FAIL because `shouldRevealCaptureWindow` still requires `hasCaptureImagesReady`.

- [x] **Step 3: Implement minimal reveal change**

Change `shouldRevealCaptureWindow` to require only:

```ts
hasSession && (status === 'selecting' || status === 'preview')
```

Keep `status === 'error'` revealing so permission/session errors remain visible.

- [x] **Step 4: Run the focused test and verify it passes**

Run: `npm test -- src/components/ScreenshotSession/captureWindowVisibility.test.ts`

Expected: PASS.

---

### Task 2: Stop Startup Hydration From Blocking Or Driving Capture

**Files:**
- Modify: `src/components/ScreenshotSession/index.tsx`
- Test: `src/components/ScreenshotSession/captureWindowVisibility.test.ts`
- Test: `src/components/ScreenshotSession/magnifier.test.ts`

- [x] **Step 1: Remove startup hydration**

In `startSession`, remove the immediate `ensureCaptureSnapshotsHydrated(nextSession.id)` call. Session metadata is sufficient for selection and hover recommendation.

- [x] **Step 2: Remove output-time frontend hydration**

Remove `await ensureCaptureSnapshotsHydrated(session.id)` before:

- `copyCaptureSelection`
- `saveCaptureSelection`
- `quickSaveCaptureSelection`
- `printCaptureSelection`
- `outputCapture`
- `runCaptureOcr`
- `renderCaptureOutput`

These commands must depend on backend-frozen pixels already stored in the session.

- [x] **Step 3: Keep optional lazy hydration helper**

Keep `ensureCaptureSnapshotsHydrated` for magnifier/color sampling only. It must not affect overlay reveal or capture completion.

- [x] **Step 4: Run focused frontend tests**

Run:

```bash
npm test -- src/components/ScreenshotSession/captureWindowVisibility.test.ts src/components/ScreenshotSession/magnifier.test.ts src/tauri/__tests__/captureSession.test.ts
```

Expected: PASS.

---

### Task 3: Make Direct Session Creation Frozen-First

**Files:**
- Modify: `src-tauri/src/application/services/capture_session_service_test.rs`
- Modify: `src-tauri/src/commands/capture_session_commands.rs`
- Modify: `src-tauri/src/application/services/capture_session_service.rs`

- [x] **Step 1: Add/adjust failing service tests**

Add tests named `create_session_view_without_monitor_images_keeps_backend_pixels_cached` and `create_session_without_monitor_images_returns_metadata_with_cached_pixels`:

```rust
#[tokio::test]
async fn create_session_view_without_monitor_images_keeps_backend_pixels_cached() {
    let backend = make_backend();
    let snapshot_calls = backend.capture_monitor_snapshots_calls.clone();
    let layout_calls = backend.capture_monitor_layouts_calls.clone();
    let service = CaptureSessionService::new(Arc::new(backend));

    let view = service.create_session().await.unwrap();
    let frontend_view = service.get_session_view_without_monitor_images(&view.id).unwrap();

    assert_eq!(*snapshot_calls.lock().unwrap(), 1);
    assert_eq!(*layout_calls.lock().unwrap(), 0);
    assert_eq!(frontend_view.monitors[0].image_base64, "");
    assert!(!service
        .session_selection_needs_freeze(
            &view.id,
            &LogicalRect {
                x: 1.0,
                y: 1.0,
                width: 2.0,
                height: 2.0,
            },
        )
        .unwrap());
}
```

- [x] **Step 2: Run the focused Rust test**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml create_session_view_without_monitor_images_keeps_backend_pixels_cached
```

Expected: PASS if the service already has the right behavior; otherwise FAIL and fix the service.

- [x] **Step 3: Change direct command startup**

Add `CaptureSessionService::create_session_without_monitor_images()` so the hot path freezes pixels and stores them without encoding full monitor base64 for the frontend view. Change `create_capture_session_from_visible_desktop` to call:

```rust
state.capture_session_service.create_session_without_monitor_images().await
```

The returned view is already metadata-only; render/output/OCR still use the stored frozen pixels.

- [x] **Step 4: Simplify shortcut startup**

Keep shortcut startup ordered as:

```text
begin capture presentation
create frozen session
return metadata-only view
open hidden capture overlay
frontend reveal
```

Remove the layout-session plus parallel snapshot-cache path from `create_triggered_capture_session_from_visible_desktop`.

- [x] **Step 5: Run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_session_service
cargo test --manifest-path src-tauri/Cargo.toml capture_window
```

Expected: PASS.

---

### Task 4: Confirm Ordinary App Window Semantics

**Files:**
- Modify: `src-tauri/src/infrastructure/system/capture_window/backend.rs`

- [x] **Step 1: Rename tests to product semantics**

Rename:

- `plans_visible_app_windows_to_hide_before_capture_snapshot`
- `plans_hidden_app_windows_to_restore_after_capture_snapshot`

to names that state only the `capture` window is special.

- [x] **Step 2: Update restore expectation**

`capture_snapshot_window_labels_to_restore` should not restore `main`, `settings`, `capture`, or `pin-*` as a side effect of screenshot capture. With the new semantics, there should normally be no business-window labels to restore because they were never hidden.

- [x] **Step 3: Run focused Rust test**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_window
```

Expected: PASS.

---

### Task 5: Verification Pass

**Files:**
- No production edits unless verification reveals a defect.

- [x] **Step 1: Run frontend tests**

Run: `npm test`

Expected: PASS.

- [x] **Step 2: Run frontend build**

Run: `npm run build`

Expected: PASS.

- [x] **Step 3: Run Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 4: Manual checks**

Manual macOS checks before release:

- Settings visible: captured if visible.
- Settings hidden: not captured.
- Settings covered by another app: covered pixels captured.
- Pinned image visible: captured if visible.
- Existing capture overlay visible from previous attempt: hidden before new freeze.
- Fullscreen Space: overlay appears after freeze.
- Rapid repeated hotkeys: second open is ignored while first is in flight.

---

## Follow-Up Phase

After the first frozen-first path is stable:

- Introduce a named `CaptureOverlayHost` application service if command-layer Tauri/AppKit details remain too broad.
- Remove or quarantine legacy `ScreenshotWorkflow`, `ScreenshotCapture`, `ScreenshotEditor`, `screenshot_window_commands.rs`, and `public/screenshot.html` after confirming no command/router references remain.
- Remove optional full-monitor hydration if magnifier/color sampling is redesigned to use a small backend sampling API.
