# Menu Bar App Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert SnapLingo from a Dock-first app into a menu bar resident utility where Settings opens only through explicit user actions and business workflows own their own windows.

**Architecture:** Introduce explicit backend modules for app shell, Settings Window lifecycle, and business-window identity. Keep translation/OCR/capture business logic intact; migrate callers so Settings is no longer a workflow bus or default app window.

**Tech Stack:** Tauri v2, Rust, macOS activation policy, Tauri tray/menu APIs, React/Vite, Zustand, Vitest, Rust unit tests.

---

## Source Documents

- Design: `docs/architecture/menu-bar-app-refactor.md`
- Existing app-shell decision to supersede: `docs/adr/0002-main-window-structure.md`
- Runtime map: `docs/architecture/runtime-map.md`

## File Structure

Create:

- `docs/adr/0006-menu-bar-app-shell.md`  
  Records the decision to make SnapLingo a menu bar resident app and supersede the app-shell part of ADR 0002.

- `src-tauri/src/settings_window.rs`  
  Owns Settings Window label, lazy creation, show/focus, hide-on-close, and visibility checks.

- `src-tauri/src/business_windows.rs`  
  Owns business-window label classification and visible business-window checks.

- `src-tauri/src/app_shell.rs`  
  Owns menu bar status item setup, menu action IDs, app reopen behavior, and explicit quit.

Modify:

- `src-tauri/src/lib.rs`  
  Wires new modules into Tauri setup/run lifecycle. Removes direct Settings show/focus logic from `RunEvent::Reopen`.

- `src-tauri/src/app_lifecycle.rs`  
  Shrinks to lifecycle predicates that do not know concrete business labels.

- `src-tauri/src/commands/mod.rs`  
  Keeps result-window commands independent from Settings and exposes business-window labels through the new module where needed.

- `src-tauri/src/startup_shortcuts.rs`  
  Reuses menu action dispatch paths for global shortcuts or delegates both to shared command functions.

- `src-tauri/tauri.conf.json`  
  Final phase removes or hides the default startup Settings Window.

- `src/App.tsx` and `src/appWindowRouting.ts`  
  Add explicit Settings route/label support while preserving existing Result/Capture/Pin routing.

Do not modify Provider, translation, OCR, capture-session, or Settings UI internals except where required to preserve routing after Settings lazy creation.

---

### Task 1: Record The App-Shell Architecture Decision

**Files:**
- Create: `docs/adr/0006-menu-bar-app-shell.md`
- Modify: `docs/architecture/runtime-map.md`

- [ ] **Step 1: Add ADR for menu bar app shell**

Create `docs/adr/0006-menu-bar-app-shell.md`:

```markdown
# ADR 0006: Menu Bar Resident App Shell

## Status
Accepted (2026-07-01)

## Context
SnapLingo currently creates a Settings Window at startup and treats macOS Reopen as a possible signal to show Settings. This couples app lifecycle, Settings, and business windows.

Bob is a menu bar app, and Snipaste uses a tray/menu-bar resident model. SnapLingo fits the same product model: global hotkeys and menu actions trigger business workflows, while Settings is explicit.

## Decision
SnapLingo will use a menu bar resident app shell. Startup creates the app runtime, global shortcuts, and menu bar status item, but does not show Settings. Settings opens only through explicit Settings entrypoints.

The Settings Window information architecture from ADR 0002 remains accepted. The app-shell assumption in ADR 0002 that SnapLingo needs a traditional primary main window is superseded.

## Consequences
- Business workflows must not depend on Settings being open.
- `RunEvent::Reopen` does not open Settings in menu bar mode.
- Settings is lazy-created and hidden on close.
- Capture overlay macOS activation logic remains owned by capture infrastructure.
```

- [ ] **Step 2: Update runtime map**

Add a short paragraph to `docs/architecture/runtime-map.md` under "Backend Runtime":

```markdown
The app shell is menu-bar resident: `app_shell` owns tray/menu setup and app reopen behavior, `settings_window` owns the Settings Window lifecycle, and `business_windows` owns business-window labels and visibility checks.
```

- [ ] **Step 3: Verify docs diff**

Run:

```bash
git diff -- docs/adr/0006-menu-bar-app-shell.md docs/architecture/runtime-map.md
```

Expected: ADR clearly supersedes only the app-shell part of ADR 0002.

- [ ] **Step 4: Commit**

```bash
git add docs/adr/0006-menu-bar-app-shell.md docs/architecture/runtime-map.md docs/architecture/menu-bar-app-refactor.md
git commit -m "docs: plan menu bar app shell"
```

---

### Task 2: Isolate Settings Window Lifecycle

**Files:**
- Create: `src-tauri/src/settings_window.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/app_lifecycle.rs`

- [ ] **Step 1: Write failing lifecycle tests**

Add tests in `src-tauri/src/settings_window.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_window_label_is_settings_domain_name() {
        assert_eq!(SETTINGS_WINDOW_LABEL, "main");
    }

    #[test]
    fn should_hide_settings_window_on_close() {
        assert!(should_hide_settings_window_instead_of_close(SETTINGS_WINDOW_LABEL));
    }

    #[test]
    fn does_not_hide_non_settings_windows_on_close() {
        assert!(!should_hide_settings_window_instead_of_close("capture-result"));
        assert!(!should_hide_settings_window_instead_of_close("capture"));
        assert!(!should_hide_settings_window_instead_of_close("pin-1"));
    }
}
```

Use `"main"` during the first migration step to avoid frontend churn. Rename to `"settings"` later.

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
cargo test -p snaplingo settings_window --lib
```

Expected: compile fails because `settings_window` does not exist.

- [ ] **Step 3: Implement `settings_window` module**

Create `src-tauri/src/settings_window.rs`:

```rust
use tauri::{Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub(crate) const SETTINGS_WINDOW_LABEL: &str = "main";

pub(crate) fn should_hide_settings_window_instead_of_close(window_label: &str) -> bool {
    window_label == SETTINGS_WINDOW_LABEL
}

pub(crate) fn show_settings_window(app: &tauri::AppHandle) -> Result<(), String> {
    let window = match app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        Some(window) => window,
        None => WebviewWindowBuilder::new(
            app,
            SETTINGS_WINDOW_LABEL,
            WebviewUrl::App("index.html?window=settings".into()),
        )
        .title("SnapLingo")
        .inner_size(900.0, 650.0)
        .min_inner_size(700.0, 500.0)
        .resizable(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?,
    };

    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

pub(crate) fn hide_settings_window(window: &WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

pub(crate) fn settings_window_is_visible(app: &tauri::AppHandle) -> bool {
    app.get_webview_window(SETTINGS_WINDOW_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}
```

- [ ] **Step 4: Wire module into `lib.rs` close handling**

In `src-tauri/src/lib.rs`:

```rust
mod settings_window;
```

Replace close predicate usage:

```rust
if !settings_window::should_hide_settings_window_instead_of_close(window.label()) {
    return;
}
```

Replace hide call:

```rust
if let Err(err) = settings_window::hide_settings_window(window) {
    log::warn!("Failed to hide settings window on close request: {}", err);
}
```

- [ ] **Step 5: Keep compatibility with `app_lifecycle` tests**

Either leave `app_lifecycle::should_hide_window_instead_of_close` delegating to `settings_window`, or move the tests fully to `settings_window`.

- [ ] **Step 6: Verify**

Run:

```bash
rustfmt --edition 2021 src-tauri/src/settings_window.rs src-tauri/src/lib.rs src-tauri/src/app_lifecycle.rs
cargo test -p snaplingo settings_window --lib
cargo test -p snaplingo app_lifecycle --lib
```

Expected: all targeted tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/settings_window.rs src-tauri/src/lib.rs src-tauri/src/app_lifecycle.rs
git commit -m "refactor: isolate settings window lifecycle"
```

---

### Task 3: Centralize Business Window Identity

**Files:**
- Create: `src-tauri/src/business_windows.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/app_lifecycle.rs`
- Modify: `src-tauri/src/commands/mod.rs`

- [ ] **Step 1: Write failing tests for business labels**

Create `src-tauri/src/business_windows.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_business_window_labels() {
        assert!(is_business_window_label("capture-result"));
        assert!(is_business_window_label("capture"));
        assert!(is_business_window_label("pin-abc"));
    }

    #[test]
    fn rejects_settings_and_unknown_labels() {
        assert!(!is_business_window_label("main"));
        assert!(!is_business_window_label("settings"));
        assert!(!is_business_window_label("random"));
    }
}
```

- [ ] **Step 2: Run tests and verify failure**

```bash
cargo test -p snaplingo business_windows --lib
```

Expected: compile fails because functions are not implemented or module is not wired.

- [ ] **Step 3: Implement business-window classification**

```rust
use tauri::Manager;

pub(crate) const CAPTURE_RESULT_WINDOW_LABEL: &str = "capture-result";
pub(crate) const CAPTURE_WINDOW_LABEL: &str = "capture";

pub(crate) fn is_business_window_label(label: &str) -> bool {
    label == CAPTURE_RESULT_WINDOW_LABEL
        || label == CAPTURE_WINDOW_LABEL
        || label.starts_with("pin-")
}

pub(crate) fn has_visible_business_window(app: &tauri::AppHandle) -> bool {
    app.webview_windows()
        .into_iter()
        .any(|(label, window)| is_business_window_label(&label) && window.is_visible().unwrap_or(false))
}
```

- [ ] **Step 4: Move capture-result label usage**

In `src-tauri/src/commands/mod.rs`, replace the local `CAPTURE_RESULT_WINDOW_LABEL` constant with:

```rust
use crate::business_windows::CAPTURE_RESULT_WINDOW_LABEL;
```

Keep `CAPTURE_WINDOW_LABEL` local only if command-specific behavior needs it; otherwise use `business_windows::CAPTURE_WINDOW_LABEL`.

- [ ] **Step 5: Update lifecycle predicate**

In `src-tauri/src/lib.rs`, replace ad hoc capture-result visibility check with:

```rust
business_windows::has_visible_business_window(app_handle)
```

- [ ] **Step 6: Verify**

```bash
rustfmt --edition 2021 src-tauri/src/business_windows.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/src/app_lifecycle.rs
cargo test -p snaplingo business_windows --lib
cargo test -p snaplingo app_lifecycle --lib
cargo test -p snaplingo commands::tests::result_entrypoints_use_dedicated_result_window_payloads --lib
```

Expected: all targeted tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/business_windows.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/src/app_lifecycle.rs
git commit -m "refactor: centralize business window labels"
```

---

### Task 4: Introduce App Shell Reopen Policy

**Files:**
- Create: `src-tauri/src/app_shell.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/app_lifecycle.rs`

- [ ] **Step 1: Write app-shell policy tests**

Create tests in `src-tauri/src/app_shell.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppShellMode {
    MenuBar,
    DockDebug,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReopenAction {
    Ignore,
    ShowSettings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_bar_mode_ignores_reopen() {
        assert_eq!(reopen_action_for_mode(AppShellMode::MenuBar), ReopenAction::Ignore);
    }

    #[test]
    fn dock_debug_mode_can_show_settings() {
        assert_eq!(reopen_action_for_mode(AppShellMode::DockDebug), ReopenAction::ShowSettings);
    }
}
```

- [ ] **Step 2: Run tests and verify failure**

```bash
cargo test -p snaplingo app_shell --lib
```

Expected: compile fails until module is wired and function implemented.

- [ ] **Step 3: Implement app-shell reopen policy**

```rust
pub(crate) fn current_app_shell_mode() -> AppShellMode {
    #[cfg(target_os = "macos")]
    {
        AppShellMode::MenuBar
    }

    #[cfg(not(target_os = "macos"))]
    {
        AppShellMode::DockDebug
    }
}

pub(crate) fn reopen_action_for_mode(mode: AppShellMode) -> ReopenAction {
    match mode {
        AppShellMode::MenuBar => ReopenAction::Ignore,
        AppShellMode::DockDebug => ReopenAction::ShowSettings,
    }
}

pub(crate) fn handle_reopen(app: &tauri::AppHandle) {
    match reopen_action_for_mode(current_app_shell_mode()) {
        ReopenAction::Ignore => {}
        ReopenAction::ShowSettings => {
            if let Err(err) = crate::settings_window::show_settings_window(app) {
                log::warn!("Failed to show settings window on app reopen: {}", err);
            }
        }
    }
}
```

- [ ] **Step 4: Replace `RunEvent::Reopen` body**

In `src-tauri/src/lib.rs`:

```rust
if let tauri::RunEvent::Reopen { .. } = event {
    app_shell::handle_reopen(app_handle);
}
```

Do not call `window.show()` or `window.set_focus()` directly in `lib.rs`.

- [ ] **Step 5: Verify**

```bash
rustfmt --edition 2021 src-tauri/src/app_shell.rs src-tauri/src/lib.rs src-tauri/src/app_lifecycle.rs
cargo test -p snaplingo app_shell --lib
cargo test -p snaplingo app_lifecycle --lib
```

Expected: app-shell tests pass. Existing app lifecycle tests may need to be updated if their old contract assumed Dock reopen always opens Settings.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/app_shell.rs src-tauri/src/lib.rs src-tauri/src/app_lifecycle.rs
git commit -m "refactor: move reopen policy into app shell"
```

---

### Task 5: Add Menu Bar Status Item And Menu Actions

**Files:**
- Modify: `src-tauri/src/app_shell.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/startup_shortcuts.rs`
- Modify: `src-tauri/Cargo.toml` only if Tauri tray/menu APIs require enabled features after compile check.

- [ ] **Step 1: Write menu action mapping tests**

Add to `src-tauri/src/app_shell.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuAction {
    TranslateSelection,
    ScreenshotTranslate,
    InputTranslation,
    ScreenshotOcr,
    FileOcr,
    Settings,
    About,
    Quit,
}

pub(crate) fn menu_action_for_id(id: &str) -> Option<MenuAction> {
    match id {
        "translate-selection" => Some(MenuAction::TranslateSelection),
        "screenshot-translate" => Some(MenuAction::ScreenshotTranslate),
        "input-translation" => Some(MenuAction::InputTranslation),
        "screenshot-ocr" => Some(MenuAction::ScreenshotOcr),
        "file-ocr" => Some(MenuAction::FileOcr),
        "settings" => Some(MenuAction::Settings),
        "about" => Some(MenuAction::About),
        "quit" => Some(MenuAction::Quit),
        _ => None,
    }
}
```

Test all known IDs and one unknown ID.

- [ ] **Step 2: Run mapping tests**

```bash
cargo test -p snaplingo app_shell --lib
```

Expected: menu action mapping tests pass before UI tray wiring.

- [ ] **Step 3: Add tray setup function**

Implement in `src-tauri/src/app_shell.rs` using Tauri v2 menu/tray APIs:

```rust
pub(crate) fn setup_menu_bar(app: &tauri::App) -> Result<(), String> {
    // Use tauri::menu and tauri::tray builders.
    // Keep all item IDs equal to menu_action_for_id inputs.
    // Wire menu events to dispatch_menu_action.
    Ok(())
}
```

Use real Tauri types during implementation. If compile errors show missing imports/features, resolve against Tauri v2 docs and the installed crate version.

- [ ] **Step 4: Implement menu action dispatch**

Dispatch through existing command paths:

```rust
pub(crate) fn dispatch_menu_action(app: tauri::AppHandle, action: MenuAction) {
    match action {
        MenuAction::TranslateSelection => { /* spawn open_selection_translation_window_for_state */ }
        MenuAction::ScreenshotTranslate => { /* spawn open_capture_window_from_shortcut(..., "screenshot-translate") */ }
        MenuAction::InputTranslation => { /* commands::open_result_window(String::new(), app) */ }
        MenuAction::ScreenshotOcr => { /* spawn open_capture_window_from_shortcut(..., "screenshot-ocr") */ }
        MenuAction::FileOcr => { /* commands::start_file_ocr(app) */ }
        MenuAction::Settings => { /* settings_window::show_settings_window(&app) */ }
        MenuAction::About => { /* settings_window::show_settings_window(&app), then optionally route About later */ }
        MenuAction::Quit => { app.exit(0); }
    }
}
```

Do not create new business behavior. Reuse existing command functions.

- [ ] **Step 5: Wire setup**

In `src-tauri/src/lib.rs` setup closure:

```rust
if let Err(err) = app_shell::setup_menu_bar(app) {
    log::warn!("Failed to setup menu bar: {}", err);
}
```

- [ ] **Step 6: Verify compile and focused tests**

```bash
rustfmt --edition 2021 src-tauri/src/app_shell.rs src-tauri/src/lib.rs src-tauri/src/startup_shortcuts.rs
cargo test -p snaplingo app_shell --lib
cargo test -p snaplingo startup_shortcuts --lib
```

Expected: compile succeeds and targeted tests pass.

- [ ] **Step 7: Manual smoke test in dev**

Run:

```bash
npm run tauri:dev
```

Expected:

- Menu bar/tray item appears.
- Settings menu item opens Settings.
- Input Translation menu item opens result/input window, not Settings.
- Quit exits app.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/app_shell.rs src-tauri/src/lib.rs src-tauri/src/startup_shortcuts.rs src-tauri/Cargo.toml
git commit -m "feat: add menu bar app shell"
```

Only include `src-tauri/Cargo.toml` if it changed.

---

### Task 6: Stop Showing Settings At Startup

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/settings_window.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/App.tsx`
- Modify: `src/appWindowRouting.ts`
- Modify: `src/appWindowRouting.test.ts`

- [ ] **Step 1: Add routing tests for Settings launch**

In `src/appWindowRouting.test.ts`, add:

```ts
it('recognizes settings window launch by label or search', () => {
  expect(isSettingsWindowLaunch('settings', '')).toBe(true);
  expect(isSettingsWindowLaunch('main', '?window=settings')).toBe(true);
  expect(isSettingsWindowLaunch('capture-result', '?window=capture-result')).toBe(false);
});
```

- [ ] **Step 2: Run frontend test and verify failure**

```bash
npm test -- src/appWindowRouting.test.ts
```

Expected: fails because `isSettingsWindowLaunch` does not exist.

- [ ] **Step 3: Implement Settings routing helper**

In `src/appWindowRouting.ts`:

```ts
export function isSettingsWindowLaunch(label: string, search: string): boolean {
  const params = new URLSearchParams(search);
  return label === 'settings' || params.get('window') === 'settings';
}
```

- [ ] **Step 4: Update `App.tsx` routing**

Ensure Settings renders when:

```ts
const isSettingsWindow = isSettingsWindowLaunch(currentWindow.label, window.location.search);
```

Render Settings only for Settings Window. Do not render Settings as fallback for unknown business windows.

- [ ] **Step 5: Hide or remove default startup window**

Preferred final change in `src-tauri/tauri.conf.json`:

```json
"windows": []
```

If Tauri config rejects empty windows, use an intermediate hidden window or remove the `windows` array according to Tauri v2 schema. The acceptance criterion is: startup must not show Settings.

- [ ] **Step 6: Verify**

```bash
npm test -- src/appWindowRouting.test.ts
npm run build
cargo test -p snaplingo settings_window --lib
```

Expected: frontend routing and backend settings tests pass.

- [ ] **Step 7: Manual startup verification**

Run:

```bash
npm run tauri:dev
```

Expected:

- App starts without visible Settings.
- Menu bar item appears.
- Settings opens only through menu item.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/src/settings_window.rs src-tauri/src/lib.rs src/App.tsx src/appWindowRouting.ts src/appWindowRouting.test.ts
git commit -m "refactor: lazy create settings window"
```

---

### Task 7: Rename Settings Window Label From `main` To `settings`

**Files:**
- Modify: `src-tauri/src/settings_window.rs`
- Modify: `src-tauri/src/app_lifecycle.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/infrastructure/system/selection/macos/self_webview.rs`
- Modify: `src-tauri/src/application/services/capture_session_service_test.rs`
- Modify: `src/App.tsx`
- Modify: `src/appWindowRouting.ts`
- Modify: `src/appWindowRouting.test.ts`

- [ ] **Step 1: Search all `main` window references**

Run:

```bash
rg -n '"main"|MAIN_WINDOW_LABEL|SETTINGS_WINDOW_LABEL|get_webview_window\\("main"\\)' src src-tauri
```

Expected: identify every call site before editing.

- [ ] **Step 2: Change Settings label constant**

In `src-tauri/src/settings_window.rs`:

```rust
pub(crate) const SETTINGS_WINDOW_LABEL: &str = "settings";
```

- [ ] **Step 3: Update tests that assert the label**

Change expected label from `"main"` to `"settings"` where the test refers to Settings Window identity.

- [ ] **Step 4: Handle self-webview selection deliberately**

`src-tauri/src/infrastructure/system/selection/macos/self_webview.rs` currently reads selection from `"main"`. Decide one of:

- If Settings selection is still useful, update it to `settings_window::SETTINGS_WINDOW_LABEL`.
- If it was only a legacy fallback, make the method unavailable when Settings is not visible.

Do not silently create Settings for selected-text acquisition.

- [ ] **Step 5: Update capture hidden-window tests**

Where tests use `"main"` as a normal non-capture window, rename to `"settings"` unless the test is explicitly about legacy labels.

- [ ] **Step 6: Verify no legacy direct main calls remain**

Run:

```bash
rg -n 'get_webview_window\\("main"\\)|"main"' src-tauri/src src | cat
```

Expected: no direct business workflow dependency on `"main"`. Some historical text in docs/tests may remain only if justified.

- [ ] **Step 7: Verify**

```bash
rustfmt --edition 2021 src-tauri/src/settings_window.rs src-tauri/src/app_lifecycle.rs src-tauri/src/lib.rs src-tauri/src/infrastructure/system/selection/macos/self_webview.rs src-tauri/src/application/services/capture_session_service_test.rs
cargo test -p snaplingo settings_window --lib
cargo test -p snaplingo app_lifecycle --lib
cargo test -p snaplingo self_webview --lib
cargo test -p snaplingo capture_session_service_test --lib
npm test -- src/appWindowRouting.test.ts
```

Expected: all targeted tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/settings_window.rs src-tauri/src/app_lifecycle.rs src-tauri/src/lib.rs src-tauri/src/infrastructure/system/selection/macos/self_webview.rs src-tauri/src/application/services/capture_session_service_test.rs src/App.tsx src/appWindowRouting.ts src/appWindowRouting.test.ts
git commit -m "refactor: rename main window to settings"
```

---

### Task 8: Remove Obsolete Main-Reopen Suppression

**Files:**
- Modify: `src-tauri/src/app_lifecycle.rs`
- Modify: `src-tauri/src/startup_shortcuts.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write lifecycle tests for final app-shell behavior**

In `src-tauri/src/app_lifecycle.rs` or `src-tauri/src/app_shell.rs`, keep tests focused on behavior:

```rust
#[test]
fn business_actions_do_not_need_main_reopen_suppression_in_menu_bar_mode() {
    assert_eq!(crate::app_shell::reopen_action_for_mode(crate::app_shell::AppShellMode::MenuBar), crate::app_shell::ReopenAction::Ignore);
}
```

- [ ] **Step 2: Remove suppression state**

Delete from `app_lifecycle.rs` if no longer used:

- `BUSINESS_HOTKEY_REOPEN_SUPPRESSION_MS`
- `SUPPRESS_MAIN_WINDOW_REOPEN_UNTIL_MS`
- `suppress_main_window_reopen_after_hotkey`
- `is_main_window_reopen_suppressed`
- `current_time_millis`

- [ ] **Step 3: Remove suppression calls**

Remove calls from:

- `src-tauri/src/startup_shortcuts.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`

Do not remove business-window visibility checks unless tests prove they are obsolete.

- [ ] **Step 4: Verify search**

Run:

```bash
rg -n 'suppress_main_window_reopen|is_main_window_reopen_suppressed|BUSINESS_HOTKEY_REOPEN' src-tauri/src
```

Expected: no matches.

- [ ] **Step 5: Verify tests**

```bash
rustfmt --edition 2021 src-tauri/src/app_lifecycle.rs src-tauri/src/startup_shortcuts.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
cargo test -p snaplingo app_lifecycle --lib
cargo test -p snaplingo app_shell --lib
cargo test -p snaplingo startup_shortcuts --lib
cargo test -p snaplingo commands::tests::result_entrypoints_use_dedicated_result_window_payloads --lib
```

Expected: all targeted tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/app_lifecycle.rs src-tauri/src/startup_shortcuts.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "refactor: remove main reopen suppression"
```

---

### Task 9: Full Verification And macOS Smoke Test

**Files:**
- All modified files.

- [ ] **Step 1: Run full automated verification**

Run:

```bash
cargo test -p snaplingo --lib
npm test
npm run build
npm run tauri:build
git diff --check
```

Expected:

- Rust library tests pass.
- Vitest suite passes.
- Frontend build passes.
- Tauri release build passes and produces signed `.app`.
- Diff has no whitespace errors.

- [ ] **Step 2: Launch release app**

Run:

```bash
open /Users/gamilian/work/code/snaplingo/target/release/bundle/macos/SnapLingo.app
```

Expected:

- Menu bar item appears.
- Settings Window does not appear.
- Dock icon is hidden or behaves according to the accepted menu-bar app-shell mode.

- [ ] **Step 3: Manual workflow verification**

Verify:

- Menu bar `Settings...` opens Settings.
- Closing Settings keeps app running.
- Menu bar `Input Translation` opens only Result/Input window.
- Selection translation hotkey opens only result window.
- Screenshot translation opens capture overlay and result window.
- Screenshot OCR opens capture overlay and OCR result.
- File OCR does not open Settings.
- Pinned windows still show, hide, and close correctly.
- Menu bar `Quit` exits the process.

- [ ] **Step 4: Capture evidence**

Record the exact commands and observed result in the PR/summary. If GUI automation is blocked by macOS permissions, state that manual verification was required.

- [ ] **Step 5: Final commit if smoke-test fixes were needed**

Only if changes were made during smoke testing:

```bash
git add <changed-files>
git commit -m "fix: stabilize menu bar app lifecycle"
```

---

## Implementation Notes

- Do not reintroduce `get_webview_window("main")` in business commands.
- Do not route translation/OCR/file OCR through Settings events.
- Do not merge capture overlay activation policy into `app_shell`.
- Keep phases small. If a phase requires broad unrelated changes, stop and split it.
- Treat macOS as the primary platform for this refactor. Preserve other platforms when straightforward, but do not add cross-platform tray redesign work unless required for compilation.

## Execution Options

Recommended execution mode: Subagent-Driven, one subagent per task, with review between tasks.

Fallback execution mode: Inline Execution, but stop after each task for diff review and targeted verification.

