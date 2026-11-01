# Result Window Deepening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox format for tracking.

**Goal:** Move Result Window payload policy, pending state, clipboard access, window opening, and delivery notification out of Commands into deep Application modules.

**Architecture:** Backend ResultWindowRuntime uses a serialized latest-request-wins slot and outward ports. Frontend Result Window Application runtime owns mode and workflow state; its View only renders and sends actions.

**Tech Stack:** Rust, Tauri 2, React, TypeScript, Vitest, Cargo tests.

---

### Task 1: Specify backend state-machine behavior with tests

**Files:**
- Create: src-tauri/src/application/result_window/mod.rs
- Create: src-tauri/src/application/result_window/runtime.rs
- Create: src-tauri/src/application/result_window/tests.rs
- Modify: src-tauri/src/application/mod.rs

- [ ] Write failing tests for open, take, latest-request-wins replacement, failed-open conditional cleanup, failed-notification retention, and concurrent newer request preservation.
- [ ] Verify RED because ResultWindowRuntime does not exist.
- [ ] Define fake window, clipboard, and notifier ports in tests.
- [ ] Commit tests with message test: specify result window runtime.

### Task 2: Implement backend runtime and owned ports

**Files:**
- Create: src-tauri/src/application/result_window/port.rs
- Modify: runtime.rs
- Modify: domain or application-local payload types as required

- [ ] Implement monotonically increasing request ids and a Mutex-protected Option slot.
- [ ] Treat notification as a wakeup hint.
- [ ] Implement atomic take-latest ownership transfer.
- [ ] Run focused tests and verify GREEN.
- [ ] Commit with message refactor: add result window runtime.

### Task 3: Move Tauri and clipboard mechanics outward

**Files:**
- Create: src-tauri/src/infrastructure/system/result_window/runtime_host.rs
- Create or extend: src-tauri/src/infrastructure/system/clipboard/
- Modify: src-tauri/src/composition.rs
- Modify: src-tauri/src/app_state.rs

- [ ] Implement Result Window ports with Tauri and arboard in Infrastructure.
- [ ] Construct and inject ResultWindowRuntime in Composition.
- [ ] Add adapter contract tests.
- [ ] Run focused Cargo tests.
- [ ] Commit with message refactor: wire result window adapters.

### Task 4: Thin backend commands and App Actions

**Files:**
- Modify: src-tauri/src/commands/mod.rs
- Create or modify: src-tauri/src/commands/result_window_commands.rs
- Modify: src-tauri/src/app_actions.rs
- Modify: src-tauri/src/lib.rs

- [ ] Move payload enums and policy out of Commands.
- [ ] Delete CAPTURE_RESULT_WINDOW_PAYLOAD and direct arboard/window/event calls.
- [ ] Make each command call one ResultWindowRuntime interface.
- [ ] Update command registration and tests.
- [ ] Commit with message refactor: thin result window commands.

### Task 5: Deepen frontend Result Window

**Files:**
- Create: src/application/result-window/runtime.ts
- Create: src/application/result-window/runtime.test.ts
- Modify: src/views/ResultWindow/ResultWindow.tsx
- Move relevant workflow helpers from src/views/ResultWindow

- [ ] Write failing workflow tests for payload hydration, OCR file flow, translation triggering, close policy, and window sizing.
- [ ] Implement injected runtime state and actions.
- [ ] Leave pure presentation helpers in the View directory.
- [ ] Make ResultWindow.tsx consume renderState and actions only.
- [ ] Run focused tests and commit with message refactor: deepen frontend result window.

### Task 6: Verification

- [ ] Run cargo fmt --check --manifest-path src-tauri/Cargo.toml.
- [ ] Run cargo test --manifest-path src-tauri/Cargo.toml.
- [ ] Run npm test.
- [ ] Run npm run build.
- [ ] Verify commands/mod.rs has no mailbox, arboard, result window creation, or event emission.

