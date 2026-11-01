# Architecture Deepening Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the remaining high-value architecture review work by moving the Capture Session contract inward, isolating Tesseract platform mechanics behind an adapter, and introducing a Pinned Image Runtime that owns state-plus-window workflows.

**Architecture:** Execute three independent backend refactors in dependency order. First, make Capture Session own its portable data and source interface while Infrastructure only implements platform adapters. Second, inject a Tesseract engine adapter into the Tesseract Provider so executable discovery, native calls, and OS paths stay in Infrastructure. Third, deepen Pinned Image into a runtime module that coordinates in-memory state, image output, and window effects behind one interface. Preserve all Tauri command names and frontend-visible payloads.

**Tech Stack:** Rust, Tauri 2, async-trait, React/TypeScript adapters, Cargo tests, Vitest.

---

## Scope

In scope:

- Move portable screenshot/Capture Session types out of `infrastructure/system/screenshot`.
- Define the screenshot source port at the Capture Session application boundary.
- Keep macOS, Windows, and Linux screenshot implementations as Infrastructure adapters.
- Move Tesseract executable discovery, process/native mechanics, and OS paths into an Infrastructure adapter.
- Inject the Tesseract adapter through Application Composition.
- Add `PinnedImageRuntime` to own Pinned Image state transitions plus window side effects.
- Make Pinned Image commands thin adapters over the runtime interface.
- Route Capture Session pin output through `PinnedImageRuntime` instead of opening windows directly in commands.
- Update architecture documentation after each seam changes.

Out of scope:

- Do not validate Windows or Linux runtime behavior; this plan verifies architecture and current-platform behavior only.
- Do not rename Tauri commands or change IPC payload shapes.
- Do not introduce `ProviderStore<P>` or merge OCR and Translation Coordinator storage.
- Do not rename the entire `application/services` directory.
- Do not refactor Capture Workspace in the same branch.
- Do not redesign Pinned Image behavior or add transaction rollback semantics for window failures.
- Do not modify the four existing untracked architecture plan files except when explicitly executing their historical plans.

## Architectural Invariants

- Application and Domain modules must not import portable Capture types from Infrastructure.
- Infrastructure adapters may depend inward on Domain types and Application ports.
- Composition selects concrete adapters; Commands do not construct adapters.
- Provider modules own OCR semantics; Infrastructure owns executable discovery and native/system mechanics.
- Pinned Image commands translate IPC arguments and errors only; workflow ordering lives in `PinnedImageRuntime`.
- Existing command names, serialized fields, Capture modes, and Pinned Image group behavior remain unchanged.

## Execution Order

1. Establish a clean baseline.
2. Phase A: move the Capture Session contract inward.
3. Phase B: isolate Tesseract mechanics.
4. Phase C: deepen Pinned Image Runtime.
5. Update architecture documents and run full verification.
6. Reassess Capture Workspace separately using the gate at the end of this plan.

Phase A must land before Phase C because both touch Capture runtime composition. Phase B is logically independent and may be developed in a separate worktree, but merge it after Phase A to minimize composition conflicts.

---

## Target File Structure

Create for Phase A:

- `src-tauri/src/application/services/capture_session_source.rs` - inward Capture Session source port.
- `src-tauri/src/infrastructure/system/screenshot/geometry.rs` - platform-adapter geometry mapping helpers currently mixed with the contract.

Modify for Phase A:

- `src-tauri/src/domain/capture.rs`
- `src-tauri/src/application/services/mod.rs`
- `src-tauri/src/application/services/capture_session_service.rs`
- `src-tauri/src/application/services/capture_session_render.rs`
- `src-tauri/src/application/services/capture_session_runtime.rs`
- `src-tauri/src/application/services/capture_session_service_test.rs`
- `src-tauri/src/infrastructure/system/screenshot/mod.rs`
- `src-tauri/src/infrastructure/system/screenshot/backend.rs`
- `src-tauri/src/infrastructure/system/screenshot/macos.rs`
- `src-tauri/src/infrastructure/system/screenshot/windows.rs`
- `src-tauri/src/infrastructure/system/screenshot/linux.rs`
- `src-tauri/src/infrastructure/system/screenshot/xcap_common.rs`
- `src-tauri/src/infrastructure/system/mod.rs`
- `src-tauri/src/composition/capture_runtime.rs`

Create for Phase B:

- `src-tauri/src/application/providers/ocr/tesseract_engine.rs` - Tesseract engine port.
- `src-tauri/src/infrastructure/system/ocr/tesseract.rs` - concrete Tesseract adapter.

Modify for Phase B:

- `src-tauri/src/application/providers/ocr/mod.rs`
- `src-tauri/src/application/providers/ocr/impls/tesseract.rs`
- `src-tauri/src/infrastructure/system/ocr/mod.rs`
- `src-tauri/src/infrastructure/system/mod.rs`
- `src-tauri/src/composition/provider_runtime.rs`
- `src-tauri/src/commands/ocr_commands.rs`

Create for Phase C:

- `src-tauri/src/application/services/pinned_image_runtime.rs` - deep Pinned Image workflow module and host port.
- `src-tauri/src/application/services/pinned_image_runtime_test.rs` - workflow tests through the runtime interface.
- `src-tauri/src/infrastructure/system/pinned_window/runtime_host.rs` - Tauri implementation of the Pinned Image host port.

Modify for Phase C:

- `src-tauri/src/application/services/mod.rs`
- `src-tauri/src/application/mod.rs`
- `src-tauri/src/infrastructure/system/pinned_window/mod.rs`
- `src-tauri/src/composition/capture_runtime.rs`
- `src-tauri/src/composition.rs`
- `src-tauri/src/app_state.rs`
- `src-tauri/src/commands/pinned_image_commands.rs`
- `src-tauri/src/commands/capture_session_commands.rs`
- `src-tauri/src/app_actions.rs`

Documentation:

- `CONTEXT.md`
- `ARCHITECTURE.md`
- `docs/architecture/runtime-map.md`

---

## Task 0: Establish a Clean Baseline

**Files:** none.

- [ ] **Step 1: Inspect repository state**

Run:

```bash
git status --short --branch
git worktree list
```

Expected: `master` contains the merged App Action Dispatch work. Preserve these existing untracked files:

```text
docs/superpowers/plans/2026-07-10-app-action-dispatch.md
docs/superpowers/plans/2026-07-10-architecture-followups.md
docs/superpowers/plans/2026-07-10-capture-ipc-types.md
docs/superpowers/plans/2026-07-10-capture-module-consolidation.md
```

- [ ] **Step 2: Create an isolated worktree**

Create a feature branch such as:

```bash
git worktree add .worktrees/architecture-deepening-phase-2 -b codex/architecture-deepening-phase-2 master
```

- [ ] **Step 3: Run backend baseline**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS. Record any pre-existing failure before editing.

- [ ] **Step 4: Run frontend baseline**

Run:

```bash
npm test
npm run build
```

Expected: PASS; the existing production chunk-size warning is non-failing.

---

## Phase A: Capture Session Owns the Portable Contract

### Task 1: Characterize the Existing Capture Source Interface

**Files:**

- Modify: `src-tauri/src/application/services/capture_session_service_test.rs`
- Modify: `src-tauri/src/application/services/capture_session_runtime.rs`

- [ ] **Step 1: Add an interface-level behavior test**

Add or strengthen a fake source test proving that Capture Session can:

- create a session from monitor snapshots;
- obtain window candidates and cursor data;
- hydrate snapshots from monitor layouts;
- capture a selected physical region.

The test must exercise `CaptureSessionService` behavior, not Infrastructure helper functions.

- [ ] **Step 2: Run focused tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_session
```

Expected: PASS before moving ownership.

- [ ] **Step 3: Commit the characterization guard**

```bash
git add src-tauri/src/application/services/capture_session_service_test.rs src-tauri/src/application/services/capture_session_runtime.rs
git commit -m "test(capture): characterize capture session source contract"
```

### Task 2: Move Portable Capture Data Into Domain

**Files:**

- Modify: `src-tauri/src/domain/capture.rs`
- Modify: all current users of `MonitorSnapshot`, `MonitorLayout`, `CapturedCursor`, `WindowCandidate`, and `ScreenRegion`

- [ ] **Step 1: Move the portable types**

Move these types from `infrastructure/system/screenshot/backend.rs` into `domain/capture.rs`:

```rust
pub struct ScreenRegion { ... }
pub struct MonitorSnapshot { ... }
pub struct MonitorLayout { ... }
pub struct CapturedCursor { ... }
pub struct WindowCandidate { ... }
```

Keep their current field shapes and visibility. Do not add serialization derives unless an IPC consumer requires them.

- [ ] **Step 2: Update imports without moving the trait yet**

Update Application, Infrastructure, and test imports to use `crate::domain::capture::*` for these data types.

- [ ] **Step 3: Verify dependency direction**

Run:

```bash
rg -n "infrastructure::system::screenshot::.*(MonitorSnapshot|MonitorLayout|CapturedCursor|WindowCandidate|ScreenRegion)" src-tauri/src
```

Expected: no Application or Domain import of those types through Infrastructure.

- [ ] **Step 4: Run focused and full backend tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_session
cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/domain/capture.rs src-tauri/src/application src-tauri/src/infrastructure
git commit -m "refactor(capture): move portable screenshot data into domain"
```

### Task 3: Move the Source Port Into Capture Session

**Files:**

- Create: `src-tauri/src/application/services/capture_session_source.rs`
- Modify: `src-tauri/src/application/services/mod.rs`
- Modify: `src-tauri/src/application/services/capture_session_service.rs`
- Modify: screenshot adapters and tests

- [ ] **Step 1: Define the inward port**

Create `CaptureSessionSource` with the existing required capabilities:

```rust
#[async_trait::async_trait]
pub trait CaptureSessionSource: Send + Sync {
    async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError>;
    async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>, AppError>;
    async fn capture_window_candidates(
        &self,
        monitors: &[MonitorSnapshot],
    ) -> Result<Vec<WindowCandidate>, AppError>;
    async fn capture_cursor(
        &self,
        monitors: &[MonitorSnapshot],
    ) -> Result<Option<CapturedCursor>, AppError>;
    fn current_cursor_position(
        &self,
        monitors: &[MonitorSnapshot],
    ) -> Result<Option<LogicalPoint>, AppError>;
    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError>;
}
```

Retain the current default implementations for optional candidate/cursor behavior.

- [ ] **Step 2: Make Capture Session depend on the port**

Change `CaptureSessionService` to own:

```rust
source: Arc<dyn CaptureSessionSource>
```

Rename local variables from `screenshot_backend` to `source` where the new vocabulary improves locality.

- [ ] **Step 3: Make platform adapters implement the inward port**

Update `MacOSScreenshotBackend`, `WindowsScreenshotBackend`, `LinuxScreenshotBackend`, and test fakes to implement `CaptureSessionSource`.

- [ ] **Step 4: Move adapter-only helpers out of the old contract file**

Move physical/logical geometry mapping helpers into `infrastructure/system/screenshot/geometry.rs`. Keep PNG encoding helpers in Infrastructure. Delete `backend.rs` once it no longer owns a contract or cohesive helper set.

- [ ] **Step 5: Rename the factory**

Replace:

```rust
get_screenshot_backend() -> Arc<dyn ScreenshotBackend>
```

with:

```rust
get_capture_session_source() -> Arc<dyn CaptureSessionSource>
```

Only Application Composition should call this factory.

- [ ] **Step 6: Verify the seam**

Run:

```bash
rg -n "ScreenshotBackend|get_screenshot_backend" src-tauri/src
rg -n "infrastructure::system::screenshot" src-tauri/src/application src-tauri/src/domain
```

Expected:

- no `ScreenshotBackend` or `get_screenshot_backend` references;
- Application does not import the screenshot Infrastructure module;
- Infrastructure imports `CaptureSessionSource` and implements it.

- [ ] **Step 7: Run tests and formatting**

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml capture_session
cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src
git commit -m "refactor(capture): move screenshot source port into capture session"
```

---

## Phase B: Tesseract Uses an Injected Infrastructure Adapter

### Task 4: Define the Tesseract Engine Port

**Files:**

- Create: `src-tauri/src/application/providers/ocr/tesseract_engine.rs`
- Modify: `src-tauri/src/application/providers/ocr/mod.rs`
- Modify: `src-tauri/src/application/providers/ocr/impls/tesseract.rs`

- [ ] **Step 1: Add a Provider-facing engine interface**

Define a crate-visible interface that exposes capabilities, not process details:

```rust
pub(crate) trait TesseractEngine: Send + Sync {
    fn available_languages(&self) -> crate::Result<Vec<String>>;
    fn recognize(&self, image_data: &[u8], language: Option<&str>) -> crate::Result<String>;
}
```

The interface must not expose executable paths, `Command`, raw Tesseract handles, or OS branches.

- [ ] **Step 2: Inject the engine into the Provider**

Change `TesseractProvider` to own:

```rust
engine: Arc<dyn TesseractEngine>
```

`recognize()` should:

1. ask the engine for available languages;
2. apply Provider-owned language selection rules;
3. call `engine.recognize(...)`;
4. return `OcrResult`.

- [ ] **Step 3: Add fake-engine Provider tests**

Test through `OcrProvider::recognize` that:

- explicit language hints are mapped correctly;
- the default prefers `chi_sim+eng` when both are available;
- engine errors are propagated with stable context;
- Provider metadata remains unchanged.

- [ ] **Step 4: Run focused tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml tesseract
```

The code may temporarily fail to compile until the adapter is added; do not commit a broken intermediate state.

### Task 5: Implement the Tesseract Infrastructure Adapter

**Files:**

- Create: `src-tauri/src/infrastructure/system/ocr/tesseract.rs`
- Modify: `src-tauri/src/infrastructure/system/ocr/mod.rs`
- Modify: `src-tauri/src/infrastructure/system/mod.rs`

- [ ] **Step 1: Move system mechanics into the adapter**

Move these responsibilities out of the Provider implementation:

- `PATH` probing and fallback executable paths;
- platform-specific binary names and install paths;
- `tesseract --version` and `--list-langs` process calls;
- image normalization into a Tesseract frame;
- the global native Tesseract lock;
- `tesseract::Tesseract` construction, frame assignment, recognition, and text extraction.

- [ ] **Step 2: Keep mapping responsibility explicit**

Provider language-code mapping and default-language policy stay in the Provider module. Parsing CLI language output and executable discovery stay in Infrastructure.

- [ ] **Step 3: Make the OCR Infrastructure module cross-platform in shape**

`infrastructure/system/ocr` must exist on every target because Tesseract is cross-platform. Keep only the macOS Vision adapter behind `#[cfg(target_os = "macos")]`.

Expose:

```rust
pub(crate) fn get_tesseract_engine() -> Arc<dyn TesseractEngine>
```

- [ ] **Step 4: Move mechanics tests with the implementation**

Move candidate-path, fallback-deduplication, CLI output parsing, invalid-image, and grayscale-frame tests into the Infrastructure adapter test module.

- [ ] **Step 5: Rewire Application Composition**

In `composition/provider_runtime.rs`:

```rust
let tesseract_provider = TesseractProvider::new(get_tesseract_engine());
```

Update command tests to use a fake engine instead of constructing a system-backed Provider.

- [ ] **Step 6: Verify the seam**

Run:

```bash
rg -n "std::process::Command|fallback_executable|tesseract_binary_name|TESSERACT_OCR_LOCK" src-tauri/src/application/providers/ocr
rg -n "target_os" src-tauri/src/application/providers/ocr/impls/tesseract.rs
```

Expected: no process, executable-discovery, or OS mechanics remain in the Provider implementation.

- [ ] **Step 7: Run tests and commit**

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml tesseract
cargo test --manifest-path src-tauri/Cargo.toml
git add src-tauri/src
git commit -m "refactor(ocr): isolate tesseract system mechanics"
```

---

## Phase C: Pinned Image Runtime Owns Workflow Ordering

### Task 6: Characterize Command-Level Workflow Ordering

**Files:**

- Create or modify focused tests near `pinned_image_commands.rs`
- Modify: `src-tauri/src/application/services/pinned_image_service_test.rs` only when preserving state behavior

- [ ] **Step 1: Protect current behavior**

Add characterization coverage for:

- clipboard pin reopens the most recently closed recoverable image;
- a new clipboard pin opens a new window;
- closing marks the image recoverable before hiding its window;
- group switch updates state before applying window visibility changes;
- moving an image updates its group before hiding the moved window;
- Capture Session `Pin` output stores the PNG before opening its window.

Do not assert rollback after a window error because current behavior does not provide transactional rollback.

- [ ] **Step 2: Run focused tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml pinned_image
cargo test --manifest-path src-tauri/Cargo.toml capture_session
```

- [ ] **Step 3: Commit characterization tests**

```bash
git add src-tauri/src
git commit -m "test(pinned-image): characterize state and window workflows"
```

### Task 7: Add the Pinned Image Runtime and Host Port

**Files:**

- Create: `src-tauri/src/application/services/pinned_image_runtime.rs`
- Create: `src-tauri/src/application/services/pinned_image_runtime_test.rs`
- Create: `src-tauri/src/infrastructure/system/pinned_window/runtime_host.rs`
- Modify: related `mod.rs` files

- [ ] **Step 1: Define the host port**

Define a crate-visible `PinnedImageRuntimeHost` interface for window effects. It should express Pinned Image operations rather than Tauri primitives:

```rust
#[async_trait::async_trait]
pub(crate) trait PinnedImageRuntimeHost: Send + Sync {
    async fn open(&self, image: PinnedImageView) -> crate::Result<()>;
    async fn show_or_open(&self, image: PinnedImageView) -> crate::Result<()>;
    async fn hide(&self, image_id: String) -> crate::Result<()>;
    async fn toggle_all(&self) -> crate::Result<Option<bool>>;
    async fn apply_group_switch(
        &self,
        hide_image_ids: Vec<String>,
        show_image_ids: Vec<String>,
    ) -> crate::Result<()>;
    async fn hide_group(&self, image_ids: Vec<String>) -> crate::Result<()>;
    async fn close_group(&self, image_ids: Vec<String>) -> crate::Result<()>;
}
```

Use owned arguments so the Tauri adapter can dispatch safely to the main thread.

- [ ] **Step 2: Implement the Tauri host adapter**

`TauriPinnedImageRuntimeHost` owns `AppHandle`, implements the port, and delegates to existing `pinned_window` mechanics on the main thread.

Keep label generation, window sizing, visibility planning, and Tauri window construction inside `infrastructure/system/pinned_window`.

- [ ] **Step 3: Build the runtime around existing modules**

`PinnedImageRuntime` owns:

```rust
service: Arc<PinnedImageService>,
image_composition: Arc<ImageCompositionService>,
output: Arc<CaptureOutputService>,
host: Arc<dyn PinnedImageRuntimeHost>,
```

Expose workflow methods matching current command use cases:

- `pin_clipboard()`;
- `pin_png_and_open(png_data)`;
- `close(image_id)`;
- `get`, `remove`, `copy`, `replace_from_clipboard`, `save`;
- `toggle_visibility`;
- `switch_group`;
- `move_to_next_group`;
- `hide_group`;
- `destroy_group`.

The runtime may delegate pure state methods to `PinnedImageService`, but Commands must not need the service separately.

- [ ] **Step 4: Add runtime interface tests**

Use a recording fake host to verify state transition and host-call ordering. Test through `PinnedImageRuntime`, not Infrastructure functions.

- [ ] **Step 5: Run focused tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml pinned_image_runtime
cargo test --manifest-path src-tauri/Cargo.toml pinned_image
```

### Task 8: Rewire Composition, AppState, Commands, and App Actions

**Files:**

- Modify: `src-tauri/src/composition/capture_runtime.rs`
- Modify: `src-tauri/src/composition.rs`
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/commands/pinned_image_commands.rs`
- Modify: `src-tauri/src/commands/capture_session_commands.rs`
- Modify: `src-tauri/src/app_actions.rs`

- [ ] **Step 1: Construct the runtime in Composition**

Create `PinnedImageService`, `TauriPinnedImageRuntimeHost`, and `PinnedImageRuntime` in `build_capture_runtime`.

Change `CaptureRuntimeState` from:

```rust
pub pinned_images: Arc<PinnedImageService>
```

to:

```rust
pub pinned_images: Arc<PinnedImageRuntime>
```

- [ ] **Step 2: Thin Pinned Image commands**

Each Tauri command should only:

1. accept IPC arguments and `State<AppState>`;
2. call one runtime method;
3. map `AppError` to `String`.

Remove direct imports from `infrastructure::system::pinned_window` in Commands.

- [ ] **Step 3: Route Capture Session pin output through the runtime**

Keep `CaptureSessionOutput::Pin(Vec<u8>)` for this phase. Replace direct `PinnedImageService::pin_png_view` plus window opening with:

```rust
state.capture.pinned_images.pin_png_and_open(png_data).await
```

This removes Pinned Image window mechanics from `capture_session_commands.rs` without coupling `CaptureSessionRuntime` directly to Pinned Image.

- [ ] **Step 4: Update App Action helpers**

Adjust the Pin Clipboard and Switch Pinned Group action paths to call thin command helpers or the runtime interface without passing `AppHandle` for window orchestration.

- [ ] **Step 5: Verify the seam**

Run:

```bash
rg -n "infrastructure::system::pinned_window" src-tauri/src/commands src-tauri/src/app_actions.rs
rg -n "PinnedImageService" src-tauri/src/commands src-tauri/src/app_state.rs
```

Expected: no direct window adapter imports and no raw `PinnedImageService` exposure through AppState or Commands.

- [ ] **Step 6: Run tests and commit**

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml pinned_image
cargo test --manifest-path src-tauri/Cargo.toml capture_session
cargo test --manifest-path src-tauri/Cargo.toml
git add src-tauri/src
git commit -m "refactor(pinned-image): centralize state and window workflows"
```

---

## Task 9: Documentation and Final Verification

**Files:**

- Modify: `CONTEXT.md`
- Modify: `ARCHITECTURE.md`
- Modify: `docs/architecture/runtime-map.md`

- [ ] **Step 1: Update domain and architecture vocabulary**

Document:

- Capture Session owns the portable capture source port and data;
- screenshot Infrastructure modules are platform adapters only;
- Tesseract Provider owns language policy while the Tesseract adapter owns system mechanics;
- Pinned Image Runtime owns state-plus-window workflow ordering;
- Commands remain IPC adapters.

- [ ] **Step 2: Confirm deferred work remains explicit**

Retain the `ProviderStore<P>` deferral. Add a note that Capture Workspace interface deepening remains conditional and is not part of this backend phase.

- [ ] **Step 3: Run formatting and static checks**

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
git diff --check
```

- [ ] **Step 4: Run full backend verification**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all Rust unit and integration tests pass.

- [ ] **Step 5: Run frontend regression verification**

```bash
npm test
npm run build
```

Expected: frontend tests and production build pass with no IPC changes.

- [ ] **Step 6: Audit dependency direction**

```bash
rg -n "infrastructure::system::screenshot" src-tauri/src/application src-tauri/src/domain
rg -n "std::process::Command|target_os" src-tauri/src/application/providers/ocr/impls/tesseract.rs
rg -n "infrastructure::system::pinned_window" src-tauri/src/commands src-tauri/src/app_actions.rs
```

Expected: no output for all three checks.

- [ ] **Step 7: Review final diff and commit docs**

```bash
git status --short
git diff --stat master...HEAD
git diff --check master...HEAD
git add CONTEXT.md ARCHITECTURE.md docs/architecture/runtime-map.md
git commit -m "docs: record capture and pinned image runtime seams"
```

---

## Capture Workspace Gate

Do not automatically continue into a frontend controller rewrite after this plan.

Create a separate plan only when at least one condition is true:

- a new annotation tool requires edits across three or more workspace modules plus `useCaptureWorkspaceController`;
- gesture bugs repeatedly require tracing through state, refs, derived geometry, and action maps;
- `CaptureWorkspaceView` needs knowledge that belongs to host/editor workflow;
- tests cannot exercise a workflow without constructing a wide controller context.

Before planning that work, run a fresh architecture review of:

```text
useCaptureWorkspaceController.ts
useCaptureWorkspaceState.ts
captureWorkspaceHost.ts
captureWorkspacePointer.ts
captureWorkspaceKeyboard.ts
captureEditorRuntime.ts
CaptureWorkspaceView.tsx
```

The target is a smaller workflow interface and better locality, not a lower line count or fewer files.

---

## Success Criteria

- Capture Session Application modules import no screenshot types or traits from Infrastructure.
- All three platform screenshot adapters implement an inward `CaptureSessionSource` port.
- Tesseract Provider contains no executable discovery, process calls, native frame mechanics, or OS path branches.
- Application Composition injects the concrete Tesseract engine.
- Pinned Image commands contain no direct window adapter calls.
- `CaptureRuntimeState` exposes `PinnedImageRuntime`, not `PinnedImageService`.
- Capture Session pin output opens a Pinned Image through the runtime interface.
- All Tauri command names and serialized payloads are unchanged.
- Rust tests, frontend tests, production build, formatting, and `git diff --check` pass.
