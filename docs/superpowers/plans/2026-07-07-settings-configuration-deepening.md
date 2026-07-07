# Settings Configuration Deepening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move durable user settings behind one backend Settings Configuration module and make frontend windows consume that module through one typed Tauri adapter seam.

**Architecture:** Keep Settings Navigation State as a frontend-only UI concern, and keep hotkey registration flow unchanged in this pass. Add a backend Settings Configuration module that owns defaults, path resolution, typed persistence, and migration of durable non-hotkey settings, then add a frontend settings adapter/store that hydrates every window role from the same snapshot.

**Tech Stack:** Tauri 2, Rust, React, Zustand, Vitest, Cargo tests.

---

## Scope

In scope:
- durable general / screenshot / translation settings
- backend-owned defaults and path resolution
- one frontend settings adapter seam
- one frontend config store shared by Settings, Capture, Result, and Pinned windows
- migration of durable non-hotkey settings out of legacy frontend persistence

Out of scope:
- hotkey registration lifecycle refactor
- Provider configuration refactor
- Selection translation platform adapters
- Settings UI redesign

## File Structure

Backend:
- Create: `src-tauri/src/application/settings/mod.rs`
  - public module surface for reading and updating settings sections
- Create: `src-tauri/src/application/settings/configuration.rs`
  - deep module that owns defaults, merge rules, path normalization, and persistence
- Modify: `src-tauri/src/application/mod.rs`
  - export the new Settings Configuration module
- Modify: `src-tauri/src/domain/config.rs`
  - replace stale `AppConfig` with real sectioned settings snapshot types
- Modify: `src-tauri/src/domain/mod.rs`
  - re-export new settings types
- Create: `src-tauri/src/commands/settings_commands.rs`
  - thin Tauri command seam for loading and updating settings
- Modify: `src-tauri/src/commands/mod.rs`
  - export settings commands
- Modify: `src-tauri/src/app_state.rs`
  - add shared Settings Configuration dependency
- Modify: `src-tauri/src/composition.rs`
  - construct and inject Settings Configuration
- Modify: `src-tauri/src/lib.rs`
  - register settings commands

Frontend:
- Create: `src/tauri/settings.ts`
  - frontend Tauri adapter seam for settings snapshot and updates
- Create: `src/stores/settingsConfigStore.ts`
  - backend-backed durable settings store
- Modify: `src/stores/settingsStore.ts`
  - keep only navigation state and current hotkey UI state
- Modify: `src/App.tsx`
  - hydrate the durable settings store for every window role
- Modify: `src/stores/appStore.ts`
  - add one entrypoint for applying translation defaults from settings
- Modify: `src/components/SettingsWindow/General/GeneralPage.tsx`
- Modify: `src/components/SettingsWindow/Screenshot/SaveSettingsPage.tsx`
- Modify: `src/components/SettingsWindow/Translation/TranslationSettingsPage.tsx`
- Modify: `src/components/ScreenshotSession/index.tsx`
- Modify: `src/components/PinnedImageWindow/index.tsx`
  - move these consumers to the durable settings store

Tests:
- Create: `src/tauri/__tests__/settings.test.ts`
- Create: `src/stores/settingsConfigStore.test.ts`
- Extend: `src-tauri/src/infrastructure/storage/config_file_test.rs`
- Add or extend inline Rust tests in `src-tauri/src/application/settings/configuration.rs`

Docs:
- Modify: `CONTEXT.md`
  - add `Settings Configuration module` as a durable term
- Modify: `ARCHITECTURE.md`
  - document the new backend/frontend settings seam

## Task 1: Replace the stale config shape with a real settings snapshot

**Files:**
- Modify: `src-tauri/src/domain/config.rs`
- Modify: `src-tauri/src/domain/mod.rs`
- Modify: `src-tauri/src/infrastructure/storage/config_file_test.rs`

- [ ] **Step 1: Add failing Rust tests for the new snapshot shape**

Cover:
- sectioned defaults for `general`, `screenshot`, and `translation`
- serde round-trip through `ConfigFile`
- screenshot save path default is not hard-coded to `~/Pictures/SnapLingo`

- [ ] **Step 2: Run the focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml config_file_test
```

Expected: FAIL because the existing `AppConfig` shape does not model the real durable settings.

- [ ] **Step 3: Replace `AppConfig` with the real settings snapshot types**

Add sectioned types in `src-tauri/src/domain/config.rs`, for example:

```rust
pub struct GeneralSettings { ... }
pub struct ScreenshotSettings { ... }
pub struct TranslationSettings { ... }
pub struct SettingsSnapshot { ... }
```

Keep the interface narrow: one snapshot type plus small section structs.

- [ ] **Step 4: Re-run the focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml config_file_test
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/domain/config.rs src-tauri/src/domain/mod.rs src-tauri/src/infrastructure/storage/config_file_test.rs
git commit -m "refactor(settings): define durable settings snapshot"
```

## Task 2: Add the backend Settings Configuration module and command seam

**Files:**
- Create: `src-tauri/src/application/settings/mod.rs`
- Create: `src-tauri/src/application/settings/configuration.rs`
- Modify: `src-tauri/src/application/mod.rs`
- Create: `src-tauri/src/commands/settings_commands.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/composition.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add failing Rust tests for the backend settings module**

Cover:
- loading a merged snapshot when config file is empty
- updating only one section preserves all other sections
- screenshot save path normalization expands `~`
- legacy frontend durable settings can be merged once without touching navigation state or hotkeys

- [ ] **Step 2: Run the focused Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml settings_configuration
```

Expected: FAIL because the settings module and commands do not exist yet.

- [ ] **Step 3: Implement the deep Settings Configuration module**

The module should expose a small interface such as:

```rust
pub struct SettingsConfiguration { ... }

impl SettingsConfiguration {
    pub fn snapshot(&self) -> Result<SettingsSnapshot>;
    pub fn update_general(&self, input: GeneralSettings) -> Result<SettingsSnapshot>;
    pub fn update_screenshot(&self, input: ScreenshotSettings) -> Result<SettingsSnapshot>;
    pub fn update_translation(&self, input: TranslationSettings) -> Result<SettingsSnapshot>;
}
```

Rules:
- defaults live here, not in frontend stores
- `CaptureOutputService` default path logic stays reusable, but the configured screenshot path becomes backend-owned state
- hotkey persistence stays in `startup_shortcuts.rs` for now

- [ ] **Step 4: Add thin Tauri commands for load and section updates**

Keep the command seam minimal:
- `get_settings_snapshot`
- `update_general_settings`
- `update_screenshot_settings`
- `update_translation_settings`

- [ ] **Step 5: Re-run focused backend tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml settings_configuration
cargo test --manifest-path src-tauri/Cargo.toml settings_commands
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/application/settings src-tauri/src/application/mod.rs src-tauri/src/commands/settings_commands.rs src-tauri/src/commands/mod.rs src-tauri/src/app_state.rs src-tauri/src/composition.rs src-tauri/src/lib.rs
git commit -m "feat(settings): add backend settings configuration seam"
```

## Task 3: Add the frontend settings adapter and backend-backed config store

**Files:**
- Create: `src/tauri/settings.ts`
- Create: `src/stores/settingsConfigStore.ts`
- Create: `src/tauri/__tests__/settings.test.ts`
- Create: `src/stores/settingsConfigStore.test.ts`
- Modify: `src/stores/settingsStore.ts`

- [ ] **Step 1: Add failing frontend tests for the adapter and store**

Cover:
- adapter command names and payload mapping
- store hydration from backend snapshot
- one-time migration of durable non-hotkey legacy values
- `settingsStore` no longer owns screenshot / translation / general durable values

- [ ] **Step 2: Run focused frontend tests**

Run:

```bash
npm test -- settings.test.ts settingsConfigStore.test.ts
```

Expected: FAIL because the adapter/store do not exist yet.

- [ ] **Step 3: Implement `src/tauri/settings.ts`**

Expose a small interface only:

```ts
getSettingsSnapshot()
updateGeneralSettings(input)
updateScreenshotSettings(input)
updateTranslationSettings(input)
```

- [ ] **Step 4: Implement the durable settings store**

Requirements:
- hydrate once from backend
- optionally migrate legacy non-hotkey local values once, then clear only migrated durable keys
- keep navigation and hotkey UI state out of this store

- [ ] **Step 5: Trim `settingsStore.ts` down to UI-only state**

Keep:
- main tab
- secondary tabs
- existing hotkey UI state for now

Delete:
- durable general / screenshot / translation fields and their setters

- [ ] **Step 6: Re-run focused frontend tests**

Run:

```bash
npm test -- settings.test.ts settingsConfigStore.test.ts
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/tauri/settings.ts src/tauri/__tests__/settings.test.ts src/stores/settingsConfigStore.ts src/stores/settingsConfigStore.test.ts src/stores/settingsStore.ts
git commit -m "feat(settings): add frontend settings adapter and config store"
```

## Task 4: Rewire settings pages and runtime consumers to the new seam

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/stores/appStore.ts`
- Modify: `src/components/SettingsWindow/General/GeneralPage.tsx`
- Modify: `src/components/SettingsWindow/Screenshot/SaveSettingsPage.tsx`
- Modify: `src/components/SettingsWindow/Translation/TranslationSettingsPage.tsx`
- Modify: `src/components/ScreenshotSession/index.tsx`
- Modify: `src/components/PinnedImageWindow/index.tsx`

- [ ] **Step 1: Add failing UI tests for durable settings consumption**

Cover:
- settings pages read and save through `settingsConfigStore`
- `ScreenshotSession` and `PinnedImageWindow` use backend-backed screenshot save path
- result window boot applies translation defaults from settings

- [ ] **Step 2: Run focused frontend tests**

Run:

```bash
npm test -- TranslationSettingsPage SaveSettingsPage GeneralPage PinnedImageWindow ScreenshotSession
```

Expected: FAIL because runtime consumers still read the old frontend-only store.

- [ ] **Step 3: Hydrate durable settings at window bootstrap**

`App.tsx` should initialize the durable settings store for every window role, not only Settings.

- [ ] **Step 4: Update runtime consumers**

Rules:
- screenshot save path comes from the durable settings store
- translation defaults are applied through one `appStore` entrypoint
- settings pages stop using local `useState` for values that should persist immediately

- [ ] **Step 5: Re-run focused frontend tests**

Run:

```bash
npm test -- TranslationSettingsPage SaveSettingsPage GeneralPage PinnedImageWindow ScreenshotSession
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/App.tsx src/stores/appStore.ts src/components/SettingsWindow/General/GeneralPage.tsx src/components/SettingsWindow/Screenshot/SaveSettingsPage.tsx src/components/SettingsWindow/Translation/TranslationSettingsPage.tsx src/components/ScreenshotSession/index.tsx src/components/PinnedImageWindow/index.tsx
git commit -m "refactor(settings): rewire runtime consumers to backend settings"
```

## Task 5: Update docs and run full verification

**Files:**
- Modify: `CONTEXT.md`
- Modify: `ARCHITECTURE.md`

- [ ] **Step 1: Update architecture docs**

Document:
- `Settings Configuration module`
- frontend `src/tauri/settings.ts` seam
- distinction between durable settings and UI navigation state

- [ ] **Step 2: Run full verification**

Run:

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --tests
git diff --check
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add CONTEXT.md ARCHITECTURE.md
git commit -m "docs(settings): document backend settings seam"
```

## Notes

- Do not move hotkey registration into the new settings module in this plan. The durable hotkey value source may stay bridged for now, but registration ownership remains where it is.
- Do not redesign Settings UI layouts in this pass.
- Prefer section-specific update commands over a broad patch interface. The codebase does not need a generic settings mutation language yet.
