# Architecture Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox format for tracking.

**Goal:** Add executable dependency rules and freeze existing architectural leakage before any module migration.

**Architecture:** Introduce source-tree checks with explicit legacy allowlists. The checks pass on the current tree, reject new leakage, and are tightened by later plans until no allowlist remains.

**Tech Stack:** TypeScript, Vitest, Rust integration tests, Cargo.

---

### Task 1: Create the branch and verify baseline

**Files:** none

- [ ] Create branch codex/architecture-foundation from the merged spec commit.
- [ ] Run npm install.
- [ ] Run npm test and expect 85 files / 605 tests passing.
- [ ] Run cargo test --manifest-path src-tauri/Cargo.toml and expect 428 tests passing.
- [ ] Commit no files; record baseline output in the task log.

### Task 2: Freeze frontend Tauri leakage

**Files:**
- Create: src/architecture/frontendDependencyRules.test.ts
- Inspect: src/tauri/**
- Inspect: src/**/*.ts and src/**/*.tsx

- [ ] Write a failing test that scans production TypeScript files and initially permits @tauri-apps imports only under the future src/platform/tauri path.
- [ ] Run npm test -- src/architecture/frontendDependencyRules.test.ts and verify RED because all current src/tauri imports are legacy violations.
- [ ] Add the exact current src/tauri import allowlist and exact current event-string caller allowlist.
- [ ] Add a test proving a synthetic new View import is rejected by the rule helper.
- [ ] Run the focused test and verify GREEN.
- [ ] Commit with message test: freeze frontend tauri dependency leaks.

The test helper must accept an in-memory list of paths/imports so rejection behavior is tested without modifying production files.

### Task 3: Freeze backend Application-to-Infrastructure imports

**Files:**
- Create: src-tauri/tests/architecture_dependency_test.rs
- Inspect: src-tauri/src/application/**

- [ ] Write a failing Rust integration test that recursively scans production Rust files under application and reports crate::infrastructure imports.
- [ ] Run cargo test --manifest-path src-tauri/Cargo.toml --test architecture_dependency_test and verify RED.
- [ ] Add an explicit path-plus-import allowlist for the current baseline.
- [ ] Add a helper test proving an unlisted synthetic import is rejected.
- [ ] Run the focused test and verify GREEN.
- [ ] Commit with message test: freeze backend dependency leaks.

### Task 4: Record target ownership

**Files:**
- Create: docs/architecture/target-module-ownership.md
- Modify: ARCHITECTURE.md

- [ ] Document Frontend View, Frontend Application, Frontend Platform, Backend Command, Backend Application, Composition, and Infrastructure ownership.
- [ ] State that the allowlists are migration inventory, not accepted architecture.
- [ ] Link the approved spec.
- [ ] Run git diff --check.
- [ ] Commit with message docs: record target module ownership.

### Task 5: Full verification

- [ ] Run npm test.
- [ ] Run npm run build.
- [ ] Run cargo fmt --check --manifest-path src-tauri/Cargo.toml.
- [ ] Run cargo test --manifest-path src-tauri/Cargo.toml.
- [ ] Verify git status --short contains no unexpected files.
