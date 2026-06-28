# Lazy Capture Overlay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make screenshot entry fast by showing a transparent canvas selection overlay immediately, then freezing pixels only after the user confirms a selection.

**Architecture:** Split capture startup into a lightweight layout session and a deferred frozen image session. The layout session contains monitor geometry, window candidates, and cursor metadata but no PNG/base64 screen image. When the user confirms/copies/saves/OCRs, the app captures only the selected region, creates a normal `CaptureSession`, and reuses the existing output/rendering pipeline.

**Tech Stack:** Tauri 2, Rust, macOS AppKit/CoreGraphics screenshot backend, React 18, TypeScript, Vitest, Cargo tests.

---

## Scope And Success Criteria

- Screenshot shortcut opens the selection overlay without waiting for full-screen PNG/base64 image decode.
- Mouse hover recommendation works from monitor/window metadata before any screen image exists.
- Copy/save/pin/OCR/translate still output pixels from the selected region.
- Editing still works after selection confirmation by freezing the selected region into an image session.
- SnapLingo settings windows are treated like ordinary app windows; only the capture overlay itself must be excluded from the captured output.
- Existing frozen-session behavior remains available as fallback until the lazy path is stable.

## File Structure

- Modify `src-tauri/src/domain/capture.rs`
  Add frontend-safe layout-session view structs with no `image_base64`.
- Modify `src/components/ScreenshotSession/types.ts`
  Add matching TypeScript layout-session types and a session phase type.
- Modify `src-tauri/src/infrastructure/system/screenshot/backend.rs`
  Add backend monitor-layout and selected-region capture seams.
- Modify `src-tauri/src/infrastructure/system/screenshot/macos.rs`
  Implement monitor layout without screenshot pixels, selected-region capture, and overlay exclusion support.
- Modify `src-tauri/src/application/services/capture_session_service.rs`
  Add APIs for creating/storing layout sessions and freezing a selected region into an existing `CaptureSession`.
- Modify `src-tauri/src/commands/capture_session_commands.rs`
  Change visible screenshot startup to create layout sessions; add freeze command.
- Modify `src-tauri/src/infrastructure/system/capture_window/{backend,tauri,macos,mod}.rs`
  Ensure capture overlay can be transparent and excluded/hidden for final capture.
- Modify `src/tauri/captureSession.ts`
  Add frontend invoke wrappers for layout session and freeze commands.
- Modify `src/components/ScreenshotSession/index.tsx`
  Support layout-only selecting state and deferred freeze before preview/output.
- Modify existing focused tests under `src/components/ScreenshotSession/*.test.ts` and `src-tauri/src/application/services/capture_session_service_test.rs`.

---

### Task 1: Add Layout-Only Capture Session Types

**Files:**
- Modify: `src-tauri/src/domain/capture.rs`
- Modify: `src/components/ScreenshotSession/types.ts`
- Test: `src-tauri/src/application/services/capture_session_service_test.rs`

- [ ] **Step 1: Add failing Rust type/serialization test**

Add a test that constructs a layout session view and asserts it has monitor geometry and candidates but no image payload field.

```rust
#[test]
fn capture_layout_session_view_contains_no_image_payload() {
    let view = CaptureLayoutSessionView {
        id: CaptureSessionId("layout-1".to_string()),
        monitors: vec![MonitorLayoutView {
            id: "monitor-1".to_string(),
            logical_bounds: LogicalRect { x: 0.0, y: 0.0, width: 100.0, height: 50.0 },
            physical_bounds: PhysicalRect { x: 0, y: 0, width: 200, height: 100 },
            scale_factor: 2.0,
        }],
        candidates: Vec::new(),
        cursor_position: Some(LogicalPoint { x: 10.0, y: 10.0 }),
    };

    let json = serde_json::to_string(&view).unwrap();
    assert!(!json.contains("image_base64"));
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml capture_layout_session_view_contains_no_image_payload`

Expected: fails because `CaptureLayoutSessionView` / `MonitorLayoutView` do not exist.

- [ ] **Step 3: Add Rust domain structs**

Add:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorLayoutView {
    pub id: String,
    pub logical_bounds: LogicalRect,
    pub physical_bounds: PhysicalRect,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureLayoutSessionView {
    pub id: CaptureSessionId,
    pub monitors: Vec<MonitorLayoutView>,
    pub candidates: Vec<CaptureCandidateView>,
    pub cursor_position: Option<LogicalPoint>,
}
```

- [ ] **Step 4: Add TypeScript types**

Add `MonitorLayoutView` and `CaptureLayoutSessionView` to `src/components/ScreenshotSession/types.ts`.

- [ ] **Step 5: Run focused tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml capture_layout_session_view_contains_no_image_payload`

Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/domain/capture.rs src/components/ScreenshotSession/types.ts src-tauri/src/application/services/capture_session_service_test.rs
git commit -m "feat: add capture layout session types"
```

---

### Task 2: Add Backend Monitor Layout Without Screenshots

**Files:**
- Modify: `src-tauri/src/infrastructure/system/screenshot/backend.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/macos.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/windows.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/linux.rs`
- Test: `src-tauri/src/infrastructure/system/screenshot/backend.rs`
- Test: `src-tauri/src/infrastructure/system/screenshot/macos.rs`

- [ ] **Step 1: Add backend model and failing helper test**

Add `MonitorLayout` near `MonitorSnapshot`:

```rust
#[derive(Debug, Clone)]
pub struct MonitorLayout {
    pub id: String,
    pub logical_bounds: LogicalRect,
    pub physical_bounds: PhysicalRect,
    pub scale_factor: f64,
}
```

Add a test for converting monitor layout metadata into frontend view metadata.

- [ ] **Step 2: Add trait method with fallback**

Add to `ScreenshotBackend`:

```rust
async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>, AppError>;
```

Do not implement default fallback by calling `capture_monitor_snapshots`; that would hide performance regressions.

- [ ] **Step 3: Implement macOS layout enumeration**

In `macos.rs`, implement `capture_visible_display_layouts()` using `Monitor::all()` and `CGDisplay::new(display_id)` metadata only. It must not call `display.image()` or `image_to_png()`.

- [ ] **Step 4: Implement Windows/Linux compile-safe layout enumeration**

Use xcap monitor metadata if available. If platform metadata is insufficient, return a clear `AppError::System("Monitor layout capture is not implemented on this platform")` so fallback can use frozen capture deliberately.

- [ ] **Step 5: Run tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml infrastructure::system::screenshot
```

Expected: all screenshot backend tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/infrastructure/system/screenshot
git commit -m "feat: capture monitor layout without screenshots"
```

---

### Task 3: Store Layout Sessions Separately From Frozen Sessions

**Files:**
- Modify: `src-tauri/src/application/services/capture_session_service.rs`
- Modify: `src-tauri/src/application/services/capture_session_service_test.rs`

- [ ] **Step 1: Write failing service tests**

Add tests:

- `create_layout_session_does_not_capture_monitor_snapshots`
- `create_layout_session_returns_window_candidates`
- `current_cursor_position_works_for_layout_session`

Use the mock backend counters to assert `capture_monitor_layouts` is called and `capture_monitor_snapshots` is not.

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml create_layout_session_does_not_capture_monitor_snapshots
```

Expected: fails because service API does not exist.

- [ ] **Step 3: Implement `CaptureLayoutSession` storage**

Add a separate map:

```rust
layout_sessions: Arc<Mutex<HashMap<CaptureSessionId, CaptureLayoutSession>>>
```

Keep frozen `sessions` unchanged.

- [ ] **Step 4: Implement `create_layout_session_with_hidden_window_labels`**

Flow:

1. `capture_monitor_layouts()`
2. create temporary layout-shaped data for candidate detection
3. call a candidate method that accepts layout metadata
4. capture current cursor position
5. store `CaptureLayoutSession`
6. return `CaptureLayoutSessionView`

- [ ] **Step 5: Implement lookup/cancel helpers**

Add:

- `get_layout_session_view`
- `cancel_layout_session`
- `take_hidden_window_labels_for_layout_session`
- `current_cursor_position_for_layout_session`

- [ ] **Step 6: Run service tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_session_service
```

Expected: all service tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/application/services/capture_session_service.rs src-tauri/src/application/services/capture_session_service_test.rs
git commit -m "feat: add lightweight capture layout sessions"
```

---

### Task 4: Open Capture Window With Layout Session

**Files:**
- Modify: `src-tauri/src/commands/capture_session_commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/tauri/captureSession.ts`
- Test: `src-tauri/src/commands/capture_session_commands.rs` or existing command tests
- Test: `src/tauri/__tests__/captureSession.test.ts`

- [ ] **Step 1: Add failing command tests**

Assert `open_capture_window_for_mode` creates a layout session and logs payload size without `view_base64_bytes`.

- [ ] **Step 2: Add frontend invoke wrappers**

Add:

```ts
export async function getCaptureLayoutSession(sessionId: string) {
  return invoke<CaptureLayoutSessionView>('get_capture_layout_session', { sessionId });
}
```

- [ ] **Step 3: Add Tauri commands**

Add:

- `create_capture_layout_session`
- `get_capture_layout_session`
- `current_capture_layout_cursor_position`
- `cancel_capture_layout_session`

Register them in `src-tauri/src/lib.rs`.

- [ ] **Step 4: Change screenshot startup path**

For visible screenshot modes, `open_capture_window_for_mode` should:

1. begin capture presentation
2. create layout session
3. open capture window with `layoutSessionId`
4. not create frozen `CaptureSession`

Keep `silent-screenshot-ocr` on existing frozen path unless product behavior explicitly changes.

- [ ] **Step 5: Keep fallback**

If layout session creation fails, log a warning and fallback to current frozen `create_capture_session_from_visible_desktop` path.

- [ ] **Step 6: Run tests**

Run:

```bash
npm test -- captureSession.test.ts
cargo test --manifest-path src-tauri/Cargo.toml capture_session_commands
```

Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/capture_session_commands.rs src-tauri/src/lib.rs src/tauri/captureSession.ts src/tauri/__tests__/captureSession.test.ts
git commit -m "feat: start screenshot overlay from layout session"
```

---

### Task 5: Render Selection UI From Layout Session Only

**Files:**
- Modify: `src/components/ScreenshotSession/windowMode.ts`
- Modify: `src/components/ScreenshotSession/types.ts`
- Modify: `src/components/ScreenshotSession/index.tsx`
- Modify: `src/components/ScreenshotSession/captureHoverPolling.ts`
- Modify: `src/components/ScreenshotSession/capturePresentation.ts`
- Test: `src/components/ScreenshotSession/windowMode.test.ts`
- Test: `src/components/ScreenshotSession/captureWindowVisibility.test.ts`
- Test: `src/components/ScreenshotSession/captureHoverPolling.test.ts`

- [ ] **Step 1: Add launch parsing tests**

Extend launch parsing to accept `layoutSessionId`.

Expected URL shape:

```text
index.html?window=capture&mode=screenshot&layoutSessionId=layout-1
```

- [ ] **Step 2: Add state model**

In `index.tsx`, separate:

- `layoutSession: CaptureLayoutSessionView | null`
- `frozenSession: CaptureSessionView | null`
- `status: 'idle' | 'loading' | 'selecting' | 'freezing' | 'preview' | 'error'`

- [ ] **Step 3: Make selection bounds use layout monitors**

`getVirtualDesktopBounds` and monitor hit logic should accept monitor-like geometry with no `image_base64`.

- [ ] **Step 4: Render no monitor `<img>` in selecting mode**

For layout-only selecting:

- root stays transparent
- draw dim/selection/hover via `captureSelectionOverlay` canvas
- no `areCaptureImagesReady` gate
- reveal after canvas paint, not image load

- [ ] **Step 5: Keep existing frozen rendering for preview/edit**

Once a frozen session exists, render its monitor images exactly as today.

- [ ] **Step 6: Run focused frontend tests**

Run:

```bash
npm test -- windowMode.test.ts captureWindowVisibility.test.ts captureHoverPolling.test.ts capturePresentation.test.ts
```

Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add src/components/ScreenshotSession
git commit -m "feat: render fast capture overlay without screen images"
```

---

### Task 6: Freeze Selected Region On Demand

**Files:**
- Modify: `src-tauri/src/infrastructure/system/capture_window/macos.rs`
- Modify: `src-tauri/src/infrastructure/system/capture_window/tauri.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/backend.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/macos.rs`
- Modify: `src-tauri/src/application/services/capture_session_service.rs`
- Modify: `src-tauri/src/commands/capture_session_commands.rs`
- Modify: `src/tauri/captureSession.ts`
- Test: `src-tauri/src/application/services/capture_session_service_test.rs`

- [ ] **Step 1: Write failing freeze test**

Add `freeze_layout_selection_captures_only_selected_region`.

Expected behavior:

- given a layout session with monitor geometry
- when freezing rect `{ x: 100, y: 100, width: 300, height: 200 }`
- backend receives one `ScreenRegion` matching physical coordinates
- returned `CaptureSessionView` contains image data only for that region

- [ ] **Step 2: Add command wrapper**

Frontend:

```ts
export async function freezeCaptureSelection(layoutSessionId: string, rect: LogicalRect) {
  return invoke<CaptureSessionView>('freeze_capture_selection', { layoutSessionId, rect });
}
```

- [ ] **Step 3: Exclude overlay from final capture**

Preferred macOS path:

- set capture window sharing type to `NSWindowSharingNone`
- keep overlay visible while capturing selected region

Fallback path:

- order out/hide capture window
- wait one frame or `30ms`
- capture selected region
- restore/reveal capture window

Add a small platform abstraction so tests can assert the planned sequence without requiring AppKit in unit tests.

- [ ] **Step 4: Implement selected-region session creation**

In service:

```rust
pub async fn freeze_layout_selection(
    &self,
    layout_session_id: &CaptureSessionId,
    selection: LogicalRect,
) -> Result<CaptureSessionView>
```

Convert selected logical rect into one or more monitor intersections. Capture one image per intersection. Store as regular `CaptureSession` so existing render/output methods work.

- [ ] **Step 5: Preserve cursor metadata**

If layout session captured cursor metadata at start, carry it into the frozen session for `include captured cursor`.

- [ ] **Step 6: Run freeze tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml freeze_layout_selection_captures_only_selected_region
```

Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/infrastructure/system/capture_window src-tauri/src/infrastructure/system/screenshot src-tauri/src/application/services/capture_session_service.rs src-tauri/src/commands/capture_session_commands.rs src/tauri/captureSession.ts
git commit -m "feat: freeze selected capture region on demand"
```

---

### Task 7: Wire Deferred Freeze Into Copy/Save/OCR/Edit

**Files:**
- Modify: `src/components/ScreenshotSession/index.tsx`
- Modify: `src/components/ScreenshotSession/captureActions.ts`
- Modify: `src/components/ScreenshotSession/captureActions.test.ts`

- [ ] **Step 1: Add failing frontend action tests**

Add tests for:

- candidate double click in copy mode freezes first, then copies
- manual selection entering preview freezes first, then shows editor
- OCR freezes first, then calls `run_capture_ocr`

- [ ] **Step 2: Add `ensureFrozenSessionForSelection`**

In `index.tsx`:

```ts
async function ensureFrozenSessionForSelection(rect: LogicalRect): Promise<CaptureSessionView>
```

Rules:

- if `frozenSession` exists for current selection, reuse it
- otherwise set status `freezing`
- call `freezeCaptureSelection(layoutSession.id, rect)`
- set `frozenSession`
- return the session

- [ ] **Step 3: Update completion flows**

Before these effects:

- `copyCaptureSelection`
- `saveCaptureSelection`
- `quickSaveCaptureSelection`
- `printCaptureSelection`
- `outputCapture({ type: 'pin' })`
- `runCaptureOcr`
- preview/edit rendering

call `ensureFrozenSessionForSelection(rect)` and use returned `session.id`.

- [ ] **Step 4: Keep selection movement behavior simple**

If user changes selection after freezing, discard the frozen session or mark it stale. Do not try to recrop old frozen image unless the rect exactly matches.

- [ ] **Step 5: Run frontend tests**

Run:

```bash
npm test -- captureActions.test.ts
npm test -- ScreenshotSession
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add src/components/ScreenshotSession
git commit -m "feat: defer screenshot pixels until selection confirmation"
```

---

### Task 8: Performance Logs And Regression Guards

**Files:**
- Modify: `src-tauri/src/commands/capture_session_commands.rs`
- Modify: `src/components/ScreenshotSession/index.tsx`
- Modify: existing perf-related tests if needed

- [ ] **Step 1: Add layout startup perf logs**

Add log events:

- `[capture-perf] create_layout_session ... total_ms=...`
- `[capture-perf] open_capture_window ... create_layout_session_ms=...`
- `[capture-perf] freeze_capture_selection ... capture_region_ms=... total_ms=...`
- frontend events: `layout_loaded`, `overlay_revealed`, `freeze_started`, `freeze_complete`

- [ ] **Step 2: Remove misleading base64 metrics from layout startup**

Layout startup logs must not report `view_base64_bytes`.

- [ ] **Step 3: Add regression test for no base64 startup path**

Use command/service tests to assert layout startup does not call `session_to_view` or snapshot base64 generation.

- [ ] **Step 4: Run tests**

Run:

```bash
npm test
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/capture_session_commands.rs src/components/ScreenshotSession/index.tsx
git commit -m "test: guard lazy capture startup performance"
```

---

### Task 9: Manual Verification And Fallback Decision

**Files:**
- Modify only if manual verification finds issues.

- [ ] **Step 1: Build release app**

Run:

```bash
npm run tauri:build
```

Expected: build succeeds. Existing bundle id/notarization warnings are acceptable.

- [ ] **Step 2: Restart local release app**

Run:

```bash
osascript -e 'tell application "SnapLingo" to quit' || true
open -n /Users/gamilian/work/code/snaplingo/target/release/bundle/macos/SnapLingo.app
```

- [ ] **Step 3: Manual test normal desktop**

Verify:

- shortcut opens overlay quickly
- no white/black/page-switch flash
- hover window preselection follows mouse
- Esc cancels without showing settings page
- copy/save output matches selected region

- [ ] **Step 4: Manual test with SnapLingo settings visible**

Verify:

- settings window is selectable like any other app window
- closing settings keeps app running
- output captures settings window when selected

- [ ] **Step 5: Manual test fullscreen Space**

Verify:

- no Space/page switching
- overlay appears on current fullscreen Space
- Esc cancels cleanly

- [ ] **Step 6: Inspect logs**

Run:

```bash
rg -n "\\[capture-perf\\]" ~/Library/Logs/com.snaplingo.app/SnapLingo.log | tail -n 80
```

Expected:

- layout startup under roughly `100ms`
- no `view_base64_bytes` before overlay reveal
- freeze cost only occurs after selection confirmation

- [ ] **Step 7: Decide fallback mode**

If `NSWindowSharingNone` reliably excludes overlay, keep that path. If not, switch default to hide-overlay-before-region-capture fallback and keep sharing-none behind a small internal helper for future testing.

- [ ] **Step 8: Final full verification**

Run:

```bash
git diff --check
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri:build
```

Expected: all pass; only existing Tauri bundle id/notarization warnings appear.

- [ ] **Step 9: Commit final fixes**

```bash
git add .
git commit -m "fix: use lazy capture overlay startup"
```

---

## Rollout Notes

- Keep the current frozen-start path available until manual fullscreen and SnapLingo-window capture tests pass.
- Do not prewarm/reuse macOS capture windows; prior regressions showed this causes fullscreen Space/page-switch problems.
- Avoid sending full-screen base64 PNGs through IPC during selection startup.
- Treat `silent-screenshot-ocr` separately; it may still require immediate capture because there is no user selection overlay.
- If selection output mismatches because screen contents changed between selection and confirmation, document this as a known semantic tradeoff or add a later optional background-freeze mode.
