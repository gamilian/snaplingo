# App Action Dispatch Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make menu and Hotkey adapters map into one shared App Action vocabulary and execute workflows through one deep dispatch module.

**Architecture:** Add a root runtime module, `src-tauri/src/app_actions.rs`, that owns `AppAction`, typed Capture launch modes, and the only workflow dispatch match. Keep `app_shell.rs` responsible for menu ID mapping and menu-bar lifecycle. Keep `startup_shortcuts.rs` responsible for display-hotkey parsing and category/action binding, with one binding function supplying both the shared App Action and release timing.

**Tech Stack:** Rust, Tauri 2, `tauri-plugin-global-shortcut`, Cargo tests.

---

## Scope

In scope:

- Add one shared `AppAction` vocabulary.
- Replace raw Capture mode strings in menu/Hotkey adapters with a typed `CaptureLaunchMode`.
- Move workflow execution out of `app_shell.rs` and `startup_shortcuts.rs` into `app_actions.rs`.
- Make one Hotkey binding function determine both App Action and pressed/released timing.
- Delete the unused, incomplete `domain::HotkeyAction` type.
- Update architecture/domain documentation.

Out of scope:

- Do not change menu labels, menu IDs, default Hotkeys, or persisted category/action keys.
- Do not change Capture Session, OCR, Translation, Pinned Image, Settings, or Result Window behavior.
- Do not rename Tauri commands or frontend IPC.
- Do not refactor Hotkey registration state or display-hotkey parsing.
- Do not add a trait or mock host around Tauri dispatch; there is only one runtime implementation.
- Do not implement new app-level Settings/About Hotkeys.

## Assumptions

- Start from commit `371fedab` or a later accepted commit containing the completed Capture module consolidation.
- Untracked files under `docs/superpowers/plans/` are user-owned planning artifacts. Do not stage them with implementation commits unless explicitly listed.
- `domain::HotkeyAction` has no active consumer and the Rust crate is not exposing it as a supported external contract.
- Existing behavior where both About and Settings menu actions open the Settings Window is intentional and must be preserved.

## Success Criteria

- Menu and Hotkey adapters both call `dispatch_app_action`.
- `app_shell.rs` contains no Capture/OCR/Translation/Pinned workflow calls.
- `startup_shortcuts.rs` contains no Capture/OCR/Translation/Pinned workflow calls.
- Every known Hotkey category/action maps through one `hotkey_action_binding` function.
- Hotkey release timing is derived from the same binding used for dispatch.
- Existing menu IDs, Hotkey keys, Capture mode strings, error logs, and workflow outcomes remain unchanged.
- Menu dispatch retains one info log; Hotkey dispatch does not gain a new info log.
- `domain::HotkeyAction` is removed with no remaining references.

## File Structure

Create:

- `src-tauri/src/app_actions.rs` — shared App Action vocabulary, typed Capture launch modes, and runtime dispatch.

Modify:

- `src-tauri/src/lib.rs` — declare the new root module.
- `src-tauri/src/app_shell.rs` — map menu IDs to `AppAction` and delegate dispatch.
- `src-tauri/src/startup_shortcuts.rs` — map category/action keys to one Hotkey binding and delegate dispatch.
- `src-tauri/src/domain/mod.rs` — remove the obsolete Hotkey domain module/export.
- `CONTEXT.md` — define App Action Dispatch and update Hotkey Runtime responsibility.
- `ARCHITECTURE.md` — record the shared dispatch seam and adapter responsibilities.

Delete:

- `src-tauri/src/domain/hotkey.rs` — unused and incomplete Hotkey-only action vocabulary.

### Task 0: Establish a Clean Baseline

**Files:** none.

- [ ] **Step 1: Create an isolated worktree**

Use `superpowers:using-git-worktrees` from commit `371fedab` or a later accepted baseline.

- [ ] **Step 2: Confirm the worktree is clean**

Run:

```bash
git status --short
```

Expected: no output.

- [ ] **Step 3: Run focused baseline tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml app_shell
cargo test --manifest-path src-tauri/Cargo.toml startup_shortcuts
cargo test --manifest-path src-tauri/Cargo.toml application::hotkeys::runtime
```

Expected: PASS.

### Task 1: Define the Shared App Action Vocabulary

**Files:**

- Create: `src-tauri/src/app_actions.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing vocabulary tests**

Create `app_actions.rs` with the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::CaptureLaunchMode;

    #[test]
    fn capture_launch_modes_keep_existing_ipc_strings() {
        assert_eq!(CaptureLaunchMode::Screenshot.as_str(), "screenshot");
        assert_eq!(CaptureLaunchMode::ScreenshotCopy.as_str(), "screenshot-copy");
        assert_eq!(
            CaptureLaunchMode::ScreenshotTranslate.as_str(),
            "screenshot-translate"
        );
        assert_eq!(CaptureLaunchMode::ScreenshotOcr.as_str(), "screenshot-ocr");
        assert_eq!(
            CaptureLaunchMode::SilentScreenshotOcr.as_str(),
            "silent-screenshot-ocr"
        );
    }
}
```

Add to `lib.rs`:

```rust
mod app_actions;
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml app_actions
```

Expected: FAIL because `CaptureLaunchMode` does not exist.

- [ ] **Step 3: Add the typed vocabulary**

Add above the test module:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureLaunchMode {
    Screenshot,
    ScreenshotCopy,
    ScreenshotTranslate,
    ScreenshotOcr,
    SilentScreenshotOcr,
}

impl CaptureLaunchMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Screenshot => "screenshot",
            Self::ScreenshotCopy => "screenshot-copy",
            Self::ScreenshotTranslate => "screenshot-translate",
            Self::ScreenshotOcr => "screenshot-ocr",
            Self::SilentScreenshotOcr => "silent-screenshot-ocr",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppAction {
    OpenCapture(CaptureLaunchMode),
    TranslateSelection,
    OpenInputTranslation,
    OpenTranslationWindow,
    RunFileOcr,
    OpenOcrWindow,
    PinClipboardImage,
    TogglePinnedImagesVisibility,
    SwitchPinnedImageGroup,
    OpenSettings,
    OpenAbout,
    Quit,
}
```

- [ ] **Step 4: Add the single runtime dispatcher**

Add the production imports and `dispatch_app_action`:

```rust
use tauri::Manager;

use crate::{commands, settings_window, AppState};

pub(crate) fn dispatch_app_action(app: tauri::AppHandle, action: AppAction) {
    match action {
        AppAction::OpenCapture(mode) => {
            tauri::async_runtime::spawn(commands::open_capture_window_from_shortcut(
                app,
                mode.as_str(),
            ));
        }
        AppAction::TranslateSelection => {
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                if let Err(err) = commands::open_selection_translation_window_for_state(
                    app.clone(),
                    state.inner(),
                )
                .await
                {
                    log::error!("Failed to open selection translation window: {}", err);
                }
            });
        }
        AppAction::OpenInputTranslation => {
            if let Err(err) = commands::open_input_translation_window(app) {
                log::error!("Failed to open input translation window: {}", err);
            }
        }
        AppAction::OpenTranslationWindow => {
            if let Err(err) = commands::show_translation_window(app) {
                log::error!("Failed to show translation window: {}", err);
            }
        }
        AppAction::RunFileOcr => {
            if let Err(err) = commands::start_file_ocr(app) {
                log::error!("Failed to start file OCR: {}", err);
            }
        }
        AppAction::OpenOcrWindow => {
            if let Err(err) = commands::show_ocr_window(app) {
                log::error!("Failed to show OCR window: {}", err);
            }
        }
        AppAction::PinClipboardImage => {
            let state = app.state::<AppState>();
            if let Err(err) = commands::pin_clipboard_image_for_state(&app, state.inner()) {
                log::error!("Failed to pin clipboard image: {}", err);
            }
        }
        AppAction::TogglePinnedImagesVisibility => {
            if let Err(err) = commands::toggle_pinned_images_visibility(app) {
                log::error!("Failed to toggle pinned images: {}", err);
            }
        }
        AppAction::SwitchPinnedImageGroup => {
            let state = app.state::<AppState>();
            if let Err(err) = commands::switch_pinned_image_group_for_state(&app, state.inner()) {
                log::error!("Failed to switch pinned image group: {}", err);
            }
        }
        AppAction::OpenSettings | AppAction::OpenAbout => {
            if let Err(err) = settings_window::show_settings_window(&app) {
                log::error!("Failed to show settings window: {}", err);
            }
        }
        AppAction::Quit => app.exit(0),
    }
}
```

Preserve the existing per-workflow error messages. Do not add an unconditional dispatcher info log because Hotkey dispatch does not currently emit one.

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml app_actions
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/app_actions.rs src-tauri/src/lib.rs
git commit -m "refactor: define shared app action dispatch"
```

### Task 2: Convert the Menu Bar into an App Action Adapter

**Files:**

- Modify: `src-tauri/src/app_shell.rs`
- Test: inline tests in `src-tauri/src/app_shell.rs`

- [ ] **Step 1: Update mapping tests first**

Change the menu mapping assertions to expect shared actions:

```rust
assert_eq!(
    menu_action_for_id("screenshot"),
    Some(AppAction::OpenCapture(CaptureLaunchMode::Screenshot))
);
assert_eq!(
    menu_action_for_id("translate-selection"),
    Some(AppAction::TranslateSelection)
);
assert_eq!(
    menu_action_for_id("screenshot-translate"),
    Some(AppAction::OpenCapture(
        CaptureLaunchMode::ScreenshotTranslate
    ))
);
assert_eq!(
    menu_action_for_id("input-translation"),
    Some(AppAction::OpenInputTranslation)
);
assert_eq!(
    menu_action_for_id("screenshot-ocr"),
    Some(AppAction::OpenCapture(CaptureLaunchMode::ScreenshotOcr))
);
assert_eq!(menu_action_for_id("file-ocr"), Some(AppAction::RunFileOcr));
assert_eq!(menu_action_for_id("settings"), Some(AppAction::OpenSettings));
assert_eq!(menu_action_for_id("about"), Some(AppAction::OpenAbout));
assert_eq!(menu_action_for_id("quit"), Some(AppAction::Quit));
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml app_shell::tests::maps_known_menu_item_ids_to_actions
```

Expected: FAIL because `menu_action_for_id` still returns `MenuAction`.

- [ ] **Step 3: Replace the menu-local vocabulary**

Import:

```rust
use crate::app_actions::{
    dispatch_app_action, AppAction, CaptureLaunchMode,
};
```

Delete the local `MenuAction` enum.

Change `menu_action_for_id` to return `Option<AppAction>` and map:

```rust
pub(crate) fn menu_action_for_id(id: &str) -> Option<AppAction> {
    match id {
        SCREENSHOT_ID => Some(AppAction::OpenCapture(CaptureLaunchMode::Screenshot)),
        TRANSLATE_SELECTION_ID => Some(AppAction::TranslateSelection),
        SCREENSHOT_TRANSLATE_ID => Some(AppAction::OpenCapture(
            CaptureLaunchMode::ScreenshotTranslate,
        )),
        INPUT_TRANSLATION_ID => Some(AppAction::OpenInputTranslation),
        SCREENSHOT_OCR_ID => Some(AppAction::OpenCapture(CaptureLaunchMode::ScreenshotOcr)),
        FILE_OCR_ID => Some(AppAction::RunFileOcr),
        SETTINGS_ID => Some(AppAction::OpenSettings),
        ABOUT_ID => Some(AppAction::OpenAbout),
        QUIT_ID => Some(AppAction::Quit),
        _ => None,
    }
}
```

- [ ] **Step 4: Delegate menu dispatch**

Change the menu callback to call:

```rust
log::info!("Dispatching menu action: {:?}", action);
dispatch_app_action(app.clone(), action);
```

Delete `dispatch_menu_action` entirely.

Remove now-unused `commands`, `settings_window`, and `AppState` imports. Keep `tauri::Manager` if required by menu icon access.

- [ ] **Step 5: Run menu tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml app_shell
```

Expected: PASS.

- [ ] **Step 6: Verify menu adapter locality**

Run:

```bash
rg -n "commands::|settings_window|AppState|open_capture_window_from_shortcut|open_selection_translation_window_for_state" src-tauri/src/app_shell.rs
```

Expected: no output.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/app_shell.rs
git commit -m "refactor: route menu actions through app dispatch"
```

### Task 3: Convert Hotkeys into App Action Bindings

**Files:**

- Modify: `src-tauri/src/startup_shortcuts.rs`
- Verify: `src-tauri/src/application/hotkeys/runtime.rs`
- Test: inline tests in `src-tauri/src/startup_shortcuts.rs`

- [ ] **Step 1: Add failing binding tests**

Add a table-driven test covering every known Hotkey action:

```rust
use std::collections::HashSet;

use crate::app_actions::{AppAction, CaptureLaunchMode};
use crate::domain::hotkey_config::{
    DEFAULT_HOTKEYS, FILE_OCR_ACTION, INPUT_TRANSLATE_ACTION, OCR_CATEGORY, PIN_ACTION,
    PIN_SWITCH_GROUP_ACTION, PIN_TOGGLE_ALL_ACTION, SCREENSHOT_ACTION, SCREENSHOT_CATEGORY,
    SCREENSHOT_COPY_ACTION, SCREENSHOT_CUSTOM_ACTION, SCREENSHOT_OCR_ACTION,
    SCREENSHOT_TRANSLATE_ACTION, SELECTION_TRANSLATE_ACTION, SHOW_OCR_WINDOW_ACTION,
    SHOW_TRANSLATION_WINDOW_ACTION, SILENT_SCREENSHOT_OCR_ACTION, TRANSLATION_CATEGORY,
};
use super::hotkey_action_binding;

#[test]
fn maps_hotkey_keys_to_app_actions_and_trigger_timing() {
    let cases = [
        (
            SCREENSHOT_CATEGORY,
            SCREENSHOT_ACTION,
            AppAction::OpenCapture(CaptureLaunchMode::Screenshot),
            true,
        ),
        (
            SCREENSHOT_CATEGORY,
            SCREENSHOT_COPY_ACTION,
            AppAction::OpenCapture(CaptureLaunchMode::ScreenshotCopy),
            true,
        ),
        (
            SCREENSHOT_CATEGORY,
            SCREENSHOT_CUSTOM_ACTION,
            AppAction::OpenCapture(CaptureLaunchMode::Screenshot),
            true,
        ),
        (
            SCREENSHOT_CATEGORY,
            PIN_ACTION,
            AppAction::PinClipboardImage,
            false,
        ),
        (
            SCREENSHOT_CATEGORY,
            PIN_TOGGLE_ALL_ACTION,
            AppAction::TogglePinnedImagesVisibility,
            false,
        ),
        (
            SCREENSHOT_CATEGORY,
            PIN_SWITCH_GROUP_ACTION,
            AppAction::SwitchPinnedImageGroup,
            false,
        ),
        (
            TRANSLATION_CATEGORY,
            SELECTION_TRANSLATE_ACTION,
            AppAction::TranslateSelection,
            true,
        ),
        (
            TRANSLATION_CATEGORY,
            SCREENSHOT_TRANSLATE_ACTION,
            AppAction::OpenCapture(CaptureLaunchMode::ScreenshotTranslate),
            true,
        ),
        (
            TRANSLATION_CATEGORY,
            INPUT_TRANSLATE_ACTION,
            AppAction::OpenInputTranslation,
            false,
        ),
        (
            TRANSLATION_CATEGORY,
            SHOW_TRANSLATION_WINDOW_ACTION,
            AppAction::OpenTranslationWindow,
            false,
        ),
        (
            OCR_CATEGORY,
            SCREENSHOT_OCR_ACTION,
            AppAction::OpenCapture(CaptureLaunchMode::ScreenshotOcr),
            true,
        ),
        (
            OCR_CATEGORY,
            SILENT_SCREENSHOT_OCR_ACTION,
            AppAction::OpenCapture(CaptureLaunchMode::SilentScreenshotOcr),
            true,
        ),
        (
            OCR_CATEGORY,
            FILE_OCR_ACTION,
            AppAction::RunFileOcr,
            false,
        ),
        (
            OCR_CATEGORY,
            SHOW_OCR_WINDOW_ACTION,
            AppAction::OpenOcrWindow,
            false,
        ),
    ];

    for &(category, action_key, expected_action, expected_release) in &cases {
        let binding = hotkey_action_binding(category, action_key).unwrap();
        assert_eq!(binding.action, expected_action);
        assert_eq!(binding.trigger_on_release, expected_release);
    }

    let covered: HashSet<_> = cases
        .iter()
        .map(|(category, action, _, _)| (*category, *action))
        .collect();
    let defaults: HashSet<_> = DEFAULT_HOTKEYS
        .iter()
        .map(|hotkey| (hotkey.category, hotkey.action))
        .collect();

    assert_eq!(cases.len(), 14);
    assert_eq!(cases.len(), DEFAULT_HOTKEYS.len());
    assert_eq!(covered, defaults);

    assert!(hotkey_action_binding("unknown", "unknown").is_none());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml maps_hotkey_keys_to_app_actions_and_trigger_timing
```

Expected: FAIL because `hotkey_action_binding` does not exist.

- [ ] **Step 3: Add the binding model**

Import:

```rust
use crate::app_actions::{
    dispatch_app_action, AppAction, CaptureLaunchMode,
};
```

Add:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HotkeyActionBinding {
    pub action: AppAction,
    pub trigger_on_release: bool,
}

pub(crate) fn hotkey_action_binding(
    category: &str,
    action_key: &str,
) -> Option<HotkeyActionBinding> {
    let action = match (category, action_key) {
        (SCREENSHOT_CATEGORY, SCREENSHOT_ACTION | SCREENSHOT_CUSTOM_ACTION) => {
            AppAction::OpenCapture(CaptureLaunchMode::Screenshot)
        }
        (SCREENSHOT_CATEGORY, SCREENSHOT_COPY_ACTION) => {
            AppAction::OpenCapture(CaptureLaunchMode::ScreenshotCopy)
        }
        (SCREENSHOT_CATEGORY, PIN_ACTION) => AppAction::PinClipboardImage,
        (SCREENSHOT_CATEGORY, PIN_TOGGLE_ALL_ACTION) => {
            AppAction::TogglePinnedImagesVisibility
        }
        (SCREENSHOT_CATEGORY, PIN_SWITCH_GROUP_ACTION) => {
            AppAction::SwitchPinnedImageGroup
        }
        (TRANSLATION_CATEGORY, SCREENSHOT_TRANSLATE_ACTION) => {
            AppAction::OpenCapture(CaptureLaunchMode::ScreenshotTranslate)
        }
        (TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION) => {
            AppAction::TranslateSelection
        }
        (TRANSLATION_CATEGORY, INPUT_TRANSLATE_ACTION) => {
            AppAction::OpenInputTranslation
        }
        (TRANSLATION_CATEGORY, SHOW_TRANSLATION_WINDOW_ACTION) => {
            AppAction::OpenTranslationWindow
        }
        (OCR_CATEGORY, SCREENSHOT_OCR_ACTION) => {
            AppAction::OpenCapture(CaptureLaunchMode::ScreenshotOcr)
        }
        (OCR_CATEGORY, SILENT_SCREENSHOT_OCR_ACTION) => {
            AppAction::OpenCapture(CaptureLaunchMode::SilentScreenshotOcr)
        }
        (OCR_CATEGORY, FILE_OCR_ACTION) => AppAction::RunFileOcr,
        (OCR_CATEGORY, SHOW_OCR_WINDOW_ACTION) => AppAction::OpenOcrWindow,
        _ => return None,
    };

    let trigger_on_release = matches!(
        action,
        AppAction::OpenCapture(_) | AppAction::TranslateSelection
    );

    Some(HotkeyActionBinding {
        action,
        trigger_on_release,
    })
}
```

- [ ] **Step 4: Delegate Hotkey dispatch**

Replace `trigger_hotkey_action` with:

```rust
pub(crate) fn trigger_hotkey_action(
    app: tauri::AppHandle,
    category: String,
    action_key: String,
) {
    let Some(binding) = hotkey_action_binding(&category, &action_key) else {
        log::warn!("Unknown hotkey action: {}:{}", category, action_key);
        return;
    };

    dispatch_app_action(app, binding.action);
}
```

Remove all direct `commands` and `AppState` workflow calls from `startup_shortcuts.rs`.

- [ ] **Step 5: Derive release timing from the same binding**

Replace `should_register_hotkey_on_release` with:

```rust
pub(crate) fn should_register_hotkey_on_release(
    category: &str,
    action_key: &str,
) -> bool {
    hotkey_action_binding(category, action_key)
        .is_some_and(|binding| binding.trigger_on_release)
}
```

Delete `capture_mode_for_screenshot_hotkey_action`.

Keep `application/hotkeys/runtime.rs` unchanged; it continues to call `should_register_hotkey_on_release` and `trigger_hotkey_action`.

- [ ] **Step 6: Update existing tests**

Extend the existing test module imports with `hotkey_action_binding`, `AppAction`, `CaptureLaunchMode`, `HashSet`, `DEFAULT_HOTKEYS`, and every constant used by the exhaustive table. Remove tests that only cover `capture_mode_for_screenshot_hotkey_action`. Keep and update:

- accelerator conversion tests
- known category/action validation tests
- release timing tests
- the new exhaustive binding test

- [ ] **Step 7: Run focused tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml startup_shortcuts
cargo test --manifest-path src-tauri/Cargo.toml application::hotkeys::runtime
```

Expected: PASS.

- [ ] **Step 8: Verify Hotkey adapter locality**

Run:

```bash
rg -n "commands::|AppState|open_capture_window_from_shortcut|open_selection_translation_window_for_state|pin_clipboard_image_for_state" src-tauri/src/startup_shortcuts.rs
```

Expected: no output.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/startup_shortcuts.rs
git commit -m "refactor: route hotkeys through app action bindings"
```

### Task 4: Remove the Obsolete Hotkey Action Type

**Files:**

- Delete: `src-tauri/src/domain/hotkey.rs`
- Modify: `src-tauri/src/domain/mod.rs`

- [ ] **Step 1: Confirm the old type is unused**

Run:

```bash
rg -n "HotkeyAction|domain::hotkey" src-tauri/src
```

Expected: only `domain/hotkey.rs` and its declaration/re-export in `domain/mod.rs`.

- [ ] **Step 2: Delete the obsolete type**

Delete `src-tauri/src/domain/hotkey.rs`.

Remove from `domain/mod.rs`:

```rust
pub mod hotkey;
pub use hotkey::HotkeyAction;
```

- [ ] **Step 3: Verify deletion**

Run:

```bash
rg -n "HotkeyAction|domain::hotkey" src-tauri/src
cargo test --manifest-path src-tauri/Cargo.toml app_actions
```

Expected: first command has no output; tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/domain
git commit -m "refactor: remove obsolete hotkey action type"
```

### Task 5: Align Architecture Documentation

**Files:**

- Modify: `CONTEXT.md`
- Modify: `ARCHITECTURE.md`

- [ ] **Step 1: Add App Action Dispatch to the domain glossary**

Add to `CONTEXT.md`:

```markdown
### App Action Dispatch（应用动作分发）
`src-tauri/src/app_actions.rs` 中的运行时 dispatch module。

**职责：**
- 定义菜单与 Hotkey adapter 共用的 `AppAction` vocabulary
- 将 typed Capture launch mode 映射为现有 Capture Session mode 字符串
- 统一执行 Capture、OCR、Translation、Pinned Image、Settings 和 Quit workflow
- 保持 menu ID 与 Hotkey category/action key 的解析留在各自 adapter

**边界：**
- 不负责 Hotkey 注册、display hotkey parser 或 pressed/released timing
- 不负责 menu-bar 创建和应用生命周期
- 不包含 workflow implementation；只调用现有 Commands interface
```

- [ ] **Step 2: Update Hotkey Runtime language**

Change the Hotkey Runtime responsibility from delegating action dispatch/timing to `startup_shortcuts.rs` to:

- `startup_shortcuts.rs` owns Hotkey key binding, display parser, and trigger timing.
- App Action Dispatch owns workflow selection/execution.

- [ ] **Step 3: Update ARCHITECTURE.md**

Record:

- `src-tauri/src/app_actions.rs` as the shared menu/Hotkey dispatch seam.
- `app_shell.rs` as menu ID adapter plus menu-bar lifecycle.
- `startup_shortcuts.rs` as Hotkey binding/parser/timing adapter.
- Both adapters call one `dispatch_app_action` interface.

Do not change ADR-0006; this plan implements its menu-bar shell decision rather than revisiting it.

- [ ] **Step 4: Commit**

```bash
git add CONTEXT.md ARCHITECTURE.md
git commit -m "docs: record shared app action dispatch"
```

### Task 6: Full Verification

**Files:** none.

- [ ] **Step 1: Run architecture residue checks**

Run:

```bash
rg -n "MenuAction|dispatch_menu_action|capture_mode_for_screenshot_hotkey_action|HotkeyAction" src-tauri/src
rg -n "commands::|AppState|settings_window|open_capture_window_from_shortcut|open_selection_translation_window_for_state" src-tauri/src/app_shell.rs src-tauri/src/startup_shortcuts.rs
rg -n "dispatch_app_action" src-tauri/src/app_shell.rs src-tauri/src/startup_shortcuts.rs src-tauri/src/app_actions.rs
```

Expected:

- First command: no output.
- Second command: no output.
- Third command: both adapters call the shared dispatcher and the dispatcher is defined once.

- [ ] **Step 2: Verify all known Hotkeys are covered**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml maps_hotkey_keys_to_app_actions_and_trigger_timing
cargo test --manifest-path src-tauri/Cargo.toml capture_hotkeys_trigger_after_the_key_combo_is_released
```

Expected: PASS.

- [ ] **Step 3: Run full backend verification**

Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

Repository-wide Rust formatting is permitted, but review its diff separately from semantic changes.

- [ ] **Step 4: Run frontend verification**

Run:

```bash
npm test
npm run build
```

Expected: PASS.

- [ ] **Step 5: Check patch integrity**

Run:

```bash
git diff --check
git status --short
```

Expected: no whitespace errors; only intentional implementation and documentation changes remain.

- [ ] **Step 6: Manual Tauri smoke test**

Run:

```bash
npm run tauri:dev
```

Verify:

- Menu Screenshot opens screenshot Capture.
- Screenshot Hotkey opens the same Capture workflow.
- Menu and Hotkey Selection Translation open the same Result Window workflow.
- Menu and Hotkey Screenshot OCR open the same OCR Capture workflow.
- Menu and Hotkey Screenshot Translation open the same translation Capture workflow.
- Input Translation and File OCR still open their existing Result Window intents.
- Pin Hotkeys still pin, toggle visibility, and switch groups.
- Settings and About menu entries still open Settings.

Stop the dev process after verification.

## Final Review Checklist

- One `dispatch_app_action` match owns workflow execution.
- Menu and Hotkey adapters contain mapping only.
- Hotkey dispatch and timing come from one binding.
- Existing strings and behaviors are unchanged.
- No extra trait or abstraction was added.
- Every changed line traces to action dispatch consolidation or its documentation.
