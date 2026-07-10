# Architecture Enforcement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox format for tracking.

**Goal:** Remove the remaining Backend Application-to-Infrastructure dependencies, delete all migration allowlists, align documentation, and add native cross-platform verification.

**Architecture:** Move shared capability interfaces inward to cohesive Application modules, leaving concrete ConfigFile, Keychain, HTTP, event bus, database, and Tauri mechanics in Infrastructure. Tighten architecture tests only after every consumer migrates.

**Tech Stack:** Rust, async-trait, GitHub Actions, TypeScript architecture tests.

---

### Task 1: Inventory remaining dependency leaks

**Files:**
- Modify: src-tauri/tests/architecture_dependency_test.rs
- Inspect: src-tauri/src/application/**

- [ ] Run the test in inventory mode and record every remaining allowlisted import.
- [ ] Group imports by capability: configuration, credentials, HTTP/LLM, events/history, capture host.
- [ ] Add a failing assertion for each group before migration.
- [ ] Commit with message test: inventory remaining application ports.

### Task 2: Move configuration and credential ports beside consumers

**Files:**
- Create: src-tauri/src/application/settings/store.rs
- Create: src-tauri/src/application/hotkeys/store.rs
- Create: src-tauri/src/application/providers/config_store.rs
- Create: src-tauri/src/application/providers/credential_store.rs
- Modify Settings, Hotkeys, Provider Configuration, OCR Configuration, Coordinators
- Modify Infrastructure ConfigFile and Keychain adapters
- Modify Composition

- [ ] Define a separate narrow port beside each consuming domain; do not create a global configuration or credentials module.
- [ ] Migrate one consumer at a time with fake-port tests.
- [ ] Remove direct ConfigFile, Keychain, keyring, and is_keychain_not_found imports from Application.
- [ ] Run focused tests after each consumer.
- [ ] Commit with message refactor: move configuration ports inward.

### Task 3: Move Provider network and LLM ports inward

**Files:**
- Create: src-tauri/src/application/providers/http_transport.rs
- Create: src-tauri/src/application/providers/llm_runtime.rs
- Move or redefine HttpClient, LLMClient, and model-listing vocabulary inside the Providers domain
- Modify Provider implementations and LlmIntrospection
- Modify Infrastructure HTTP/LLM adapters
- Modify Composition

- [ ] Write interface tests using fakes before moving implementations.
- [ ] Keep request mechanics in Infrastructure and Provider semantics in Application.
- [ ] Remove all infrastructure::http and infrastructure::llm imports from Application.
- [ ] Run Provider and LLM tests.
- [ ] Commit with message refactor: move network ports inward.

### Task 4: Move event, history, and capture ports beside consumers

**Files:**
- Create or extend: src-tauri/src/application/history/repository.rs
- Create: src-tauri/src/application/history/event_source.rs
- Create: src-tauri/src/application/providers/event_sink.rs
- Create or extend: src-tauri/src/application/capture/runtime_host.rs
- Move TauriCaptureSessionRuntimeHost to Infrastructure
- Modify History, Coordinators, Capture Runtime, Composition

- [ ] Define narrow event ports separately beside History and Providers; do not introduce a global events port bucket.
- [ ] Adapt EventBus and HistoryDatabase in Infrastructure.
- [ ] Move concrete Tauri capture host implementation out of Application.
- [ ] Run History, Coordinator, and Capture Runtime tests.
- [ ] Commit with message refactor: finish backend adapter direction.

### Task 5: Remove all allowlists

**Files:**
- Modify: src-tauri/tests/architecture_dependency_test.rs
- Modify: src/architecture/frontendDependencyRules.test.ts

- [ ] Delete backend path allowlist.
- [ ] Assert zero crate::infrastructure imports under backend Application production files.
- [ ] Delete remaining frontend migration inventory.
- [ ] Assert final frontend rules from the spec.
- [ ] Run both architecture test suites.
- [ ] Commit with message test: enforce final architecture direction.

### Task 6: Native cross-platform CI

**Files:**
- Create: .github/workflows/ci.yml
- Modify platform setup documentation if required

- [ ] Add macOS, Windows, and Ubuntu native jobs.
- [ ] Install each platform's Tauri, Tesseract, Leptonica, and Linux GTK dependencies.
- [ ] Run frontend tests/build and backend fmt/test/check on native runners.
- [ ] Avoid cross-compilation claims.
- [ ] Validate workflow syntax and commit with message ci: verify native desktop targets.

### Task 7: Documentation and final deletion audit

**Files:**
- Modify: CONTEXT.md
- Modify: ARCHITECTURE.md
- Modify: docs/architecture/runtime-map.md
- Modify or add ADR superseding ADR 0005 compatibility command

- [ ] Update all module names and dependency diagrams.
- [ ] Remove stale src/tauri, Registry, mailbox, raw event, and compatibility-command references.
- [ ] Run rg deletion checks.
- [ ] Commit with message docs: record rebuilt architecture.

### Task 8: Final verification

- [ ] Run npm test.
- [ ] Run npm run build.
- [ ] Run cargo fmt --check --manifest-path src-tauri/Cargo.toml.
- [ ] Run cargo test --manifest-path src-tauri/Cargo.toml.
- [ ] Run frontend and backend architecture tests.
- [ ] Confirm all six branch completion criteria from the spec.
