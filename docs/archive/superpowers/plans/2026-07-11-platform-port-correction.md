# Platform Port Correction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox format for tracking.

**Goal:** Make System OCR and Selected Text Acquisition own their inward ports and leave OS modules as adapters selected only by Composition.

**Architecture:** System OCR mirrors the existing Tesseract injection direction. Selected Text Acquisition absorbs method vocabulary, collection, ordering, lookup, and diagnostics; the Infrastructure Registry is deleted.

**Tech Stack:** Rust, async-trait, Cargo tests.

---

### Task 1: Characterize System OCR injection

**Files:**
- Modify tests in src-tauri/src/application/providers/ocr/impls/system_ocr.rs
- Create: src-tauri/src/application/providers/ocr/system_engine.rs

- [ ] Write a failing test that constructs SystemOcrProvider with a fake Application-owned engine.
- [ ] Verify RED because Provider currently constructs Infrastructure.
- [ ] Define the minimal engine port beside the Provider.
- [ ] Inject the engine and verify GREEN.
- [ ] Commit with message refactor: move system ocr port inward.

### Task 2: Move System OCR adapter selection to Composition

**Files:**
- Modify: src-tauri/src/infrastructure/system/ocr/mod.rs
- Modify: src-tauri/src/infrastructure/system/ocr/macos.rs
- Modify: src-tauri/src/composition/provider_runtime.rs
- Modify: src-tauri/src/application/providers/ocr/impls/mod.rs

- [ ] Make the macOS engine implement the Application-owned port.
- [ ] Remove the Infrastructure factory import from the Provider.
- [ ] Remove platform cfg from portable Provider implementation modules.
- [ ] Register System OCR only in Composition when the adapter exists.
- [ ] Run OCR tests and commit with message refactor: compose system ocr adapter.

### Task 3: Characterize Selected Text ordering and diagnostics

**Files:**
- Modify: src-tauri/src/application/selected_text/mod.rs
- Create or extend: src-tauri/src/application/selected_text/tests.rs

- [ ] Write tests for macOS ordered success short-circuit.
- [ ] Write tests for Windows/Linux ShortcutCopy selection.
- [ ] Write tests for unsupported, unavailable, failed, and empty diagnostics.
- [ ] Verify tests protect Application behavior before moving ownership.
- [ ] Commit with message test: characterize selected text workflow.

### Task 4: Move method vocabulary inward

**Files:**
- Create: src-tauri/src/application/selected_text/method.rs
- Modify: src-tauri/src/domain/selection.rs
- Modify: src-tauri/src/infrastructure/system/selection/backend.rs
- Modify all platform selection adapters

- [ ] Move SelectionMethod and related workflow outcomes to Application/domain ownership.
- [ ] Make OS modules implement the inward port.
- [ ] Keep OS-specific context mechanics in Infrastructure.
- [ ] Run focused tests and commit with message refactor: move selection method seam inward.

### Task 5: Delete the Registry

**Files:**
- Delete: src-tauri/src/infrastructure/system/selection/registry.rs
- Modify: src-tauri/src/application/selected_text/mod.rs
- Modify: src-tauri/src/composition/selection_runtime.rs
- Modify: src-tauri/src/infrastructure/system/selection/mod.rs

- [ ] Let SelectedTextAcquirer own the ordered method collection.
- [ ] Delete Registry exports and tests.
- [ ] Run deletion test: confirm removing Registry concentrates lookup/order in Acquirer.
- [ ] Run all selection tests.
- [ ] Commit with message refactor: let selected text own method lookup.

### Task 6: Documentation and verification

**Files:**
- Modify: CONTEXT.md
- Modify: ARCHITECTURE.md
- Modify: docs/architecture/runtime-map.md

- [ ] Remove SelectionMethodRegistry domain language.
- [ ] Document System OCR and Selected Text port ownership.
- [ ] Run cargo fmt --check and full cargo test.
- [ ] Run npm test and npm run build.

