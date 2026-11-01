# Current Architecture Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deepen the remaining shallow modules identified by the latest architecture review while keeping behavior unchanged.

**Architecture:** Work from the current documented architecture: Frontend Tauri Adapter seam, backend command seam, Provider Coordinators, Provider Configuration Module, Capture Session Runtime, and Application Composition. Phase A concentrates backend startup and Provider lifecycle knowledge; Phase B gradually collapses frontend Capture Session interaction rules behind a deeper module.

**Tech Stack:** Tauri 2, Rust 2021, React 18, TypeScript, Zustand, Vitest, Cargo tests.

---

## Scope

This plan assumes the documentation cleanup has already happened. Do not reintroduce historical Registry/Service docs or old phase reports.

Do this in a feature branch:

```bash
git switch -c codex/current-architecture-refactor
```

Do not touch these unrelated untracked files unless the user explicitly asks:

- `screenshot.html`
- `screenshot-test.html`
- `script/check-screenshot-flow.mjs`
- `src-tauri/src/lib.rs.bak`

## Success Criteria

- `src-tauri/src/lib.rs` only owns module declarations, Tauri builder/plugin setup, command registration, and calls into startup modules.
- Application state shape and construction no longer live directly in `lib.rs`.
- Custom Translation Provider startup restore and runtime add/remove share one Provider Configuration Module path.
- Settings navigation validity is resolved in the navigation model, not scattered across the UI caller.
- Screenshot Session starts moving interaction decisions out of the 3,400-line UI module without a big-bang rewrite.
- All existing tests pass:

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --tests
```

## File Map

Backend files:

- Create: `src-tauri/src/app_state.rs`
- Create: `src-tauri/src/startup_shortcuts.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/composition.rs`
- Modify: `src-tauri/src/application/providers/configuration.rs`
- Modify: `src-tauri/src/application/providers/mod.rs`
- Modify: `src-tauri/src/commands/provider_commands.rs`

Frontend files:

- Create: `src/components/SettingsWindow/settingsNavigationState.ts`
- Test: `src/components/SettingsWindow/settingsNavigationState.test.ts`
- Modify: `src/components/SettingsWindow/index.tsx`
- Modify: `src/components/SettingsWindow/navigationModel.tsx`
- Create: `src/components/ScreenshotSession/captureInteractionModel.ts`
- Test: `src/components/ScreenshotSession/captureInteractionModel.test.ts`
- Modify: `src/components/ScreenshotSession/index.tsx`

Docs:

- Modify: `ARCHITECTURE.md`
- Modify: `CONTEXT.md` only if a new durable domain term is introduced.

---

## Phase A: Backend Locality

### Task 1: Move AppState shape out of `lib.rs`

**Files:**

- Create: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/composition.rs`

- [ ] **Step 1: Add the new app state module**

Move these definitions from `src-tauri/src/lib.rs` into `src-tauri/src/app_state.rs`:

- `ScreenshotState`
- `AppState`
- `impl AppState { shutdown }`

Keep fields unchanged for this task. Do not move construction yet.

- [ ] **Step 2: Re-export AppState from lib**

In `src-tauri/src/lib.rs`, add:

```rust
mod app_state;
pub use app_state::{AppState, ScreenshotState};
```

Remove now-unused imports from `lib.rs`.

- [ ] **Step 3: Run backend compile-oriented tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: PASS. Existing warnings are acceptable.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/app_state.rs
git commit -m "refactor(app): move app state shape out of lib"
```

### Task 2: Make Application Composition own AppState construction

**Files:**

- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/composition.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Move construction into composition**

Move `AppState::new(config_path, app)` implementation body into `composition::build_app_state(config_path, app)`.

Keep `AppState` as a data struct in `app_state.rs`. Avoid a public `AppState::new` constructor unless a test or caller still needs it.

- [ ] **Step 2: Keep `lib.rs` as startup shell**

In `lib.rs`, the setup hook should call only:

```rust
let app_state = composition::build_app_state(config_path, app.handle().clone());
composition::subscribe_history_service(&app_state);
app.manage(app_state);
```

`lib.rs` should not construct `ConfigFile`, `Keychain`, Coordinators, capture modules, history modules, or workflow modules directly.

- [ ] **Step 3: Verify no construction leaked back**

Run:

```bash
rg "ConfigFile::new|Keychain::new|ReqwestHttpClient::new|CaptureSessionRuntime::new|WorkflowService::new" src-tauri/src/lib.rs
```

Expected: no matches.

- [ ] **Step 4: Run tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/app_state.rs src-tauri/src/composition.rs
git commit -m "refactor(app): deepen application composition"
```

### Task 3: Move startup shortcut registration behind a startup module

**Files:**

- Create: `src-tauri/src/startup_shortcuts.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/composition.rs`

- [ ] **Step 1: Extract shortcut constants and registration functions**

Move these from `lib.rs` into `startup_shortcuts.rs`:

- `SCREENSHOT_SHORTCUT`
- `SCREENSHOT_OCR_SHORTCUT`
- screenshot shortcut registration
- screenshot OCR shortcut registration
- pinned image shortcut registration
- pinned image visibility shortcut registration
- pinned image group switch shortcut registration

Expose one function:

```rust
pub(crate) async fn register_startup_shortcuts(app: tauri::AppHandle)
```

This function owns the 100ms delay and logs each registration result.

- [ ] **Step 2: Keep lib setup thin**

In `lib.rs` setup hook:

```rust
let app_handle = app.handle().clone();
tauri::async_runtime::spawn(startup_shortcuts::register_startup_shortcuts(app_handle));
```

- [ ] **Step 3: Run tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/startup_shortcuts.rs
git commit -m "refactor(app): isolate startup shortcuts"
```

### Task 4: Consolidate custom Translation Provider construction

**Files:**

- Modify: `src-tauri/src/application/providers/configuration.rs`
- Modify: `src-tauri/src/composition.rs`
- Modify: `src-tauri/src/commands/provider_commands.rs`
- Test: add tests inside `src-tauri/src/application/providers/configuration.rs`

- [ ] **Step 1: Add tests for the existing LLM constructor path**

In `configuration.rs`, add unit tests that prove:

- `create_llm_translation_provider` creates a provider with the custom id and name.
- OpenAI, Anthropic, and Gemini protocols are accepted.
- Reasoning level is preserved.

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib application::providers::configuration
```

Expected: FAIL only for missing tests or missing helper visibility if needed.

- [ ] **Step 2: Make composition use the shared constructor**

In `composition.rs`, replace manual `LLMClient` construction inside `register_custom_translation_providers` with:

```rust
let provider = create_llm_translation_provider(&def, http_client.clone(), api_key);
```

Remove now-unused LLM imports from `composition.rs`.

- [ ] **Step 3: Make runtime add use the shared constructor**

In `provider_commands.rs`, replace manual LLM construction in `add_custom_translation_provider` with:

```rust
let provider = create_llm_translation_provider(&def, state.http_client.clone(), request.api_key.clone());
```

Remove now-unused `Arc` and `LLMClient` construction imports.

- [ ] **Step 4: Run focused backend tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib application::providers
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/providers/configuration.rs src-tauri/src/composition.rs src-tauri/src/commands/provider_commands.rs
git commit -m "refactor(provider): reuse custom translation provider construction"
```

### Task 5: Move custom Translation Provider lifecycle into Provider Configuration Module

**Files:**

- Modify: `src-tauri/src/application/providers/configuration.rs`
- Modify: `src-tauri/src/commands/provider_commands.rs`
- Modify: `src-tauri/src/application/providers/mod.rs`

- [ ] **Step 1: Add application-level input and output structs**

In `configuration.rs`, add structs owned by the Provider Configuration Module:

```rust
pub struct AddCustomTranslationProviderInput {
    pub name: String,
    pub protocol: String,
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    pub reasoning_level: Option<String>,
}

pub struct CustomTranslationProviderView {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub endpoint: String,
    pub model: String,
    pub reasoning_level: Option<String>,
}
```

Keep command response types in `provider_commands.rs`; map from this view there.

- [ ] **Step 2: Add tests for validation and parsing**

Test:

- blank name is rejected
- blank endpoint is rejected
- blank model is rejected
- blank API key is rejected
- invalid protocol is rejected
- invalid reasoning level is rejected

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib application::providers::configuration
```

Expected: FAIL until implementation exists.

- [ ] **Step 3: Implement lifecycle helper**

Add a function in `configuration.rs` that owns validation, id creation, config save, keychain save, provider construction, coordinator registration, rollback, and activation.

Keep the command interface thin:

```rust
let view = add_custom_translation_provider(input, deps...).map_err(|e| e.to_string())?;
Ok(ProviderInfo::from(view))
```

The exact function shape may be adjusted during implementation to match existing `Keychain`, `ConfigFile`, and `TranslationCoordinator` ownership.

- [ ] **Step 4: Run provider command tests and backend tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib application::providers
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/providers/configuration.rs src-tauri/src/application/providers/mod.rs src-tauri/src/commands/provider_commands.rs
git commit -m "refactor(provider): centralize custom translation lifecycle"
```

---

## Phase B: Frontend Locality

### Task 6: Make Settings navigation state model-owned

**Files:**

- Create: `src/components/SettingsWindow/settingsNavigationState.ts`
- Create: `src/components/SettingsWindow/settingsNavigationState.test.ts`
- Modify: `src/components/SettingsWindow/index.tsx`
- Modify: `src/components/SettingsWindow/navigationModel.tsx`

- [ ] **Step 1: Write model tests**

Test a function that resolves active secondary navigation from persisted state:

- valid section + valid key returns that key
- valid section + stale key returns first secondary key
- simple section returns no secondary key
- invalid click key is ignored

Run:

```bash
npm test -- src/components/SettingsWindow/settingsNavigationState.test.ts
```

Expected: FAIL because file/function does not exist.

- [ ] **Step 2: Implement `settingsNavigationState.ts`**

Move `getActiveSecondaryKey` and `setSecondaryTab` logic out of `index.tsx`.

The module should accept:

- current `SettingsSection`
- persisted secondary keys
- requested secondary key
- setter functions

It should return resolved active item key and perform guarded updates.

- [ ] **Step 3: Thin `SettingsWindow/index.tsx`**

`SettingsWindow/index.tsx` should render the resolved model state and no longer contain switch statements for every section.

- [ ] **Step 4: Run focused tests**

```bash
npm test -- src/components/SettingsWindow/navigationModel.test.tsx src/components/SettingsWindow/settingsNavigationState.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/SettingsWindow/index.tsx src/components/SettingsWindow/navigationModel.tsx src/components/SettingsWindow/settingsNavigationState.ts src/components/SettingsWindow/settingsNavigationState.test.ts
git commit -m "refactor(settings): centralize navigation state"
```

### Task 7: Start Capture Session interaction model extraction

**Files:**

- Create: `src/components/ScreenshotSession/captureInteractionModel.ts`
- Create: `src/components/ScreenshotSession/captureInteractionModel.test.ts`
- Modify: `src/components/ScreenshotSession/index.tsx`

- [ ] **Step 1: Pick the first narrow extraction**

Start with capture completion flow only. Do not move selection geometry, annotation geometry, or rendering in this task.

The extracted module should decide:

- which completion action records a successful capture
- which flow comes from each Capture Mode
- what should happen after `copy`, `save`, `quick-save`, `pin`, `ocr`, `ocr-translate`, `print`, and `cancel`

- [ ] **Step 2: Write tests around current behavior**

Use existing behavior from `captureActions.ts` and `index.tsx` as the source of truth.

Run:

```bash
npm test -- src/components/ScreenshotSession/captureInteractionModel.test.ts
```

Expected: FAIL because the new module does not exist.

- [ ] **Step 3: Implement the model by composing existing helpers**

Do not duplicate geometry logic. Import from `captureActions.ts` where the helper already exists.

The model should be a pure TypeScript module. No React hooks, no Tauri calls, no DOM calls.

- [ ] **Step 4: Replace only the matching branch in `index.tsx`**

Use the new model for completion flow decisions, but leave the actual side effects in `index.tsx` for now.

- [ ] **Step 5: Run focused Screenshot Session tests**

```bash
npm test -- src/components/ScreenshotSession/captureActions.test.ts src/components/ScreenshotSession/captureInteractionModel.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/ScreenshotSession/index.tsx src/components/ScreenshotSession/captureInteractionModel.ts src/components/ScreenshotSession/captureInteractionModel.test.ts
git commit -m "refactor(capture): introduce interaction model"
```

### Task 8: Document the new module seams

**Files:**

- Modify: `ARCHITECTURE.md`
- Modify: `docs/architecture/runtime-map.md`
- Modify: `CONTEXT.md` only if a new durable term was introduced.

- [ ] **Step 1: Update architecture docs**

Document:

- `src-tauri/src/app_state.rs` owns AppState shape.
- `src-tauri/src/composition.rs` owns runtime dependency construction.
- `src-tauri/src/startup_shortcuts.rs` owns startup shortcut registration.
- Provider Configuration Module owns custom Translation Provider lifecycle.
- Settings navigation state and Capture interaction model are frontend modules with pure test seams.

- [ ] **Step 2: Search for outdated statements**

```bash
rg "AppState 形状 \\+ Tauri builder|custom Translation Provider restoration|Registry/Service|Provider Registry" ARCHITECTURE.md CONTEXT.md docs/architecture docs/adr -n
```

Expected: no misleading current-architecture statements.

- [ ] **Step 3: Commit**

```bash
git add ARCHITECTURE.md docs/architecture/runtime-map.md CONTEXT.md
git commit -m "docs: update architecture seams after refactor"
```

---

## Final Verification

Run the full verification suite:

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --tests
```

Expected: all pass. Existing warnings are acceptable if they predate this refactor.

Then inspect the final diff:

```bash
git status --short
git diff --stat master...HEAD
```

Expected: no unrelated temp files staged; no changes to user-owned untracked files.

## Execution Recommendation

Use Inline Execution if you want to stay in this conversation and review each checkpoint.

Use Subagent-Driven only if the backend and frontend phases should progress in parallel. If using subagents, split by phase:

- Backend subagent: Tasks 1-5
- Frontend subagent: Tasks 6-7 after backend reaches green
- Docs/review subagent: Task 8 and final verification
