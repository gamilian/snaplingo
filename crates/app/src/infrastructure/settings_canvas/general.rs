//! Reuses the production General workflow behind the shared Rust drawing.
use std::sync::Arc;

use snaplingo_ui::{GeneralCommand, GeneralRuntime, Snapshot};

use super::general_controller::{GeneralAction, GeneralController, GeneralServices, UpdatePhase};
use crate::application::SystemPermission;

pub(crate) fn from_state(
    state: &crate::AppState,
    version: String,
    open_notices: Arc<dyn Fn() -> crate::Result<()> + Send + Sync>,
) -> Arc<GeneralController> {
    Arc::new(GeneralController::new(GeneralServices {
        configuration: state.settings.configuration.clone(),
        settings: state.settings.application.clone(),
        history: state.history.history.clone(),
        permissions: state.permissions.clone(),
        updates: state.updates.clone(),
        version,
        open_url: Arc::new(crate::infrastructure::system::external_open::open_url),
        open_notices,
    }))
}

pub(crate) fn refresh(
    controller: &GeneralController,
    snapshot: &mut Snapshot,
) -> Result<(), String> {
    let projection = controller.projection()?;
    let description = match projection.update_phase {
        UpdatePhase::Checking => "正在检查更新…".into(),
        UpdatePhase::Downloading => "正在下载并校验安装包…".into(),
        UpdatePhase::Idle if projection.downloaded_path.is_some() => "安装包已下载并打开".into(),
        UpdatePhase::Idle => match &projection.update {
            Some(status) if status.available => format!(
                "发现新版本 {}",
                status.latest_version.as_deref().unwrap_or("")
            ),
            Some(status) if status.latest_version.is_some() => "当前已是最新版本".into(),
            Some(_) => "暂时没有已发布的正式版本".into(),
            None => "打开设置时自动查找最新正式版".into(),
        },
    };
    let context = projection.permission_context.as_ref();
    snapshot.general_runtime = GeneralRuntime {
        permissions_required: context.is_none_or(|c| c.platform == "macos"),
        screen_recording: projection.permissions.as_ref().map(|s| s.screen_recording),
        accessibility: projection.permissions.as_ref().map(|s| s.accessibility),
        permission_error: None,
        permission_busy: projection.permission_busy,
        app_path: context.and_then(|c| c.app_path.clone()),
        needs_installation: context.is_some_and(|c| c.needs_installation),
        can_reset_permissions: context.is_some_and(|c| c.can_reset),
        logs: projection
            .logs
            .iter()
            .map(|e| format!("[{}] {}", e.level, e.message))
            .collect(),
        version: projection.version,
        update_description: description,
        update_busy: projection.update_phase != UpdatePhase::Idle,
        update_download_available: projection
            .update
            .as_ref()
            .is_some_and(|s| s.available && s.download_size.is_some()),
        update_available: projection.update.as_ref().is_some_and(|s| s.available),
        update_size: projection.update.as_ref().and_then(|s| s.download_size),
        update_notes: projection
            .update
            .as_ref()
            .map(|s| s.notes.clone())
            .unwrap_or_default(),
        downloaded_path: projection.downloaded_path,
    };
    if snapshot.error.is_none() {
        snapshot.error = projection.error;
    }
    Ok(())
}

pub(crate) fn action(command: GeneralCommand) -> GeneralAction {
    match command {
        GeneralCommand::RefreshPermissions => GeneralAction::RefreshPermissions,
        GeneralCommand::RequestScreenRecording => {
            GeneralAction::RequestPermission(SystemPermission::ScreenRecording)
        }
        GeneralCommand::RequestAccessibility => {
            GeneralAction::RequestPermission(SystemPermission::Accessibility)
        }
        GeneralCommand::ResetScreenRecording => {
            GeneralAction::ResetPermission(SystemPermission::ScreenRecording)
        }
        GeneralCommand::ResetAccessibility => {
            GeneralAction::ResetPermission(SystemPermission::Accessibility)
        }
        GeneralCommand::Restart => GeneralAction::Restart,
        GeneralCommand::RefreshLogs => GeneralAction::RefreshLogs,
        GeneralCommand::ClearLogs => GeneralAction::ClearLogs,
        GeneralCommand::ClearHistory => GeneralAction::ClearHistory,
        GeneralCommand::OpenProject => GeneralAction::OpenProject,
        GeneralCommand::ShowNotices => GeneralAction::OpenNotices,
        GeneralCommand::CheckUpdates => GeneralAction::CheckUpdates,
        GeneralCommand::OpenReleasePage => GeneralAction::OpenReleases,
        GeneralCommand::DownloadUpdate => GeneralAction::DownloadUpdate,
    }
}
pub(crate) fn confirmation(command: GeneralCommand) -> Option<&'static str> {
    match command {
        GeneralCommand::ClearLogs => Some("确定要清空所有应用日志吗？"),
        GeneralCommand::ClearHistory => {
            Some("确定要清空全部翻译和 OCR 历史吗？独立收藏不会被删除。")
        }
        GeneralCommand::ResetScreenRecording => {
            Some("将重置 SnapLingo 的“屏幕录制”权限，之后需要重新允许。")
        }
        GeneralCommand::ResetAccessibility => {
            Some("将重置 SnapLingo 的“辅助功能”权限，之后需要重新允许。")
        }
        _ => None,
    }
}
