# Hotkey Lifecycle Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move global hotkey defaults, persistence, startup registration, and Settings UI updates behind one backend-owned Hotkey Configuration lifecycle while keeping existing hotkey behavior unchanged.

**Architecture:** Keep `startup_shortcuts.rs` as the thin startup trigger/dispatcher, but move durable hotkey snapshot ownership into an Application-level `hotkeys` module. Add a frontend `src/tauri/hotkeys.ts` snapshot adapter and a dedicated hotkey config store so `settingsStore.ts` remains Settings UI navigation only. Commands should delegate to one backend lifecycle method that persists and registers atomically enough to avoid frontend/runtime divergence.

**Tech Stack:** Tauri 2, Rust, React, Zustand, Vitest, Cargo tests.

---

## Scope

In scope:
- screenshot / translation / OCR global hotkey defaults
- backend-owned hotkey snapshot and validation
- startup registration from backend hotkey configuration
- runtime update command that persists and registers through one backend module
- frontend hotkey adapter/store shared by Settings hotkey pages and app bootstrap
- legacy WebKit localStorage migration for hotkey values

Out of scope:
- changing hotkey defaults
- adding new hotkey actions
- redesigning hotkey UI
- implementing real Windows/Linux global shortcut backends beyond existing infrastructure
- adding system-wide conflict detection UI

## File Structure

Backend:
- Create: `src-tauri/src/domain/hotkey_config.rs`
  - sectioned hotkey snapshot types and `HotkeyCategory` / `HotkeyActionKey` value objects if useful
- Modify: `src-tauri/src/domain/mod.rs`
  - export hotkey config snapshot types
- Create: `src-tauri/src/application/hotkeys/mod.rs`
  - public module surface for hotkey configuration and runtime registration
- Create: `src-tauri/src/application/hotkeys/configuration.rs`
  - owns defaults, merge rules, validation, persistence, and legacy migration
- Create: `src-tauri/src/application/hotkeys/runtime.rs`
  - owns registration state, update lifecycle, rollback/diagnostics, and startup registration orchestration
- Modify: `src-tauri/src/application/mod.rs`
  - export the new hotkey module types
- Create: `src-tauri/src/commands/hotkey_commands.rs`
  - thin Tauri command seam for hotkey snapshot and updates
- Modify: `src-tauri/src/commands/mod.rs`
  - export hotkey commands and remove old command bodies from the broad command module
- Modify: `src-tauri/src/app_state.rs`
  - add shared Hotkey Runtime dependency
- Modify: `src-tauri/src/composition.rs`
  - construct and inject Hotkey Configuration / Runtime
- Modify: `src-tauri/src/lib.rs`
  - register new hotkey commands; startup hook calls the new runtime
- Modify: `src-tauri/src/startup_shortcuts.rs`
  - keep action dispatch helpers, but remove config persistence/default ownership from this file

Frontend:
- Modify: `src/tauri/hotkeys.ts`
  - expose `getHotkeySnapshot()` and `updateHotkey(category, action, hotkey)`
- Create: `src/stores/hotkeyConfigStore.ts`
  - backend-backed hotkey store with hydrate/update/reset helpers
- Modify: `src/stores/settingsStore.ts`
  - remove hotkey state and defaults, keeping only Settings navigation state
- Modify: `src/App.tsx`
  - hydrate hotkey store for Settings window; remove Settings-window batch registration effect
- Modify: `src/components/SettingsWindow/Screenshot/HotkeysPage.tsx`
- Modify: `src/components/SettingsWindow/Translation/HotkeysPage.tsx`
- Modify: `src/components/SettingsWindow/OCR/HotkeysPage.tsx`
  - read/write hotkeys through `hotkeyConfigStore`
- Modify: `src/components/SettingsWindow/Hotkey/hotkeyRegistration.ts`
  - either delete or reduce to a thin UI helper that calls store update

Tests:
- Add inline Rust tests in `src-tauri/src/application/hotkeys/configuration.rs`
- Add inline Rust tests in `src-tauri/src/application/hotkeys/runtime.rs`
- Add inline Rust tests in `src-tauri/src/commands/hotkey_commands.rs`
- Create: `src/tauri/__tests__/hotkeys.test.ts`
- Create: `src/stores/hotkeyConfigStore.test.ts`
- Update: `src/stores/settingsConfigStore.test.ts`
- Update: `src/components/SettingsWindow/Hotkey/hotkeyRegistration.test.ts`
- Add or update page tests for the three hotkey pages if needed

Docs:
- Modify: `CONTEXT.md`
  - add `Hotkey Configuration Module` and clarify Settings UI store boundary
- Modify: `ARCHITECTURE.md`
  - document hotkey lifecycle seam and startup/runtime ownership

## Task 1: Define a backend-owned hotkey snapshot

**Files:**
- Create: `src-tauri/src/domain/hotkey_config.rs`
- Modify: `src-tauri/src/domain/mod.rs`
- Modify: `src-tauri/src/startup_shortcuts.rs`

- [ ] **Step 1: Add failing Rust tests for sectioned hotkey defaults**

Cover:
- screenshot defaults match current `DEFAULT_HOTKEYS.screenshot`
- translation defaults match current `DEFAULT_HOTKEYS.translation`
- OCR defaults match current `DEFAULT_HOTKEYS.ocr`
- unset values remain represented as `"未设置"`
- unknown categories/actions are rejected by validation

- [ ] **Step 2: Run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml hotkey_config
```

Expected: FAIL because the hotkey config snapshot does not exist yet.

- [ ] **Step 3: Implement hotkey snapshot domain types**

Create `HotkeySettingsSnapshot` with three sections:

```rust
pub struct HotkeySettingsSnapshot {
    pub screenshot: HashMap<String, String>,
    pub translation: HashMap<String, String>,
    pub ocr: HashMap<String, String>,
}
```

Keep this data-only in Domain. Do not put Tauri accelerator conversion or registration logic in Domain.

- [ ] **Step 4: Move default hotkey data into reusable backend helpers**

Move the current `STARTUP_HOTKEYS` table out of `startup_shortcuts.rs` into the new backend hotkey configuration module, while preserving values exactly.

- [ ] **Step 5: Re-run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml hotkey_config
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/domain/hotkey_config.rs src-tauri/src/domain/mod.rs src-tauri/src/startup_shortcuts.rs
git commit -m "refactor(hotkeys): define backend hotkey snapshot"
```

## Task 2: Add Hotkey Configuration persistence and legacy migration

**Files:**
- Create: `src-tauri/src/application/hotkeys/mod.rs`
- Create: `src-tauri/src/application/hotkeys/configuration.rs`
- Modify: `src-tauri/src/application/mod.rs`
- Modify: `src-tauri/src/startup_shortcuts.rs`

- [ ] **Step 1: Add failing Rust tests for hotkey configuration**

Cover:
- `snapshot()` returns merged defaults when no backend config exists
- saving one action preserves all other actions
- invalid saved shortcuts fall back to defaults
- unknown saved actions are ignored
- legacy WebKit localStorage hotkeys migrate once into backend config
- durable non-hotkey settings from legacy localStorage are ignored here

- [ ] **Step 2: Run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml hotkey_configuration
```

Expected: FAIL because the module does not exist.

- [ ] **Step 3: Implement `HotkeyConfiguration`**

Expose a narrow interface:

```rust
pub struct HotkeyConfiguration { ... }

impl HotkeyConfiguration {
    pub fn snapshot(&self) -> Result<HotkeySettingsSnapshot>;
    pub fn update_hotkey(
        &self,
        category: &str,
        action: &str,
        hotkey: &str,
    ) -> Result<HotkeySettingsSnapshot>;
}
```

Rules:
- use the existing config key `"hotkeys"` to avoid a migration churn
- reuse the existing legacy WebKit decode logic, but keep it inside this module
- preserve existing display hotkey strings; do not store Tauri accelerators
- validation should call or share the current display-hotkey parser

- [ ] **Step 4: Thin `startup_shortcuts.rs`**

Keep only:
- display hotkey to accelerator parsing if runtime still uses it
- action dispatch mapping
- release-vs-press timing rule
- shortcut registration/unregistration wrappers if not yet moved to runtime

Remove from `startup_shortcuts.rs`:
- backend config loading/saving
- legacy localStorage migration
- default snapshot ownership

- [ ] **Step 5: Re-run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml hotkey_configuration
cargo test --manifest-path src-tauri/Cargo.toml startup_shortcuts
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/application/hotkeys src-tauri/src/application/mod.rs src-tauri/src/startup_shortcuts.rs
git commit -m "feat(hotkeys): add backend hotkey configuration"
```

## Task 3: Add Hotkey Runtime registration lifecycle

**Files:**
- Create: `src-tauri/src/application/hotkeys/runtime.rs`
- Modify: `src-tauri/src/application/hotkeys/mod.rs`
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/composition.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/startup_shortcuts.rs`

- [ ] **Step 1: Add failing Rust tests for registration lifecycle**

Cover:
- startup registration uses the configuration snapshot, not hard-coded defaults
- updating to the same accelerator is a no-op
- updating from one accelerator to another unregisters the previous accelerator
- updating to `"未设置"` unregisters the previous accelerator and persists the unset value
- if runtime registration fails, the persisted snapshot does not advance silently
- release-timing actions still register via the release path

- [ ] **Step 2: Run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml hotkey_runtime
```

Expected: FAIL because runtime lifecycle does not exist.

- [ ] **Step 3: Implement `HotkeyRuntime`**

Expose:

```rust
pub struct HotkeyRuntime { ... }

impl HotkeyRuntime {
    pub fn snapshot(&self) -> Result<HotkeySettingsSnapshot>;
    pub fn register_startup_hotkeys(&self, app: &tauri::AppHandle) -> Result<()>;
    pub fn update_hotkey(
        &self,
        app: &tauri::AppHandle,
        category: String,
        action: String,
        hotkey: String,
    ) -> Result<HotkeyUpdateOutcome>;
}
```

`HotkeyUpdateOutcome` should include the updated snapshot and resolved accelerator:

```rust
pub struct HotkeyUpdateOutcome {
    pub snapshot: HotkeySettingsSnapshot,
    pub accelerator: Option<String>,
}
```

- [ ] **Step 4: Move registration state out of `startup_shortcuts.rs`**

Move `HOTKEY_REGISTRATIONS` and the registration update sequence into `HotkeyRuntime`.

Important behavior:
- Resolve and validate first.
- Register new accelerator before unregistering old one, preserving current behavior.
- Persist only after successful runtime registration.
- If persistence fails after registration succeeds, either revert registration to previous accelerator or return an explicit error and leave tests documenting the chosen behavior. Prefer rollback if it stays small.

- [ ] **Step 5: Keep action dispatch in one place**

Either:
- keep `trigger_hotkey_action(...)` in `startup_shortcuts.rs` and call it from runtime; or
- move dispatch into `hotkeys/runtime.rs` if the file stays readable.

Do not duplicate action dispatch between startup and update paths.

- [ ] **Step 6: Wire AppState and startup hook**

`composition.rs` should construct:

```rust
let hotkey_configuration = Arc::new(HotkeyConfiguration::new(config_file.clone()));
let hotkey_runtime = Arc::new(HotkeyRuntime::new(hotkey_configuration.clone()));
```

`lib.rs` startup should call the runtime through AppState instead of `startup_shortcuts::register_startup_shortcuts(...)`.

- [ ] **Step 7: Re-run focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml hotkey_runtime
cargo test --manifest-path src-tauri/Cargo.toml startup_shortcuts
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/application/hotkeys src-tauri/src/app_state.rs src-tauri/src/composition.rs src-tauri/src/lib.rs src-tauri/src/startup_shortcuts.rs
git commit -m "refactor(hotkeys): centralize registration lifecycle"
```

## Task 4: Add backend hotkey commands and typed frontend adapter

**Files:**
- Create: `src-tauri/src/commands/hotkey_commands.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/tauri/hotkeys.ts`
- Create: `src/tauri/__tests__/hotkeys.test.ts`

- [ ] **Step 1: Add failing command and adapter tests**

Backend command tests cover:
- `get_hotkey_snapshot` delegates to `HotkeyRuntime.snapshot()`
- `update_hotkey` returns updated snapshot and accelerator
- errors are returned as one string surface

Frontend adapter tests cover:
- command names are `get_hotkey_snapshot` and `update_hotkey`
- frontend camelCase maps to backend snake_case if needed
- adapter returns normalized frontend shape

- [ ] **Step 2: Run focused tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml hotkey_commands
npm test -- hotkeys.test.ts
```

Expected: FAIL because commands/adapter do not exist yet.

- [ ] **Step 3: Implement backend commands**

Commands:
- `get_hotkey_snapshot`
- `update_hotkey`

Remove the old `configure_translation_hotkey` command unless a compatibility test proves it is still needed. Prefer one generic command.

- [ ] **Step 4: Implement frontend adapter**

`src/tauri/hotkeys.ts` should expose:

```ts
export async function getHotkeySnapshot(): Promise<HotkeySnapshot>;
export async function updateHotkey(input: HotkeyUpdateInput): Promise<HotkeyUpdateOutcome>;
```

Keep `HotkeyCategory` exported for UI code.

- [ ] **Step 5: Register commands in `lib.rs`**

Replace old command registrations as needed:
- remove `commands::configure_translation_hotkey`
- keep `commands::configure_hotkey` only if it is now a compatibility wrapper
- add `commands::get_hotkey_snapshot`
- add `commands::update_hotkey`

- [ ] **Step 6: Re-run focused tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml hotkey_commands
npm test -- hotkeys.test.ts
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/hotkey_commands.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src/tauri/hotkeys.ts src/tauri/__tests__/hotkeys.test.ts
git commit -m "feat(hotkeys): add hotkey command adapter"
```

## Task 5: Add frontend backend-backed hotkey store

**Files:**
- Create: `src/stores/hotkeyConfigStore.ts`
- Create: `src/stores/hotkeyConfigStore.test.ts`
- Modify: `src/stores/settingsStore.ts`
- Modify: `src/stores/settingsConfigStore.test.ts`

- [ ] **Step 1: Add failing frontend store tests**

Cover:
- store hydrates once from backend snapshot
- update calls backend and applies returned snapshot
- reset action uses backend default from snapshot rather than local constants
- old `settingsStore` no longer exposes `hotkeys`, `setHotkey`, `clearHotkey`, or `resetHotkeys`
- legacy persisted hotkeys in `snaplingo-settings` are left for backend migration and not re-owned by Settings UI store

- [ ] **Step 2: Run focused frontend tests**

Run:

```bash
npm test -- hotkeyConfigStore.test.ts settingsConfigStore.test.ts
```

Expected: FAIL because store does not exist and `settingsStore` still owns hotkeys.

- [ ] **Step 3: Implement `hotkeyConfigStore.ts`**

State shape:

```ts
interface HotkeyConfigState {
  hydrated: boolean;
  snapshot: HotkeySnapshot | null;
  hydrate: () => Promise<HotkeySnapshot>;
  updateHotkey: (category: HotkeyCategory, action: string, hotkey: string) => Promise<HotkeySnapshot>;
  resetHotkey: (category: HotkeyCategory, action: string) => Promise<HotkeySnapshot>;
  resetCategory: (category: HotkeyCategory) => Promise<HotkeySnapshot>;
}
```

Implementation rules:
- backend remains source of truth
- keep frontend store as cache + update helper
- do not duplicate default hotkey constants in frontend

- [ ] **Step 4: Trim `settingsStore.ts`**

Keep only:
- `activeMainTab`
- `screenshotSubTab`
- `translationSubTab`
- `ocrSubTab`
- `servicesSubTab`
- setters for those UI states

Remove:
- `DEFAULT_HOTKEYS`
- hotkey maps
- hotkey setters/resetters

- [ ] **Step 5: Re-run focused frontend tests**

Run:

```bash
npm test -- hotkeyConfigStore.test.ts settingsConfigStore.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/stores/hotkeyConfigStore.ts src/stores/hotkeyConfigStore.test.ts src/stores/settingsStore.ts src/stores/settingsConfigStore.test.ts
git commit -m "feat(hotkeys): add backend backed hotkey store"
```

## Task 6: Rewire Settings hotkey pages and App bootstrap

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/components/SettingsWindow/Screenshot/HotkeysPage.tsx`
- Modify: `src/components/SettingsWindow/Translation/HotkeysPage.tsx`
- Modify: `src/components/SettingsWindow/OCR/HotkeysPage.tsx`
- Modify: `src/components/SettingsWindow/Hotkey/hotkeyRegistration.ts`
- Modify: `src/components/SettingsWindow/Hotkey/hotkeyRegistration.test.ts`
- Add page-level tests if needed:
  - `src/components/SettingsWindow/Screenshot/HotkeysPage.test.tsx`
  - `src/components/SettingsWindow/Translation/HotkeysPage.test.tsx`
  - `src/components/SettingsWindow/OCR/HotkeysPage.test.tsx`

- [ ] **Step 1: Add failing UI tests for hotkey page consumption**

Cover:
- each hotkey page reads values from `hotkeyConfigStore`
- recording a hotkey calls store `updateHotkey`
- clear sets the value to `"未设置"` through backend update
- reset uses backend defaults through store helper
- duplicate warning still works from current snapshot values
- `App.tsx` no longer batch-registers hotkeys from frontend state when the Settings window opens

- [ ] **Step 2: Run focused frontend tests**

Run:

```bash
npm test -- HotkeysPage hotkeyRegistration.test.ts
```

Expected: FAIL because pages still use `settingsStore`.

- [ ] **Step 3: Rewire pages to `hotkeyConfigStore`**

For each page:
- select the relevant category from `snapshot`
- render a loading state if hotkeys are not hydrated yet
- call `updateHotkey(category, action, value)` on save
- call `updateHotkey(category, action, "未设置")` on clear
- call `resetHotkey(category, action)` or `resetCategory(category)` for reset

- [ ] **Step 4: Remove frontend batch registration in `App.tsx`**

Delete the effect that loops over `hotkeys` and calls `configureHotkey(...)` when Settings window is open. Runtime registration now happens inside backend startup/update lifecycle.

- [ ] **Step 5: Re-run focused frontend tests**

Run:

```bash
npm test -- HotkeysPage hotkeyRegistration.test.ts hotkeyConfigStore.test.ts
npm run build
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/App.tsx src/components/SettingsWindow src/stores/hotkeyConfigStore.ts src/stores/hotkeyConfigStore.test.ts
git commit -m "refactor(hotkeys): rewire settings pages to backend hotkeys"
```

## Task 7: Update docs and run full verification

**Files:**
- Modify: `CONTEXT.md`
- Modify: `ARCHITECTURE.md`

- [ ] **Step 1: Update architecture docs**

Document:
- `Hotkey Configuration Module`
- `Hotkey Runtime`
- frontend `src/tauri/hotkeys.ts` adapter
- distinction between Settings navigation state, durable app settings, and hotkey configuration
- startup registration vs runtime update ownership

- [ ] **Step 2: Run full verification**

Run:

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --tests
cargo fmt --manifest-path src-tauri/Cargo.toml --check
git diff --check
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add CONTEXT.md ARCHITECTURE.md
git commit -m "docs(hotkeys): document hotkey lifecycle seam"
```

## Notes

- This plan intentionally does not add conflict detection UI. The backend can continue returning registration errors as strings.
- Keep display hotkey values stable. Do not change `⌥D`, `⇧⌥S`, `"未设置"`, or other defaults in this refactor.
- Avoid a generic settings mutation language. Hotkeys get their own module because runtime registration is a side effect, unlike durable settings section updates.
- Prefer preserving existing command names as compatibility wrappers only if tests or current frontend callers require it; otherwise move to `get_hotkey_snapshot` / `update_hotkey`.
