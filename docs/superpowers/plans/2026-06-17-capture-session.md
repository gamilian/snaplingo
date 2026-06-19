# Capture Session Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current screenshot flow with a Snipaste-like Capture Session flow that freezes the screen once, lets the user select/edit on the frozen image, and outputs from the same frozen data.

**Architecture:** Introduce Capture Session as an Application-layer module backed by Infrastructure screenshot adapters and Domain-layer coordinate types. Commands stay thin, frontend manages interaction only, and all crop/output work is based on session id plus explicit logical/physical coordinate conversion.

**Tech Stack:** Rust, Tauri 2, React, TypeScript, existing `image` crate, existing `xcap`/`core-graphics` screenshot backends, existing OCR/translation services.

---

## Scope Check

This plan implements the first production slice of the architecture:

- Phase 1: Session foundation
- Phase 2: Frozen selection UI
- Phase 3: Output pipeline for copy/save/OCR hooks

Multi-monitor, DPI hardening, window detection, magnifier, color picking, and advanced pin/paste are intentionally left as follow-up phases. The new types and interfaces are shaped so those phases can be added without replacing the first slice.

## File Structure

### Files to Create

- `src-tauri/src/application/services/capture_session_service.rs`
  Owns Capture Session lifecycle, in-memory session storage, and session cleanup.

- `src-tauri/src/application/services/image_composition_service.rs`
  Crops frozen images and prepares output image bytes. First version supports single-monitor crops without annotations.

- `src-tauri/src/application/services/capture_output_service.rs`
  Handles save/copy/pin output actions. First version wires save and returns TODO-friendly copy/pin errors if clipboard/pin backend is not ready.

- `src-tauri/src/commands/capture_session_commands.rs`
  Tauri command layer for create/cancel/render/output/OCR operations.

- `src-tauri/src/application/services/capture_session_service_test.rs`
  Unit tests for session lifecycle and coordinate conversion.

- `src-tauri/src/application/services/image_composition_service_test.rs`
  Unit tests for crop behavior.

- `src/components/ScreenshotSession/index.tsx`
  New React workflow component for frozen screenshot sessions.

- `src/components/ScreenshotSession/types.ts`
  Frontend session and coordinate types.

- `src/components/ScreenshotSession/selection.ts`
  Pure frontend selection helpers.

### Files to Modify

- `src-tauri/src/domain/capture.rs`
  Add Capture Session domain types, explicit coordinate types, output action, and annotation placeholder types.

- `src-tauri/src/application/services/mod.rs`
  Export new services.

- `src-tauri/src/application/mod.rs`
  Export new services.

- `src-tauri/src/lib.rs`
  Add services to `AppState`, initialize them, and register commands.

- `src-tauri/src/commands/mod.rs`
  Export capture session commands.

- `src-tauri/src/infrastructure/system/screenshot/backend.rs`
  Extend backend trait with monitor snapshot support while keeping existing methods temporarily.

- `src-tauri/src/infrastructure/system/screenshot/macos.rs`
  Implement single-monitor session snapshot using existing `CGDisplay::main()` path first.

- `src-tauri/src/infrastructure/system/screenshot/windows.rs`
  Implement single-monitor session snapshot using existing xcap primary-monitor path first.

- `src-tauri/src/infrastructure/system/screenshot/linux.rs`
  Implement single-monitor session snapshot using existing xcap primary-monitor path first.

- `src/App.tsx`
  Mount `ScreenshotSession` instead of or before the old `ScreenshotWorkflow`.

- `src/components/ScreenshotWorkflow/index.tsx`
  Keep temporarily as fallback; remove once `ScreenshotSession` covers screenshot/OCR/translate modes.

### Files to Defer

- `src/components/ScreenshotCapture/index.tsx`
  Leave in place until `ScreenshotSession` replaces it, then delete in a cleanup task.

- `src/components/ScreenshotEditor/index.tsx`
  Reuse minimally or bypass in first slice. Full annotation editor belongs in a later plan.

---

## Task 1: Add Domain Types for Capture Session

**Files:**
- Modify: `src-tauri/src/domain/capture.rs`
- Test: compile via `cargo test`

- [ ] **Step 1: Add explicit coordinate and session types**

Add the following types without removing existing capture types:

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LogicalRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PhysicalRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CaptureSessionId(pub String);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MonitorSnapshotView {
    pub id: String,
    pub logical_bounds: LogicalRect,
    pub physical_bounds: PhysicalRect,
    pub scale_factor: f64,
    pub image_base64: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CaptureSessionView {
    pub id: CaptureSessionId,
    pub monitors: Vec<MonitorSnapshotView>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CaptureOutputAction {
    Copy,
    Save { path: String },
    Pin,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AnnotationCommand {
    pub kind: String,
}
```

- [ ] **Step 2: Run compile check**

Run: `cargo test -p snaplingo --lib domain::capture`

Expected: PASS or no matching unit tests, with no compile errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/domain/capture.rs
git commit -m "feat(capture): add capture session domain types"
```

---

## Task 2: Extend ScreenshotBackend with Session Snapshot Interface

**Files:**
- Modify: `src-tauri/src/infrastructure/system/screenshot/backend.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/macos.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/windows.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/linux.rs`

- [ ] **Step 1: Add infrastructure snapshot structs**

In `backend.rs`, add backend-owned snapshot types that use raw PNG bytes:

```rust
#[derive(Debug, Clone)]
pub struct MonitorSnapshot {
    pub id: String,
    pub logical_bounds: crate::domain::capture::LogicalRect,
    pub physical_bounds: crate::domain::capture::PhysicalRect,
    pub scale_factor: f64,
    pub png_data: Vec<u8>,
}
```

Extend `ScreenshotBackend`:

```rust
async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError>;
```

Keep `capture_full_screen` and `capture_region` for compatibility.

- [ ] **Step 2: Implement single-monitor adapters**

For macOS, use the existing main-display capture path. For Windows/Linux, wrap the existing xcap primary-monitor path. Set:

- `id`: `"primary"` for the first slice
- `logical_bounds`: `0,0,width/scale,height/scale`
- `physical_bounds`: `0,0,width,height`
- `scale_factor`: platform-reported scale if available, otherwise `1.0`

- [ ] **Step 3: Run backend compile check**

Run: `cargo check -p snaplingo`

Expected: SUCCESS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/infrastructure/system/screenshot
git commit -m "feat(capture): add monitor snapshot backend interface"
```

---

## Task 3: Implement CaptureSessionService

**Files:**
- Create: `src-tauri/src/application/services/capture_session_service.rs`
- Create: `src-tauri/src/application/services/capture_session_service_test.rs`
- Modify: `src-tauri/src/application/services/mod.rs`

- [ ] **Step 1: Write failing lifecycle test**

Create a mock backend that returns one 10x10 PNG snapshot. Test:

```rust
#[tokio::test]
async fn create_session_stores_snapshot_and_returns_view() {
    let service = CaptureSessionService::new(Arc::new(MockScreenshotBackend::new()));

    let view = service.create_session().await.unwrap();

    assert_eq!(view.monitors.len(), 1);
    assert!(service.has_session(&view.id));
}
```

- [ ] **Step 2: Run test and verify failure**

Run: `cargo test -p snaplingo capture_session_service --lib`

Expected: FAIL because `CaptureSessionService` does not exist.

- [ ] **Step 3: Implement service**

Implement:

```rust
pub struct CaptureSessionService {
    screenshot_backend: Arc<dyn ScreenshotBackend>,
    sessions: Arc<Mutex<HashMap<CaptureSessionId, CaptureSession>>>,
}
```

Methods:

- `new(screenshot_backend)`
- `create_session() -> Result<CaptureSessionView>`
- `cancel_session(id) -> Result<()>`
- `get_session(id) -> Result<CaptureSession>`
- `has_session(id) -> bool`

Use UUID-like ids. If the repo already has `uuid`, use it. If not, use timestamp plus atomic counter to avoid adding a dependency in this task.

In `src-tauri/src/application/services/mod.rs`, declare the test module:

```rust
#[cfg(test)]
mod capture_session_service_test;
```

- [ ] **Step 4: Run lifecycle tests**

Run: `cargo test -p snaplingo capture_session_service --lib`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/services/capture_session_service.rs
git add src-tauri/src/application/services/capture_session_service_test.rs
git add src-tauri/src/application/services/mod.rs
git commit -m "feat(capture): add capture session lifecycle service"
```

---

## Task 4: Implement Logical-to-Physical Conversion

**Files:**
- Modify: `src-tauri/src/application/services/capture_session_service.rs`
- Modify: `src-tauri/src/application/services/capture_session_service_test.rs`

- [ ] **Step 1: Write failing coordinate test**

Test a monitor with scale factor 2:

```rust
#[test]
fn converts_logical_rect_to_physical_rect() {
    let monitor = monitor_with_scale(2.0);
    let logical = LogicalRect { x: 10.0, y: 20.0, width: 30.0, height: 40.0 };

    let physical = logical_to_physical(&logical, &monitor).unwrap();

    assert_eq!(physical.x, 20);
    assert_eq!(physical.y, 40);
    assert_eq!(physical.width, 60);
    assert_eq!(physical.height, 80);
}
```

- [ ] **Step 2: Run test and verify failure**

Run: `cargo test -p snaplingo converts_logical_rect_to_physical_rect --lib`

Expected: FAIL because conversion helper does not exist.

- [ ] **Step 3: Implement conversion helper**

Add a private helper first. Do not expose it through commands yet.

Rules:

- clamp rect to monitor bounds
- reject zero width/height
- round outward so selected pixels are not accidentally lost

- [ ] **Step 4: Run tests**

Run: `cargo test -p snaplingo capture_session_service --lib`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/services/capture_session_service.rs
git add src-tauri/src/application/services/capture_session_service_test.rs
git commit -m "feat(capture): convert logical selection to physical pixels"
```

---

## Task 5: Implement ImageCompositionService Cropping

**Files:**
- Create: `src-tauri/src/application/services/image_composition_service.rs`
- Create: `src-tauri/src/application/services/image_composition_service_test.rs`
- Modify: `src-tauri/src/application/services/mod.rs`

- [ ] **Step 1: Write failing crop test**

Create a 4x4 test PNG with known pixels. Crop a 2x2 rect and assert the output dimensions.

```rust
#[test]
fn crops_png_to_physical_rect() {
    let png = make_test_png(4, 4);
    let rect = PhysicalRect { x: 1, y: 1, width: 2, height: 2 };

    let cropped = ImageCompositionService::new().crop_png(&png, &rect).unwrap();

    assert_png_dimensions(&cropped, 2, 2);
}
```

- [ ] **Step 2: Run test and verify failure**

Run: `cargo test -p snaplingo image_composition_service --lib`

Expected: FAIL because service does not exist.

- [ ] **Step 3: Implement crop**

Use the existing `image` crate:

- decode PNG from memory
- crop immutable view
- encode as PNG

Do not implement annotations in this task.

In `src-tauri/src/application/services/mod.rs`, declare the test module:

```rust
#[cfg(test)]
mod image_composition_service_test;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p snaplingo image_composition_service --lib`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/services/image_composition_service.rs
git add src-tauri/src/application/services/image_composition_service_test.rs
git add src-tauri/src/application/services/mod.rs
git commit -m "feat(capture): crop frozen screenshots from session data"
```

---

## Task 6: Add Capture Session Commands

**Files:**
- Create: `src-tauri/src/commands/capture_session_commands.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add commands**

Implement:

```rust
#[tauri::command]
pub async fn create_capture_session(
    state: State<'_, crate::AppState>,
) -> Result<CaptureSessionView, String>

#[tauri::command]
pub async fn cancel_capture_session(
    session_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String>

#[tauri::command]
pub async fn render_capture_output(
    session_id: String,
    rect: LogicalRect,
    state: State<'_, crate::AppState>,
) -> Result<String, String>
```

`render_capture_output` returns base64 PNG for the first slice.

- [ ] **Step 2: Wire AppState**

Add:

- `capture_session_service: Arc<CaptureSessionService>`
- `image_composition_service: Arc<ImageCompositionService>`

Initialize them next to existing `capture_service`.

- [ ] **Step 3: Register commands**

Register new commands in `tauri::generate_handler!`.

- [ ] **Step 4: Compile**

Run: `cargo check -p snaplingo`

Expected: SUCCESS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/capture_session_commands.rs
git add src-tauri/src/commands/mod.rs
git add src-tauri/src/lib.rs
git commit -m "feat(capture): expose capture session commands"
```

---

## Task 7: Build Frontend ScreenshotSession Selection Helpers

**Files:**
- Create: `src/components/ScreenshotSession/types.ts`
- Create: `src/components/ScreenshotSession/selection.ts`

- [ ] **Step 1: Add frontend types**

Mirror command payloads:

```ts
export interface LogicalRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface MonitorSnapshotView {
  id: string;
  logical_bounds: LogicalRect;
  physical_bounds: { x: number; y: number; width: number; height: number };
  scale_factor: number;
  image_base64: string;
}

export interface CaptureSessionView {
  id: string;
  monitors: MonitorSnapshotView[];
}
```

- [ ] **Step 2: Add selection helper**

Implement:

```ts
export function normalizeSelection(start: Point, current: Point): LogicalRect {
  return {
    x: Math.min(start.x, current.x),
    y: Math.min(start.y, current.y),
    width: Math.abs(current.x - start.x),
    height: Math.abs(current.y - start.y),
  };
}
```

- [ ] **Step 3: Run TypeScript check**

Run: `npm run build`

Expected: SUCCESS or existing unrelated errors only. If unrelated errors exist, capture them in the task notes before continuing.

- [ ] **Step 4: Commit**

```bash
git add src/components/ScreenshotSession/types.ts
git add src/components/ScreenshotSession/selection.ts
git commit -m "feat(capture): add frontend capture session types"
```

---

## Task 8: Build Frozen ScreenshotSession UI

**Files:**
- Create: `src/components/ScreenshotSession/index.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Implement session state machine**

States:

- `idle`
- `loading`
- `selecting`
- `preview`
- `error`

On screenshot hotkey:

1. call `create_capture_session`
2. render returned monitor image as visible background
3. enter `selecting`

- [ ] **Step 2: Implement visible frozen background**

Render the first monitor image:

```tsx
<img
  src={`data:image/png;base64,${monitor.image_base64}`}
  className="absolute inset-0 h-full w-full object-fill"
  draggable={false}
/>
```

Do not use `opacity-0`.

- [ ] **Step 3: Implement drag selection**

Use `normalizeSelection`. Show:

- dim overlay
- selected area border
- size label
- cursor crosshair

- [ ] **Step 4: Implement cancel**

On `Escape`, call `cancel_capture_session` and return to `idle`.

- [ ] **Step 5: Wire into App**

Mount `ScreenshotSession`. Keep old `ScreenshotWorkflow` disabled or behind a temporary fallback comment to avoid double hotkey handling.

- [ ] **Step 6: Run frontend build**

Run: `npm run build`

Expected: SUCCESS.

- [ ] **Step 7: Commit**

```bash
git add src/components/ScreenshotSession/index.tsx
git add src/App.tsx
git commit -m "feat(capture): render frozen screenshot session UI"
```

---

## Task 9: Render Selected Output from Frozen Session

**Files:**
- Modify: `src/components/ScreenshotSession/index.tsx`

- [ ] **Step 1: Call render command after mouse up**

When selection is larger than 10x10 logical pixels, call:

```ts
const base64 = await invoke<string>("render_capture_output", {
  sessionId,
  rect: selectedRect,
});
```

- [ ] **Step 2: Show preview in original position**

First slice may show a lightweight preview border and toolbar at the selected rect. Do not center the image in the viewport.

- [ ] **Step 3: Add keyboard copy placeholder**

On `Enter` or `Cmd/Ctrl+C`, call output command once Task 10 exists. Until then, keep the UI in preview state.

- [ ] **Step 4: Run build**

Run: `npm run build`

Expected: SUCCESS.

- [ ] **Step 5: Commit**

```bash
git add src/components/ScreenshotSession/index.tsx
git commit -m "feat(capture): preview selected area from frozen session"
```

---

## Task 10: Implement Save Output Action

**Files:**
- Create: `src-tauri/src/application/services/capture_output_service.rs`
- Modify: `src-tauri/src/application/services/mod.rs`
- Modify: `src-tauri/src/commands/capture_session_commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing output test**

Use a temp path and assert file exists after save.

```rust
#[tokio::test]
async fn save_output_writes_png_to_path() {
    let service = CaptureOutputService::new();
    let path = temp_png_path();
    let png = make_test_png(2, 2);

    service.save_png(&png, &path).await.unwrap();

    assert!(path.exists());
}
```

- [ ] **Step 2: Run test and verify failure**

Run: `cargo test -p snaplingo capture_output_service --lib`

Expected: FAIL because service does not exist.

- [ ] **Step 3: Implement save action**

Implement:

- `save_png(data: &[u8], path: &Path) -> Result<PathBuf>`
- parent directory validation
- write bytes using `tokio::fs` or existing project style

- [ ] **Step 4: Add command**

Add:

```rust
#[tauri::command]
pub async fn output_capture(
    session_id: String,
    rect: LogicalRect,
    action: CaptureOutputAction,
    state: State<'_, crate::AppState>,
) -> Result<(), String>
```

First slice supports `Save`. Return clear errors for `Copy` and `Pin` until their adapters are ready.

- [ ] **Step 5: Run tests and compile**

Run:

```bash
cargo test -p snaplingo capture_output_service --lib
cargo check -p snaplingo
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/application/services/capture_output_service.rs
git add src-tauri/src/application/services/mod.rs
git add src-tauri/src/commands/capture_session_commands.rs
git add src-tauri/src/lib.rs
git commit -m "feat(capture): save output from frozen capture session"
```

---

## Task 11: Wire OCR Mode to Capture Session

**Files:**
- Modify: `src-tauri/src/commands/capture_session_commands.rs`
- Modify: `src/components/ScreenshotSession/index.tsx`

- [ ] **Step 1: Add OCR command**

Add:

```rust
#[tauri::command]
pub async fn run_capture_ocr(
    session_id: String,
    rect: LogicalRect,
    state: State<'_, crate::AppState>,
) -> Result<OcrResult, String>
```

Implementation:

- crop original frozen image
- call existing OCR coordinator/service
- return OCR result

- [ ] **Step 2: Wire frontend mode payload**

Track hotkey payload:

- `screenshot`
- `screenshot-ocr`
- `screenshot-translate`

For `screenshot-ocr`, after mouse up call `run_capture_ocr`.

- [ ] **Step 3: Compile**

Run:

```bash
cargo check -p snaplingo
npm run build
```

Expected: SUCCESS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/capture_session_commands.rs
git add src/components/ScreenshotSession/index.tsx
git commit -m "feat(capture): run OCR from capture session original image"
```

---

## Task 12: Manual Verification Pass

**Files:**
- No code changes unless issues are found.

- [ ] **Step 1: Run full checks**

Run:

```bash
cargo test -p snaplingo
npm run build
```

Expected: PASS.

- [ ] **Step 2: Run app**

Run: `npm run tauri:dev`

Expected: App starts and global screenshot hotkey triggers the new frozen session UI.

- [ ] **Step 3: Verify scenarios**

Manual checks:

- screenshot hotkey shows visible frozen screen
- dragging shows selection rectangle
- `Esc` cancels and clears session
- after selecting, preview comes from frozen image
- changing screen contents after session creation does not change selected output
- save writes the selected image
- OCR mode uses selected original image

- [ ] **Step 4: Record follow-up issues**

Create follow-up notes for:

- multi-monitor and DPI
- copy clipboard backend
- pin/paste window backend
- annotations
- window detection

- [ ] **Step 5: Commit fixes if any**

```bash
git add <changed files>
git commit -m "fix(capture): address capture session verification issues"
```

---

## Follow-Up Plan Outline

After this plan lands, create separate plans for:

1. Multi-monitor and mixed-DPI capture sessions
2. Clipboard image copy and successful screenshot history
3. Basic pin/paste windows
4. Annotation command model and image composition
5. Window detection and Snipaste-style `Tab` mode
6. Magnifier, color picker, and pixel-level keyboard control
