//! Fixed preferences bound to the running application's service owners.
//! Native hosts confirm destructive actions and supply their own modal windows.
use super::general_controller::GeneralController;
use crate::application::hotkeys::HotkeyRecording;
use crate::domain::hotkey_config::{
    DEFAULT_HOTKEYS, OCR_CATEGORY, SCREENSHOT_CATEGORY, TRANSLATION_CATEGORY,
};
use snaplingo_ui::{Action, Page, Size, Snapshot};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub(crate) struct PreferencesSession {
    state: Arc<crate::AppState>,
    pub(crate) general: Arc<GeneralController>,
    recording: HotkeyRecording,
    sequence: u64,
    hotkey_queue: Arc<Mutex<HotkeyQueue>>,
    usage: Arc<super::capacity_usage::CapacityUsage>,
    _usage_subscription: crate::infrastructure::desktop_events::DesktopSubscription,
}
impl PreferencesSession {
    pub(crate) fn new(
        state: Arc<crate::AppState>,
        recording: HotkeyRecording,
        open_notices: Arc<dyn Fn() -> crate::Result<()> + Send + Sync>,
    ) -> Self {
        let general =
            super::general::from_state(&state, env!("CARGO_PKG_VERSION").into(), open_notices);
        let notify = state.desktop_events.clone();
        let (usage, usage_subscription) = super::capacity_usage::CapacityUsage::subscribe(
            state.library_index.clone(),
            &state.desktop_events,
            move || {
                notify.publish(crate::infrastructure::desktop_events::DesktopEvent::SettingsChanged)
            },
        );
        Self {
            state,
            general,
            recording,
            sequence: 0,
            hotkey_queue: Default::default(),
            usage,
            _usage_subscription: usage_subscription,
        }
    }
    pub(crate) fn snapshot(&self) -> Result<Snapshot, String> {
        let saved = self
            .state
            .settings
            .configuration
            .snapshot()
            .map_err(|e| e.to_string())?;
        let hotkeys = self
            .state
            .settings
            .hotkeys
            .snapshot()
            .map_err(|e| e.to_string())?;
        let mut snapshot = snaplingo_settings::project_snapshot(
            &saved,
            &hotkeys,
            &self.state.settings.hotkeys.default_snapshot(),
        );
        super::general::refresh(&self.general, &mut snapshot)?;
        self.usage.apply(&mut snapshot);
        if let Some(error) = self.hotkey_queue.lock().unwrap().error.clone() {
            snapshot.error = Some(error);
        }
        Ok(snapshot)
    }
    /// Window-only actions and General's async commands are handled by the host.
    pub(crate) fn dispatch(&mut self, page: Page, action: Action) -> Result<(), String> {
        let settings = &self.state.settings;
        match action {
            Action::EditGeneral { field, value } => {
                if !field.accepts(&value) {
                    return Err("设置值不符合此通用控件".into());
                }
                self.general.edit(
                    snaplingo_settings::general_field(field),
                    snaplingo_settings::general_value(value),
                )?;
            }
            Action::EditOcr { field, value } => {
                settings
                    .configuration
                    .edit_ocr(
                        snaplingo_settings::ocr_field(field),
                        snaplingo_settings::general_value(value),
                    )
                    .map_err(|e| e.to_string())?;
            }
            Action::EditTranslation { field, value } => {
                settings
                    .configuration
                    .edit_translation(
                        snaplingo_settings::translation_field(field),
                        snaplingo_settings::general_value(value),
                    )
                    .map_err(|e| e.to_string())?;
            }
            Action::EditScreenshot { field, value } => {
                settings
                    .configuration
                    .edit_screenshot(
                        snaplingo_settings::screenshot_field(field),
                        snaplingo_settings::general_value(value),
                    )
                    .map_err(|e| e.to_string())?;
                self.recording.cancel_for_window("settings");
            }
            Action::BeginHotkeyRecording { .. } | Action::BeginLocalShortcutRecording { .. } => {
                self.sequence += 1;
                self.recording
                    .begin("settings", format!("shared-preferences:{}", self.sequence));
            }
            Action::EndHotkeyRecording => self.recording.cancel_for_window("settings"),
            action @ (Action::SaveHotkey { .. }
            | Action::ResetHotkey { .. }
            | Action::ResetHotkeys) => {
                hotkey_target(page, 0)?;
                self.recording.cancel_for_window("settings");
                let mut queue = self.hotkey_queue.lock().unwrap();
                queue.pending.push_back((page, action));
                if !queue.running {
                    queue.running = true;
                    let queue = self.hotkey_queue.clone();
                    let runtime = self.state.settings.hotkeys.clone();
                    let state = self.state.clone();
                    crate::infrastructure::runtime::spawn_blocking(move || loop {
                        let next = {
                            let mut queue = queue.lock().unwrap();
                            if queue.disposed {
                                queue.pending.clear();
                            }
                            let next = queue.pending.pop_front();
                            if next.is_none() {
                                queue.running = false;
                            }
                            next
                        };
                        let Some((page, action)) = next else { break };
                        let result = hotkey_mutation(&runtime, page, action);
                        // Registration calls can marshal synchronously to the OS
                        // UI thread, so this worker never owns a Settings lock.
                        queue.lock().unwrap().error = result.err();
                        state.desktop_events.publish(
                            crate::infrastructure::desktop_events::DesktopEvent::HotkeysChanged,
                        );
                    });
                }
            }
            Action::CheckHotkeyConflicts => snaplingo_settings::validate_hotkey_conflicts(
                &settings.hotkeys.snapshot().map_err(|e| e.to_string())?,
            )?,
            Action::Navigate(_) => self.recording.cancel_for_window("settings"),
            _ => return Err("此操作需要窗口宿主提供对话框或异步工作流".into()),
        }
        Ok(())
    }
    pub(crate) fn interface_scale(&self, display: Size) -> f32 {
        let mode = self
            .state
            .settings
            .configuration
            .snapshot()
            .map(|s| s.screenshot.capture_ui_scale)
            .unwrap_or_else(|_| "auto".into());
        crate::application::capture::interface_scale(
            &mode,
            Some(&crate::domain::capture::LogicalRect {
                x: 0.,
                y: 0.,
                width: display.width.into(),
                height: display.height.into(),
            }),
        ) as f32
    }
}
impl Drop for PreferencesSession {
    fn drop(&mut self) {
        self.recording.cancel_for_window("settings");
        self.hotkey_queue.lock().unwrap().disposed = true;
        self.general.invalidate_pending_updates();
        self.usage.dispose();
    }
}
fn hotkey_target(page: Page, index: usize) -> Result<(&'static str, &'static str), String> {
    let category = match page {
        Page::Screenshot => SCREENSHOT_CATEGORY,
        Page::Ocr => OCR_CATEGORY,
        Page::Translation => TRANSLATION_CATEGORY,
        _ => return Err("当前页面没有快捷键配置".into()),
    };
    DEFAULT_HOTKEYS
        .iter()
        .filter(|h| h.category == category)
        .nth(index)
        .map(|h| (category, h.action))
        .ok_or_else(|| "Unknown Settings shortcut".into())
}

#[derive(Default)]
struct HotkeyQueue {
    pending: VecDeque<(Page, Action)>,
    running: bool,
    disposed: bool,
    error: Option<String>,
}
fn hotkey_mutation(
    runtime: &crate::application::hotkeys::HotkeyRuntime,
    page: Page,
    action: Action,
) -> Result<(), String> {
    let category = hotkey_target(page, 0)?.0;
    match action {
        Action::SaveHotkey { index, shortcut } => runtime
            .update_hotkey(
                category.into(),
                hotkey_target(page, index)?.1.into(),
                shortcut,
            )
            .map(|_| ()),
        Action::ResetHotkey { index } => runtime
            .reset_hotkey(category.into(), hotkey_target(page, index)?.1.into())
            .map(|_| ()),
        Action::ResetHotkeys => runtime.reset_category(category.into()).map(|_| ()),
        _ => unreachable!(),
    }
    .map_err(|e| e.to_string())
}
