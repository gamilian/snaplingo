//! Shared completion policy for quick capture and both native annotation hosts.
use std::path::PathBuf;

use async_trait::async_trait;
use snaplingo_core::capture::model::{
    AnnotationCommand, CaptureOutputAction, CaptureSessionId, LogicalRect,
};
use snaplingo_core::capture::{CaptureOcrTarget, CaptureSelectionMode};
use snaplingo_core::settings::model::ScreenshotSettings;
use snaplingo_ui::capture::CaptureCommand;

use super::Services;
use crate::Result;

pub(super) struct Request<'a> {
    pub id: &'a CaptureSessionId,
    pub mode: CaptureSelectionMode,
    pub command: CaptureCommand,
    pub rect: &'a LogicalRect,
    pub annotations: &'a [AnnotationCommand],
    pub include_cursor: bool,
}

/// Native effects operate only on the supplied session. Dialog/print effects may
/// suspend its presentation; completion owns cancellation recovery. Errors return
/// to the caller, which restores presentation and reports the error together.
#[async_trait]
pub(super) trait Effects: Sync {
    async fn is_current(&self, id: &CaptureSessionId) -> Result<bool>;
    async fn save_path(
        &self,
        id: &CaptureSessionId,
        default_path: PathBuf,
        format: &str,
    ) -> Result<Option<PathBuf>>;
    async fn print(&self, id: &CaptureSessionId, png: Vec<u8>) -> Result<bool>;
    async fn resume(&self, id: &CaptureSessionId) -> Result<()>;
    async fn close(&self, id: &CaptureSessionId) -> Result<()>;
}

pub(super) async fn complete(
    services: &Services,
    request: Request<'_>,
    effects: &dyn Effects,
) -> Result<()> {
    if !effects.is_current(request.id).await? {
        return Ok(());
    }
    let runtime = services.runtime()?;
    let preferences = services.settings.snapshot()?;
    let target = match request.command {
        CaptureCommand::Primary => match request.mode {
            CaptureSelectionMode::Ocr => Some(CaptureOcrTarget::OcrWindow),
            CaptureSelectionMode::SilentOcr => Some(CaptureOcrTarget::Clipboard),
            CaptureSelectionMode::Translate => Some(CaptureOcrTarget::TranslationWindow),
            CaptureSelectionMode::Edit | CaptureSelectionMode::Copy => None,
        },
        CaptureCommand::Ocr => Some(CaptureOcrTarget::OcrWindow),
        _ => None,
    };
    if let Some(target) = target {
        runtime
            .prepare_capture_ocr(
                request.id,
                request.rect,
                request.annotations,
                target,
                preferences.ocr,
            )
            .await?;
        return runtime.complete_capture_ocr(request.id).await;
    }

    let screenshot = preferences.screenshot;
    let output = match request.command {
        CaptureCommand::Primary | CaptureCommand::Copy => CaptureOutputAction::Copy,
        CaptureCommand::Pin => CaptureOutputAction::Pin,
        CaptureCommand::QuickSave => save_action(
            services.output.quick_capture_save_path(
                Some(&screenshot.save_path),
                &screenshot.format,
                &screenshot.naming_rule,
                &screenshot.custom_file_name,
            ),
            &screenshot,
            false,
        ),
        CaptureCommand::Save => {
            let format = screenshot
                .last_save_format
                .as_deref()
                .unwrap_or(&screenshot.format);
            let directory = screenshot
                .last_save_directory
                .as_deref()
                .unwrap_or(&screenshot.save_path);
            let default_path = services.output.dialog_capture_save_path(
                Some(directory),
                screenshot
                    .last_save_directory
                    .as_ref()
                    .map(|_| screenshot.save_path.as_str()),
                format,
                &screenshot.naming_rule,
                &screenshot.custom_file_name,
            );
            let selected = effects.save_path(request.id, default_path, format).await?;
            // A dialog may outlive the capture that opened it.
            if !effects.is_current(request.id).await? {
                return Ok(());
            }
            let Some(path) = selected else {
                return effects.resume(request.id).await;
            };
            save_action(path, &screenshot, true)
        }
        CaptureCommand::Scrolling => {
            return runtime
                .begin_scrolling_capture(request.id.clone(), request.rect.clone())
                .await
                .map(|_| ());
        }
        CaptureCommand::Favorite => {
            services
                .favorites
                .upgrade()
                .ok_or("Screenshot favorite runtime has stopped")?
                .add_selection(
                    request.id,
                    request.rect,
                    request.annotations,
                    request.include_cursor,
                )
                .await?;
            runtime.remember_capture_selection(request.rect)?;
            effects.close(request.id).await?;
            return runtime.cancel_capture_session(request.id).await;
        }
        CaptureCommand::Print => {
            let png = runtime.render_png(
                request.id,
                request.rect,
                request.annotations,
                request.include_cursor,
            )?;
            let completed = effects.print(request.id, png).await?;
            if !effects.is_current(request.id).await? {
                return Ok(());
            }
            if !completed {
                return effects.resume(request.id).await;
            }
            effects.close(request.id).await?;
            return runtime.cancel_capture_session(request.id).await;
        }
        _ => return Err("Unknown capture completion command".into()),
    };
    runtime
        .complete_annotated_capture_selection(
            request.id,
            request.rect,
            request.annotations,
            request.include_cursor,
            output,
        )
        .await
}

fn save_action(
    path: PathBuf,
    settings: &ScreenshotSettings,
    remember_choice: bool,
) -> CaptureOutputAction {
    CaptureOutputAction::Save {
        path: path.to_string_lossy().into_owned(),
        format: if remember_choice {
            settings
                .last_save_format
                .as_ref()
                .unwrap_or(&settings.format)
        } else {
            &settings.format
        }
        .clone(),
        quality: settings.quality,
        copy_after_save: settings.auto_copy,
        remember_choice,
    }
}

#[cfg(test)]
mod tests;
