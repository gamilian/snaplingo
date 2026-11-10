//! Fixed preferences bound to the running application's service owners.
//! Native hosts confirm destructive actions and supply their own modal windows.
use super::general_controller::GeneralController;
use snaplingo_core::hotkeys::model::DEFAULT_HOTKEYS;
use snaplingo_core::hotkeys::model::OCR_CATEGORY;
use snaplingo_core::hotkeys::model::SCREENSHOT_CATEGORY;
use snaplingo_core::hotkeys::model::TRANSLATION_CATEGORY;
use snaplingo_core::hotkeys::HotkeyRecording;
use snaplingo_ui::{Action, Page, Size, Snapshot};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct PreferencesDependencies {
    pub configuration: Arc<snaplingo_core::settings::SettingsConfiguration>,
    pub settings: Arc<snaplingo_core::settings::SettingsApplication>,
    pub hotkeys: Arc<snaplingo_core::hotkeys::HotkeyRuntime>,
    pub general: Arc<GeneralController>,
    pub usage: Arc<super::capacity_usage::CapacityUsage>,
}

pub struct PreferencesSession {
    dependencies: PreferencesDependencies,
    executor: tokio::runtime::Handle,
    changed: Arc<dyn Fn() + Send + Sync>,
    pub general: Arc<GeneralController>,
    recording: HotkeyRecording,
    sequence: u64,
    hotkey_queue: Arc<Mutex<HotkeyQueue>>,
    usage: Arc<super::capacity_usage::CapacityUsage>,
}
impl PreferencesSession {
    pub fn new(
        dependencies: PreferencesDependencies,
        recording: HotkeyRecording,
        executor: tokio::runtime::Handle,
        changed: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self {
            general: dependencies.general.clone(),
            usage: dependencies.usage.clone(),
            dependencies,
            executor,
            changed,
            recording,
            sequence: 0,
            hotkey_queue: Default::default(),
        }
    }
    pub fn saving_hotkeys(&self) -> bool {
        self.hotkey_queue.lock().unwrap().running
    }
    pub fn recording_id(&self) -> Option<String> {
        self.recording.session().map(|session| session.id)
    }
    pub fn snapshot(&self) -> Result<Snapshot, String> {
        let saved = self
            .dependencies
            .configuration
            .snapshot()
            .map_err(|e| e.to_string())?;
        let hotkeys = self
            .dependencies
            .hotkeys
            .snapshot()
            .map_err(|e| e.to_string())?;
        let mut snapshot = crate::features::settings::presenter::project_snapshot(
            &saved,
            &hotkeys,
            &self.dependencies.hotkeys.default_snapshot(),
        );
        super::general::refresh(&self.general, &mut snapshot)?;
        self.usage.apply(&mut snapshot);
        if let Some(error) = self.hotkey_queue.lock().unwrap().error.clone() {
            snapshot.error = Some(error);
        }
        Ok(snapshot)
    }
    /// Window-only actions and General's async commands are handled by the host.
    pub fn dispatch(&mut self, page: Page, action: Action) -> Result<(), String> {
        let settings = &self.dependencies;
        match action {
            Action::EditGeneral { field, value } => {
                if !field.accepts(&value) {
                    return Err("设置值不符合此通用控件".into());
                }
                self.general.edit(
                    crate::features::settings::presenter::general_field(field),
                    crate::features::settings::presenter::general_value(value),
                )?;
            }
            Action::EditOcr { field, value } => {
                settings
                    .settings
                    .edit_ocr(
                        crate::features::settings::presenter::ocr_field(field),
                        crate::features::settings::presenter::general_value(value),
                    )
                    .map_err(|e| e.to_string())?;
            }
            Action::EditTranslation { field, value } => {
                settings
                    .settings
                    .edit_translation(
                        crate::features::settings::presenter::translation_field(field),
                        crate::features::settings::presenter::general_value(value),
                    )
                    .map_err(|e| e.to_string())?;
            }
            Action::EditScreenshot { field, value } => {
                settings
                    .configuration
                    .edit_screenshot(
                        crate::features::settings::presenter::screenshot_field(field),
                        crate::features::settings::presenter::general_value(value),
                    )
                    .map_err(|e| e.to_string())?;
                self.recording.cancel_for_window("settings");
            }
            Action::BeginHotkeyRecording { .. } | Action::BeginLocalShortcutRecording { .. } => {
                static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                self.sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
                    let runtime = self.dependencies.hotkeys.clone();
                    let changed = self.changed.clone();
                    self.executor.spawn_blocking(move || loop {
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
                        changed();
                    });
                }
            }
            Action::CheckHotkeyConflicts => {
                crate::features::settings::presenter::validate_hotkey_conflicts(
                    &settings.hotkeys.snapshot().map_err(|e| e.to_string())?,
                )?
            }
            Action::Navigate(_) => self.recording.cancel_for_window("settings"),
            _ => return Err("此操作需要窗口宿主提供对话框或异步工作流".into()),
        }
        Ok(())
    }
    pub fn interface_scale(&self, display: Size) -> f32 {
        let mode = self
            .dependencies
            .configuration
            .snapshot()
            .map(|s| s.screenshot.capture_ui_scale)
            .unwrap_or_else(|_| "auto".into());
        snaplingo_core::capture::interface_scale(
            &mode,
            Some(&snaplingo_core::capture::model::LogicalRect {
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
pub fn hotkey_target(page: Page, index: usize) -> Result<(&'static str, &'static str), String> {
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
pub fn hotkey_mutation(
    runtime: &snaplingo_core::hotkeys::HotkeyRuntime,
    page: Page,
    action: Action,
) -> Result<(), String> {
    let category = hotkey_target(page, 0)?.0;
    let mut next = runtime.snapshot().map_err(|e| e.to_string())?;
    use snaplingo_core::hotkeys::model::hotkey_category;
    use snaplingo_core::hotkeys::model::hotkey_category_mut;
    let keys = hotkey_category_mut(&mut next, category).map_err(|e| e.to_string())?;
    match &action {
        Action::SaveHotkey { index, shortcut } => {
            keys.insert(hotkey_target(page, *index)?.1.into(), shortcut.clone());
        }
        Action::ResetHotkey { index } => {
            let target = hotkey_target(page, *index)?.1;
            keys.insert(
                target.into(),
                hotkey_category(&runtime.default_snapshot(), category).unwrap()[target].clone(),
            );
        }
        Action::ResetHotkeys => {
            *keys = hotkey_category(&runtime.default_snapshot(), category)
                .unwrap()
                .clone();
        }
        _ => unreachable!(),
    }
    crate::features::settings::presenter::validate_hotkey_conflicts(&next)?;
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
