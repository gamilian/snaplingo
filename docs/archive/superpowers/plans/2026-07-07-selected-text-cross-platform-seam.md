# Selected Text Cross-Platform Seam Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn Selected Text acquisition into a real cross-platform seam by keeping orchestration in `SelectedTextAcquirer` and adding real Windows/Linux adapters with explicit diagnostics.

**Architecture:** Preserve the current deep split: `SelectedTextAcquirer` owns method ordering and diagnostics; platform mechanics stay in `src-tauri/src/infrastructure/system/selection/*`. Reuse the common clipboard transaction helpers, keep the macOS adapter behavior intact, and add one real `ShortcutCopy` adapter for Windows and Linux so the seam is no longer macOS-only.

**Tech Stack:** Rust, Tauri, `enigo`, `arboard`, platform-specific system crates, Cargo tests.

---

## Scope

In scope:
- Windows selected-text acquisition via shortcut copy
- Linux selected-text acquisition via shortcut copy
- explicit diagnostics when a platform adapter is unavailable or returns empty text
- composition wiring and unit tests for scheme/order

Out of scope:
- Settings refactor
- hotkey registration refactor
- new frontend UI for selection capability
- browser-specific Linux/Windows adapters beyond shortcut copy

## File Structure

Core:
- Modify: `src-tauri/src/application/services/selected_text_acquirer.rs`
  - keep workflow ownership here, improve diagnostics only as needed
- Modify: `src-tauri/src/composition/selection_runtime.rs`
  - wire the real platform adapters into the runtime
- Modify: `src-tauri/src/domain/selection.rs`
  - extend types only if the existing diagnostics surface is not enough

Infrastructure:
- Modify: `src-tauri/src/infrastructure/system/selection/linux/mod.rs`
- Modify: `src-tauri/src/infrastructure/system/selection/windows/mod.rs`
- Create: `src-tauri/src/infrastructure/system/selection/linux/shortcut_copy.rs`
- Create: `src-tauri/src/infrastructure/system/selection/windows/shortcut_copy.rs`
- Modify: `src-tauri/src/infrastructure/system/selection/common/mod.rs`
- Reuse: `src-tauri/src/infrastructure/system/selection/common/clipboard_transaction.rs`
- Reuse: `src-tauri/src/infrastructure/system/selection/common/shortcut_copy.rs`

Tests:
- Extend: `src-tauri/src/application/services/selected_text_acquirer.rs`
- Extend: `src-tauri/src/infrastructure/system/selection/registry.rs`
- Add inline tests in:
  - `src-tauri/src/infrastructure/system/selection/linux/mod.rs`
  - `src-tauri/src/infrastructure/system/selection/windows/mod.rs`
  - `src-tauri/src/infrastructure/system/selection/linux/shortcut_copy.rs`
  - `src-tauri/src/infrastructure/system/selection/windows/shortcut_copy.rs`

Docs:
- Modify: `CONTEXT.md`
  - add `Selected Text acquisition` as a durable architectural term if needed
- Modify: `ARCHITECTURE.md`
  - document Windows/Linux selection adapters

## Task 1: Lock the seam behavior with focused tests

**Files:**
- Modify: `src-tauri/src/application/services/selected_text_acquirer.rs`
- Modify: `src-tauri/src/infrastructure/system/selection/registry.rs`
- Modify: `src-tauri/src/infrastructure/system/selection/linux/mod.rs`
- Modify: `src-tauri/src/infrastructure/system/selection/windows/mod.rs`

- [ ] **Step 1: Add failing tests for scheme and diagnostics**

Cover:
- Linux provider default scheme is no longer empty
- Windows provider default scheme is no longer empty
- `SelectedTextAcquirer` error diagnostics mention the attempted method when no text is acquired
- existing macOS ordering assumptions remain unchanged

- [ ] **Step 2: Run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml selected_text_acquirer
cargo test --manifest-path src-tauri/Cargo.toml selection_registry
```

Expected: FAIL because Linux and Windows providers still return empty method lists.

- [ ] **Step 3: Update platform provider modules to express the intended method order**

Target order:
- Windows: `ShortcutCopy`
- Linux: `ShortcutCopy`
- macOS: unchanged

- [ ] **Step 4: Re-run the focused tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml selected_text_acquirer
cargo test --manifest-path src-tauri/Cargo.toml selection_registry
```

Expected: some tests still fail because the real adapters do not exist yet, but scheme-related assertions should now be implementable.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/services/selected_text_acquirer.rs src-tauri/src/infrastructure/system/selection/registry.rs src-tauri/src/infrastructure/system/selection/linux/mod.rs src-tauri/src/infrastructure/system/selection/windows/mod.rs
git commit -m "test(selection): lock cross-platform seam behavior"
```

## Task 2: Implement the Windows `ShortcutCopy` adapter

**Files:**
- Create: `src-tauri/src/infrastructure/system/selection/windows/shortcut_copy.rs`
- Modify: `src-tauri/src/infrastructure/system/selection/windows/mod.rs`

- [ ] **Step 1: Add failing tests for the Windows adapter helpers**

Cover:
- method kind is `SelectionMethodKind::ShortcutCopy`
- availability is explicit
- helper code uses `Ctrl+C`, not platform-generic text assumptions

- [ ] **Step 2: Run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml windows::shortcut_copy
```

Expected: FAIL because the file does not exist yet.

- [ ] **Step 3: Implement the Windows shortcut-copy method**

Requirements:
- use `enigo` to press `Ctrl+C`
- use `clipboard_transaction::wait_for_clipboard_text_after_action(...)`
- return `SelectionAttempt::success`, `empty`, or `failed` through the adapter seam

Keep the interface local to the module. Do not add a second orchestration layer.

- [ ] **Step 4: Register the Windows adapter in `windows/mod.rs`**

`default_scheme()` and `methods()` must both include the new adapter.

- [ ] **Step 5: Re-run focused tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml windows::shortcut_copy
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/infrastructure/system/selection/windows/shortcut_copy.rs src-tauri/src/infrastructure/system/selection/windows/mod.rs
git commit -m "feat(selection): add windows shortcut copy adapter"
```

## Task 3: Implement the Linux `ShortcutCopy` adapter

**Files:**
- Create: `src-tauri/src/infrastructure/system/selection/linux/shortcut_copy.rs`
- Modify: `src-tauri/src/infrastructure/system/selection/linux/mod.rs`

- [ ] **Step 1: Add failing tests for the Linux adapter helpers**

Cover:
- method kind is `SelectionMethodKind::ShortcutCopy`
- adapter returns explicit failure messages instead of an empty provider
- provider module registers the adapter in the default scheme

- [ ] **Step 2: Run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml linux::shortcut_copy
```

Expected: FAIL because the file does not exist yet.

- [ ] **Step 3: Implement the Linux shortcut-copy method**

Requirements:
- use `enigo` to press `Ctrl+C`
- use `clipboard_transaction::wait_for_clipboard_text_after_action(...)`
- keep error messages explicit when clipboard or synthetic input fails

Keep this adapter best-effort but real. Do not leave the Linux provider empty after this task.

- [ ] **Step 4: Register the Linux adapter in `linux/mod.rs`**

`default_scheme()` and `methods()` must both include the new adapter.

- [ ] **Step 5: Re-run focused tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml linux::shortcut_copy
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/infrastructure/system/selection/linux/shortcut_copy.rs src-tauri/src/infrastructure/system/selection/linux/mod.rs
git commit -m "feat(selection): add linux shortcut copy adapter"
```

## Task 4: Tighten diagnostics and composition wiring

**Files:**
- Modify: `src-tauri/src/application/services/selected_text_acquirer.rs`
- Modify: `src-tauri/src/composition/selection_runtime.rs`
- Modify: `src-tauri/src/domain/selection.rs`

- [ ] **Step 1: Add failing tests for error diagnostics**

Cover:
- when all methods fail, the returned message includes the attempted method names
- unsupported and unavailable reasons survive the seam
- `open_selection_translation_window_for_state(...)` still receives one string error surface

- [ ] **Step 2: Run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml selected_text_acquirer
```

Expected: FAIL until diagnostics are updated.

- [ ] **Step 3: Improve diagnostics without adding a new shallow layer**

Rules:
- keep method ordering in `SelectedTextAcquirer`
- keep platform-specific messages below the seam
- do not add a separate “selection service” wrapper

- [ ] **Step 4: Re-run focused tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml selected_text_acquirer
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/services/selected_text_acquirer.rs src-tauri/src/composition/selection_runtime.rs src-tauri/src/domain/selection.rs
git commit -m "refactor(selection): improve cross-platform diagnostics"
```

## Task 5: Update docs and run verification

**Files:**
- Modify: `CONTEXT.md`
- Modify: `ARCHITECTURE.md`

- [ ] **Step 1: Update architecture docs**

Document:
- `SelectedTextAcquirer` as the workflow module
- Linux/Windows `ShortcutCopy` adapters
- the fact that platform methods live under `infrastructure/system/selection/*`

- [ ] **Step 2: Run full verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --tests
git diff --check
```

Expected: PASS.

- [ ] **Step 3: Manual verification on each platform**

Windows checklist:
- launch app
- select text in a native app
- trigger selection translate hotkey
- confirm result window opens with selected text

Linux checklist:
- launch app
- select text in a native app
- trigger selection translate hotkey
- confirm result window opens with selected text

- [ ] **Step 4: Commit**

```bash
git add CONTEXT.md ARCHITECTURE.md
git commit -m "docs(selection): document cross-platform selection seam"
```

## Notes

- Do not add a generic capability API unless the existing diagnostics surface proves insufficient. The current seam can stay deep with one orchestrator and several adapters.
- Do not change the macOS adapter ordering in this plan.
- If Linux synthetic copy proves desktop-session-specific, keep the adapter real and explicit about failures rather than falling back to an empty provider again.
