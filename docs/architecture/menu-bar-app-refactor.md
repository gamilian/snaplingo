# Menu Bar App Refactor

## Status

Draft design, 2026-07-01.

This document describes a lifecycle and window architecture refactor for SnapLingo. It is intended to guide implementation work, not to document the current behavior as final.

## Summary

SnapLingo should move from a regular Dock-first desktop app to a menu bar resident utility, similar to Bob and Snipaste:

- The app starts in the background and shows a menu bar status item.
- Settings are opened only by explicit user intent.
- Translation, OCR, screenshot, and pinned image workflows own their own business windows.
- System activation events do not imply "open Settings".

This refactor is not about removing windows. It is about removing Settings Window from the business workflow path.

## References

Bob and Snipaste are useful references, but they are not identical.

- Bob describes itself as a menu bar app. Its official guide says: "Bob 是一个菜单栏软件，启动之后，菜单栏会出现一个图标". Features are triggered through menu items or shortcuts.
- Snipaste uses a tray/menu-bar resident model. Its docs describe starting snip by hotkey or by left-clicking the tray icon, and opening Preferences from explicit commands or the tray menu.

The relevant product model for SnapLingo is therefore: menu bar resident utility with explicit Preferences/Settings, not a windowless app.

## Relationship To Existing ADRs

ADR 0002 says SnapLingo needs a traditional main window and a tray resident model. This refactor changes that app-shell decision:

- Superseded: "需要传统主窗口（非纯托盘应用）" as the primary app shell.
- Preserved: the Settings Window information architecture with feature-domain navigation and centralized Provider configuration.

If this design is accepted, add a new ADR that supersedes the app-shell part of ADR 0002 while keeping the Settings Window structure.

## Current Problems

The current app has three concerns coupled together:

1. App lifecycle: startup, Dock reopen, close behavior, global shortcuts.
2. Settings Window: configuration UI and Provider management.
3. Business workflows: translation result, OCR result, screenshot overlay, pinned images.

This coupling has produced recurring window bugs:

- Business actions can accidentally show or focus the Settings Window.
- macOS `RunEvent::Reopen` is overloaded as a proxy for user intent.
- Result-window activation requires suppression windows and visible-window guards.
- Business commands have historically emitted events through the `main` window.
- Startup creates a Settings Window even when the user only wants hotkeys or menu bar actions.

Recent fixes reduced the symptoms by making `capture-result` independent and suppressing unwanted main-window reopen behavior. The architectural issue remains: Settings still exists as the default app window.

## Goals

- Start SnapLingo as a menu bar resident app without showing Settings.
- Make Settings open only through explicit Settings entrypoints.
- Ensure business workflows never depend on Settings being open.
- Centralize window lifecycle rules behind small backend modules.
- Remove most special-case "hide main" and "suppress main reopen" logic after the migration.
- Keep current Settings UI structure and Provider configuration behavior.
- Keep global hotkeys working while no Settings Window exists.
- Keep screenshot overlay, result window, and pinned image behavior independent.

## Non-Goals

- Do not redesign the Settings UI.
- Do not rewrite Provider configuration, translation, OCR, or capture workflow logic.
- Do not introduce a new frontend router framework.
- Do not remove screenshot overlay macOS-specific window handling.
- Do not make the app truly windowless.
- Do not support multiple simultaneous Settings windows.

## Target Architecture

### Module Ownership

```text
src-tauri/src/
├─ app_shell.rs
│  Owns app startup shell, menu bar status item, menu actions, activation policy.
│
├─ settings_window.rs
│  Owns lazy creation and show/focus/hide behavior for the Settings Window.
│
├─ business_windows.rs
│  Owns common business-window visibility queries and labels.
│
├─ commands/
│  Remains the frontend-facing command adapter seam.
│  Business commands call business window modules, not Settings.
│
├─ startup_shortcuts.rs
│  Registers global shortcuts and dispatches to business commands.
│
└─ infrastructure/system/
   Keeps OS-specific adapters: capture overlay, screenshots, selection, pinned windows.
```

Frontend window routing remains explicit:

```text
src/
├─ App.tsx
├─ appWindowRouting.ts
├─ components/SettingsWindow/
├─ components/ResultWindow/
├─ components/ScreenshotSession/
└─ components/PinnedImageWindow/
```

### Window Types

| Window | Label | Created By | Opens When | Can Open Settings? |
| --- | --- | --- | --- | --- |
| Settings | `settings` or `main` during migration | `settings_window` | Menu item, explicit shortcut, first-run onboarding | Not applicable |
| Result | `capture-result` | result/business window module | Translation, OCR, file OCR, manual input | No |
| Capture | `capture` | capture window module | Screenshot workflows | No |
| Pinned Image | `pin-*` | pinned window module | Pin image/text actions | No |

During migration the Settings label may remain `main` to avoid broad frontend churn. The final state should prefer a domain label such as `settings`.

### App Shell

`app_shell` should be the only module that knows SnapLingo is menu-bar resident.

Responsibilities:

- Set macOS activation policy for menu bar mode.
- Create menu bar status item.
- Build tray/menu items.
- Dispatch menu actions.
- Coordinate explicit quit.
- Decide what to do with system reopen/activate events.

The app shell must not know how translation results are rendered. It can invoke high-level commands such as "open manual translation window" or "open Settings".

### Settings Window

`settings_window` owns the Settings Window lifecycle.

Interface sketch:

```rust
pub(crate) const SETTINGS_WINDOW_LABEL: &str = "settings";

pub(crate) fn show_settings_window(app: &tauri::AppHandle) -> Result<(), String>;
pub(crate) fn hide_settings_window(window: &tauri::WebviewWindow) -> Result<(), String>;
pub(crate) fn settings_window_is_visible(app: &tauri::AppHandle) -> bool;
```

Rules:

- The Settings Window is lazy-created.
- Closing Settings hides it; it does not quit the app.
- `show_settings_window` is the only allowed way to show/focus Settings.
- Business workflows must not emit events through Settings.
- Settings can register hotkey changes and Provider changes, but it is not required for startup hotkeys to work.

### Business Windows

`business_windows` owns common business-window identity and visibility behavior.

Interface sketch:

```rust
pub(crate) fn has_visible_business_window(app: &tauri::AppHandle) -> bool;
pub(crate) fn is_business_window_label(label: &str) -> bool;
```

Initial business labels:

- `capture-result`
- `capture`
- `pin-*`

Rules:

- Business windows can be created from hotkeys, menu actions, and commands.
- Business windows never fallback to showing Settings.
- Business windows can outlive Settings.
- Business windows should own their payload handoff directly, not through Settings events.

### Tauri Commands

Tauri command modules remain the frontend/backend seam. The command layer should call explicit window modules instead of directly creating or focusing arbitrary labels.

Allowed direction:

```text
command -> result_window/business_windows
command -> settings_window only for explicit settings commands
command -> capture_window/pinned_window adapters
```

Disallowed direction:

```text
translation command -> get_webview_window("main") -> show/focus/emit
ocr command -> Settings Window event
capture command error -> show Settings unless explicitly user-facing settings error
```

### RunEvent::Reopen

In menu bar mode, `RunEvent::Reopen` should not open Settings.

Recommended behavior:

- macOS menu bar mode: ignore `RunEvent::Reopen`.
- Development/Dock debug mode, if retained: call `settings_window::show_settings_window`.

Do not infer business intent from `RunEvent::Reopen`.

## Menu Bar Actions

Initial menu:

```text
SnapLingo
├─ Translate Selection
├─ Screenshot Translate
├─ Input Translation
├─ Screenshot OCR
├─ File OCR
├─ Settings...
├─ About
└─ Quit SnapLingo
```

Behavior:

- `Translate Selection`: same backend path as selection translation hotkey.
- `Screenshot Translate`: same backend path as screenshot translation hotkey.
- `Input Translation`: opens the result/input window, not Settings.
- `Screenshot OCR`: opens capture overlay in OCR mode.
- `File OCR`: opens result/OCR workflow or file picker path, not Settings.
- `Settings...`: calls `settings_window::show_settings_window`.
- `Quit`: explicit process exit.

Later versions can add menu customization. Do not add that in the initial refactor.

## Activation Policy

Target release behavior on macOS:

- Use menu bar status item.
- Hide Dock icon with accessory-style activation policy.
- Avoid automatic Settings Window creation.
- Temporarily switch activation policy only when a window type requires it, and restore predictably.

Important constraint:

The screenshot overlay already has specialized macOS activation handling. Do not merge that logic into the app shell. The app shell owns normal app residency; capture infrastructure owns overlay behavior.

## Migration Plan

### Phase 1: Introduce Settings Window Module

Purpose: isolate Settings lifecycle without changing product behavior yet.

Tasks:

- Add `src-tauri/src/settings_window.rs`.
- Move main-window show/focus/hide behavior into it.
- Keep current label `main` for compatibility.
- Add unit tests for close-hides and explicit show semantics.
- Replace direct main show/focus call sites with `settings_window::show_settings_window` only where the user intent is explicitly Settings.

Verification:

- Closing Settings hides it and app keeps running.
- Existing settings navigation still works.
- Business actions still use existing result/capture windows.

### Phase 2: Introduce Business Window Registry

Purpose: centralize business-window identity and visibility.

Tasks:

- Add `src-tauri/src/business_windows.rs`.
- Move `capture-result`, `capture`, and `pin-*` label classification into one place.
- Replace ad hoc label checks in lifecycle code.
- Add tests for known labels and visible business window decisions.

Verification:

- `capture-result` visible prevents Settings reopen.
- `capture` visible prevents Settings reopen.
- `pin-*` behavior is unchanged.

### Phase 3: Add App Shell Module

Purpose: isolate menu bar and app lifecycle.

Tasks:

- Add `src-tauri/src/app_shell.rs`.
- Move `RunEvent::Reopen` handling out of `lib.rs` into app-shell functions.
- Add menu action dispatch functions.
- Keep the existing default window during this phase if needed.

Verification:

- Unit tests prove menu actions dispatch to explicit command categories.
- Reopen no longer directly contains Settings show logic in `lib.rs`.

### Phase 4: Add Menu Bar Status Item

Purpose: expose the real Bob/Snipaste-style entrypoint.

Tasks:

- Create tray/menu bar item in setup.
- Add menu items listed above.
- Wire menu item events to existing command paths.
- Add explicit Quit behavior.

Verification:

- App shows menu bar item.
- Menu item "Settings..." opens Settings.
- Menu item "Input Translation" opens result/input window only.
- Menu item "Quit" exits.

### Phase 5: Stop Creating Settings At Startup

Purpose: complete product behavior change.

Tasks:

- Remove default visible window from `tauri.conf.json`, or set startup window hidden as an intermediate step.
- Lazy-create Settings via `settings_window`.
- Ensure global shortcut registration does not depend on mounted Settings frontend state.
- Ensure Provider credential hydration does not depend on Settings.

Verification:

- Launch app: no Settings Window appears.
- Menu bar icon appears.
- Hotkeys work after launch without opening Settings.

### Phase 6: Rename `main` To `settings`

Purpose: make the architecture explicit.

Tasks:

- Change the Settings Window label from `main` to `settings`.
- Update frontend routing and backend label constants.
- Keep compatibility helper only if needed for migration.
- Remove stale `main` terminology from new code paths.

Verification:

- Settings opens via menu item.
- Settings close/hide works.
- Business windows do not reference `main`.

### Phase 7: Delete Obsolete Suppression Logic

Purpose: remove symptom-level patches once architecture no longer needs them.

Tasks:

- Audit `suppress_main_window_reopen_after_hotkey`.
- Remove suppression if `RunEvent::Reopen` is no-op in menu bar mode.
- Keep only business-window visibility rules that still protect dev/Dock mode, if retained.

Verification:

- No business hotkey opens Settings.
- No result-window focus opens Settings.
- Tests cover the intended lifecycle, not implementation timing.

## Testing Strategy

### Rust Unit Tests

Add or update tests for:

- Settings close hides instead of quits.
- Settings only shows through explicit Settings entrypoint.
- `RunEvent::Reopen` does not imply Settings in menu bar mode.
- Business-window labels are classified correctly.
- Menu item IDs dispatch to expected high-level actions.
- Startup shortcut registration does not require Settings Window.

### Frontend Tests

Add or update tests for:

- `appWindowRouting` recognizes `settings`, `capture-result`, `capture`, and pinned windows.
- Settings rendering is isolated from Result Window rendering.
- Result Window standalone presentation still works without Settings mounted.

### Manual macOS Verification

Required because activation policy and menu bar status items are OS behavior.

Checklist:

- Launch app: only menu bar icon appears.
- Dock icon is hidden in release menu-bar mode.
- Click menu bar "Settings...": Settings opens.
- Close Settings: app remains running.
- Trigger selection translation: only result window appears.
- Trigger screenshot translation: only capture overlay and result window appear.
- Trigger input translation: only result/input window appears.
- Trigger file OCR: no Settings Window appears.
- Click menu bar "Quit": app exits.
- Reopen app after quit: no Settings Window appears until requested.

## Risks And Mitigations

### Risk: Tauri Tray/Menu Behavior Differs Across Platforms

Mitigation:

- Treat macOS as the primary target for this refactor.
- Keep platform-specific app-shell behavior behind small functions.
- Preserve Windows/Linux tray behavior as best-effort unless explicitly scoped.

### Risk: Settings Frontend Currently Owns Hotkey Configuration Effects

Mitigation:

- Ensure startup hotkeys are registered from backend defaults/config, not from mounted Settings UI.
- Keep frontend hotkey configuration only for user edits.

### Risk: Activation Policy Breaks Capture Overlay

Mitigation:

- Keep capture overlay activation logic in `infrastructure/system/capture_window`.
- Do not merge capture overlay policy into app shell.
- Regression-test full-screen, multi-space, and overlay focus behavior manually.

### Risk: Lazy Settings Creation Breaks Existing Frontend Assumptions

Mitigation:

- Route Settings explicitly with `?window=settings`.
- Keep `main` label during early phases.
- Rename to `settings` only after tests cover routing.

### Risk: Removing Suppression Too Early Reintroduces Settings Reopen Bugs

Mitigation:

- Delete suppression only after menu-bar mode makes `RunEvent::Reopen` no-op.
- Keep lifecycle tests around business-window visibility until the behavior is proven.

## Acceptance Criteria

The refactor is complete when:

- App startup does not show Settings.
- Menu bar icon is the primary persistent app presence.
- Settings appears only from explicit Settings entrypoints.
- Business workflows do not reference or emit through Settings.
- `RunEvent::Reopen` no longer opens Settings in menu-bar mode.
- Existing Settings UI and Provider configuration still work.
- Existing translation, OCR, screenshot, and pinned image workflows still work.
- Test and build commands pass:
  - `cargo test -p snaplingo --lib`
  - `npm test`
  - `npm run build`
  - `npm run tauri:build`

## Recommended Implementation Order

Do this as several small commits:

1. `refactor: isolate settings window lifecycle`
2. `refactor: centralize business window labels`
3. `refactor: introduce menu bar app shell`
4. `feat: add menu bar actions`
5. `refactor: lazy create settings window`
6. `refactor: rename main window to settings`
7. `refactor: remove obsolete main reopen suppression`

Stop after each commit and run the focused tests for that phase. Do not combine Provider, translation, OCR, or Settings UI redesign work with this refactor.

