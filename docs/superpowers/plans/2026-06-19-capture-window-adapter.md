# Capture Window Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move capture-window runtime behavior from the Tauri Command Module into an Infrastructure Adapter while preserving screenshot behavior.

**Architecture:** Add `src-tauri/src/infrastructure/system/capture_window/` as the new Module. Commands remain the Tauri entry point but call a deeper Infrastructure Adapter for capture-window URL, bounds, hide/restore, settle delay, and window create/reuse behavior.

**Tech Stack:** Rust, Tauri, existing SnapLingo Domain types, existing Cargo tests.

---

### Task 1: Add Capture Window Planning Module

**Files:**
- Create: `src-tauri/src/infrastructure/system/capture_window/backend.rs`
- Create: `src-tauri/src/infrastructure/system/capture_window/mod.rs`
- Modify: `src-tauri/src/infrastructure/system/mod.rs`

- [x] **Step 1: Move pure planning helpers and tests**

Move these helpers out of `src-tauri/src/commands/capture_session_commands.rs`:

- `capture_window_url`
- `capture_window_url_with_session`
- `capture_snapshot_window_labels_to_hide`
- `capture_snapshot_window_labels_to_restore`
- `capture_snapshot_hide_settle_delay_ms`
- `capture_window_bounds`

Keep behavior byte-for-byte where possible.

- [x] **Step 2: Run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_window
```

Expected: new capture-window tests pass.

### Task 2: Add Tauri Capture Window Adapter

**Files:**
- Create: `src-tauri/src/infrastructure/system/capture_window/tauri.rs`
- Modify: `src-tauri/src/infrastructure/system/capture_window/mod.rs`
- Modify: `src-tauri/src/commands/capture_session_commands.rs`

- [x] **Step 1: Implement Adapter functions**

Add Infrastructure functions that receive `&tauri::AppHandle`:

- hide capture snapshot windows
- restore capture snapshot windows
- open capture window for session

The Adapter owns `WebviewWindowBuilder` and window visibility calls.

- [x] **Step 2: Replace Command Module calls**

Update Commands to call the Adapter and remove direct imports of:

- `LogicalPosition`
- `LogicalSize`
- `WebviewWindowBuilder`
- capture-window helper functions moved in Task 1

- [x] **Step 3: Run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml capture_session_commands capture_window
```

Expected: moved tests pass and command tests still compile.

### Task 3: Regression Verification

**Files:**
- No new files expected.

- [x] **Step 1: Run Rust library tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: Rust library tests pass.

Current status: passed. `cargo test --manifest-path src-tauri/Cargo.toml --lib`
ran 191 tests: 191 passed. The HTTP client tests now probe local TCP listener
availability before starting `mockito`, so restricted sandboxes skip the mock
server path while normal environments still execute the original assertions.

- [x] **Step 2: Run frontend tests related to capture window visibility**

Run:

```bash
npm test -- --run src/components/ScreenshotSession/captureWindowVisibility.test.ts src/components/ScreenshotSession/captureSessionLifecycle.test.ts
```

Expected: capture frontend tests pass.

- [x] **Step 3: Inspect diff**

Run:

```bash
git diff -- src-tauri/src/commands/capture_session_commands.rs src-tauri/src/infrastructure/system/capture_window src-tauri/src/infrastructure/system/mod.rs docs/superpowers/specs/2026-06-19-capture-window-adapter-design.md docs/superpowers/plans/2026-06-19-capture-window-adapter.md
```

Expected: diff only contains the phase 1 Adapter extraction and docs.

Current status: diff inspected. The implementation now includes the original phase 1
capture-window Adapter extraction plus follow-up deepening for Capture Session output
and pinned-window runtime behavior.

Follow-up architecture status: `capture_window` and `pinned_window` infrastructure
Adapters now own only Tauri/window work. Application service orchestration stays in
the Command Module helpers, so the infrastructure window Modules no longer depend on
Application service types.

Follow-up interface status: the infrastructure window Modules now re-export only the
functions needed across the Module interface. Internal planning helpers remain inside
their backend Modules with `pub(super)` visibility.
