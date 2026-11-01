# Hotkey Unification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Make all implemented global hotkeys use the same configurable registration path, and stop showing divergent default values between settings pages and runtime behavior.

**Architecture:** Frontend settings remain the source of user-selected hotkey strings, but startup registration still installs safe defaults before the main window synchronizes persisted settings. Backend shortcut registration uses one category/action registry so screenshot, translation, OCR, and pinned-image shortcuts all use the same display-string-to-Tauri-accelerator conversion.

**Tech Stack:** React, Zustand, Tauri v2 commands/events, `tauri-plugin-global-shortcut`, Rust unit tests, Vitest.

---

### Task 1: Centralize Frontend Defaults

**Files:**
- Modify: `src/stores/settingsStore.ts`
- Modify: `src/components/SettingsWindow/Screenshot/HotkeysPage.tsx`
- Modify: `src/components/SettingsWindow/OCR/HotkeysPage.tsx`
- Modify: `src/components/SettingsWindow/Translation/HotkeysPage.tsx`

- [x] **Step 1: Export one default hotkey object from the settings store**

Move the default hotkey object to an exported `DEFAULT_HOTKEYS` constant.

- [x] **Step 2: Use the exported defaults in each hotkey page**

Replace each page-local `defaultHotkeys` object with `DEFAULT_HOTKEYS.<category>`.

- [x] **Step 3: Keep unsupported OCR actions unset**

Ensure OCR `silent-screenshot-ocr`, `file-ocr`, and `show-window` reset to `未设置`.

- [x] **Step 4: Verify frontend**

Run: `npm test`
Expected: all Vitest tests pass.

Run: `npm run build`
Expected: TypeScript and Vite build pass.

### Task 2: Generalize Backend Hotkey Registration

**Files:**
- Modify: `src-tauri/src/startup_shortcuts.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Replace translation-only registry with category/action registry**

Use keys like `translation:input-translate`, `screenshot:pin`, and `ocr:screenshot-ocr`.

- [x] **Step 2: Register implemented startup defaults through the common function**

Supported actions:
- `screenshot:screenshot`
- `screenshot:screenshot-copy`
- `screenshot:screenshot-custom`
- `screenshot:pin`
- `screenshot:pin-toggle-all`
- `screenshot:pin-switch-group`
- `translation:selection-translate`
- `translation:screenshot-translate`
- `translation:input-translate`
- `translation:show-window`
- `ocr:screenshot-ocr`

Unsupported but known actions:
- `ocr:silent-screenshot-ocr`
- `ocr:file-ocr`
- `ocr:show-window`

- [x] **Step 3: Add a generic Tauri command**

Add `configure_hotkey(category, action, hotkey)` and keep `configure_translation_hotkey` only if needed for compatibility.

- [x] **Step 4: Add Rust tests**

Cover:
- display hotkey conversion to Tauri accelerators
- converted accelerators parse with `Shortcut::from_str`
- unset values unregister/no-op
- unsupported OCR actions reject non-empty hotkeys

- [x] **Step 5: Verify backend targeted tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml startup_shortcuts --lib`
Expected: targeted startup shortcut tests pass.

### Task 3: Synchronize All Frontend Categories

**Files:**
- Modify: `src/tauri/hotkeys.ts`
- Modify: `src/App.tsx`

- [x] **Step 1: Replace translation-only adapter with generic hotkey adapter**

Add `configureHotkey(category, action, hotkey)`.

- [x] **Step 2: Sync screenshot, translation, and OCR maps from the main window**

On app startup and persisted settings changes, call `configureHotkey` for every category/action.

- [x] **Step 3: Verify full build**

Run: `npm run build`
Expected: TypeScript and Vite build pass.

### Task 4: Full Verification and Self-Review

**Files:**
- All modified files.

- [x] **Step 1: Run full verification**

Run:
- `npm test`
- `npm run build`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib`
- `cargo test --manifest-path src-tauri/Cargo.toml --tests`

- [x] **Step 2: Review diff**

Confirm every changed line traces to hotkey registration, default alignment, or verification tests.

- [x] **Step 3: Report residual risks**

Call out that OCR silent/file/show-window remain product gaps rather than silently registered shortcuts.
