use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use base64::Engine;

use super::scrolling_image::{ScrollingImage, ScrollingStep};
use super::CaptureSessionRuntime;
use crate::capture::model::CaptureSessionId;
use crate::capture::model::DesktopPosition;
use crate::capture::model::LogicalPoint;
use crate::capture::model::LogicalRect;
use crate::capture::model::MonitorSnapshot;
use crate::capture::model::PhysicalPoint;
use crate::capture::model::ScrollingCaptureOutputAction;
use crate::capture::model::ScrollingCaptureStatus;
use crate::capture::model::ScrollingCaptureView;
use crate::capture::model::ScrollingShortcutAction;

#[derive(Clone)]
pub struct ScrollingCaptureGeometry {
    pub viewport: LogicalRect,
    pub desktop_scale: Option<f64>,
    pub monitors: Vec<MonitorSnapshot>,
    pub displays: Vec<LogicalRect>,
}

#[async_trait::async_trait]
pub trait ScrollingCaptureHost: Send + Sync {
    async fn open(
        &self,
        view: &ScrollingCaptureView,
        geometry: &ScrollingCaptureGeometry,
    ) -> crate::Result<ScrollingWindowOptions>;
    async fn capture_snapshots(
        &self,
        id: &str,
        geometry: &ScrollingCaptureGeometry,
    ) -> crate::Result<Vec<MonitorSnapshot>>;
    // None rolls back an opening failure; Some ends an adopted capture.
    async fn close(
        &self,
        id: &str,
        session_id: &CaptureSessionId,
        completed: Option<bool>,
    ) -> crate::Result<()>;
    fn publish(&self, _view: &ScrollingCaptureView) {}
    async fn set_preview_visible(&self, _id: &str, _visible: bool) -> crate::Result<bool> {
        Ok(false)
    }
}

pub struct ScrollingWindowOptions {
    pub shortcuts: Vec<ScrollingShortcutAction>,
    pub preview_expanded: bool,
}

pub(super) struct UnconfiguredScrollingCaptureHost;
#[async_trait::async_trait]
impl ScrollingCaptureHost for UnconfiguredScrollingCaptureHost {
    async fn open(
        &self,
        _: &ScrollingCaptureView,
        _: &ScrollingCaptureGeometry,
    ) -> crate::Result<ScrollingWindowOptions> {
        Err("Scrolling capture window is not configured".into())
    }
    async fn capture_snapshots(
        &self,
        _: &str,
        _: &ScrollingCaptureGeometry,
    ) -> crate::Result<Vec<MonitorSnapshot>> {
        Err("Scrolling capture source is not configured".into())
    }
    async fn close(&self, _: &str, _: &CaptureSessionId, _: Option<bool>) -> crate::Result<()> {
        Ok(())
    }
}

pub(super) struct ScrollingCaptureTask {
    id: String,
    revision: u64,
    session_id: CaptureSessionId,
    geometry: ScrollingCaptureGeometry,
    image: ScrollingImage,
    preview: String,
    preview_dirty: bool,
    status: ScrollingCaptureStatus,
    hint: Option<String>,
    shortcuts: Vec<ScrollingShortcutAction>,
    preview_expanded: bool,
    preview_available: bool,
    ui_scale: f64,
}

impl ScrollingCaptureTask {
    fn view(&mut self) -> crate::Result<ScrollingCaptureView> {
        if self.preview_dirty {
            let thumbnail = self.image.thumbnail(
                (240.0 * self.ui_scale).round() as u32,
                (360.0 * self.ui_scale).round() as u32,
            );
            let mut png = Vec::new();
            image::DynamicImage::ImageRgba8(thumbnail)
                .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
                .map_err(|error| format!("Failed to encode scrolling capture preview: {error}"))?;
            self.preview = base64::engine::general_purpose::STANDARD.encode(png);
            self.preview_dirty = false;
        }
        let (width, height) = self.image.dimensions();
        Ok(ScrollingCaptureView {
            id: self.id.clone(),
            revision: self.revision,
            width,
            height,
            can_undo: self.image.can_undo(),
            preview_base64: self.preview.clone(),
            status: self.status.clone(),
            hint: self.hint.clone(),
            shortcut_actions: self.shortcuts.clone(),
            preview_expanded: self.preview_expanded,
            preview_available: self.preview_available,
            ui_scale: self.ui_scale,
        })
    }
}

impl CaptureSessionRuntime {
    pub fn with_scrolling_host(mut self, host: Arc<dyn ScrollingCaptureHost>) -> Self {
        self.scrolling_host = host;
        self
    }

    pub async fn begin_scrolling_capture(
        &self,
        session_id: CaptureSessionId,
        viewport: LogicalRect,
    ) -> crate::Result<ScrollingCaptureView> {
        let mut slot = self.scrolling.lock().await;
        if let Some(task) = slot.as_mut() {
            if task.session_id == session_id {
                return task.view();
            }
            return Err("Another scrolling capture is already active".into());
        }
        let _startup = self.startup.lock().await;
        let generation = *self.ocr_generation.borrow();
        if self.presentations.lock().await.get(&session_id) != Some(&generation) {
            return Err("Capture session is no longer current".into());
        }
        self.ensure_selection_snapshots_ready(&session_id, &viewport)?;
        let session = self.sessions.get_session(&session_id)?;
        let plan = super::render::capture_image_composition_plan(
            &viewport,
            &session.snapshots,
            session.desktop_scale,
        )
        .map_err(crate::AppError::System)?;
        let settings = self
            .settings
            .as_ref()
            .map(|settings| settings.snapshot())
            .transpose()?
            .unwrap_or_default()
            .screenshot;
        ScrollingImage::validate_dimensions_with_limit(
            plan.width,
            plan.height,
            settings.longshot_max_height,
        )?;
        let first = super::render::compose_snapshot_image(
            &self.image_composition,
            &session.snapshots,
            session.desktop_scale,
            &viewport,
            &[],
            None,
        )?;
        let monitors = session
            .layout_snapshots
            .iter()
            .filter(|monitor| intersects(&monitor.logical_bounds, &viewport))
            .cloned()
            .map(|mut monitor| {
                monitor.png_data.clear();
                monitor
            })
            .collect();
        let geometry = ScrollingCaptureGeometry {
            displays: session
                .layout_snapshots
                .iter()
                .map(|monitor| monitor.logical_bounds.clone())
                .collect(),
            viewport,
            desktop_scale: session.desktop_scale,
            monitors,
        };
        let ui_scale = super::interface_scale::interface_scale(
            &settings.capture_ui_scale,
            geometry
                .monitors
                .first()
                .map(|monitor| &monitor.logical_bounds),
        );
        let mut task = ScrollingCaptureTask {
            id: format!("scrolling-{}", session_id.0),
            revision: 0,
            session_id,
            geometry,
            image: ScrollingImage::with_max_height(first, settings.longshot_max_height)?,
            preview: String::new(),
            preview_dirty: true,
            status: ScrollingCaptureStatus::Capturing,
            hint: None,
            shortcuts: Vec::new(),
            preview_expanded: false,
            preview_available: false,
            ui_scale,
        };
        let view = task.view()?;
        self.host.hide_capture_window().await?;
        match self.scrolling_host.open(&view, &task.geometry).await {
            Ok(options) => {
                task.shortcuts = options.shortcuts;
                task.preview_expanded = options.preview_expanded;
                task.preview_available = options.preview_expanded;
            }
            Err(error) => {
                let _ = self
                    .scrolling_host
                    .close(&task.id, &task.session_id, None)
                    .await;
                let _ = self.reveal_capture_window(Some(&task.session_id)).await;
                return Err(error);
            }
        }
        *slot = Some(task);
        let view = slot.as_mut().unwrap().view()?;
        self.scrolling_host.publish(&view);
        Ok(view)
    }

    pub async fn scrolling_capture_view(&self, id: &str) -> crate::Result<ScrollingCaptureView> {
        let mut slot = self.scrolling.lock().await;
        let task = current_task(&mut slot, id)?;
        task.view()
    }

    async fn update_scrolling_frame(&self, task: &mut ScrollingCaptureTask) -> crate::Result<()> {
        let captured = self
            .scrolling_host
            .capture_snapshots(&task.id, &task.geometry)
            .await?;
        let mut snapshots = task.geometry.monitors.clone();
        for snapshot in &mut snapshots {
            let fresh = captured
                .iter()
                .find(|candidate| candidate.id == snapshot.id)
                .ok_or("Scrolling capture display is unavailable")?;
            if fresh.physical_bounds != snapshot.physical_bounds {
                return Err("Display layout changed; select the scrolling region again".into());
            }
            snapshot.png_data = fresh.png_data.clone();
        }
        let frame = super::render::compose_snapshot_image(
            &self.image_composition,
            &snapshots,
            task.geometry.desktop_scale,
            &task.geometry.viewport,
            &[],
            None,
        )?;
        task.status = ScrollingCaptureStatus::Capturing;
        task.hint = None;
        match task.image.push(frame) {
            ScrollingStep::Appended(_) => task.preview_dirty = true,
            ScrollingStep::Still => {}
            ScrollingStep::Failed => {
                task.hint = Some("画面无法匹配，请放慢滚动或滚回上次位置。".into())
            }
            ScrollingStep::Limit => {
                task.preview_dirty = true;
                task.status = ScrollingCaptureStatus::Limit;
                task.hint = Some("已达到长图上限，正在复制并保存 PNG。".into());
            }
        }
        Ok(())
    }

    pub async fn capture_scrolling_frame(&self, id: &str) -> crate::Result<ScrollingCaptureView> {
        let mut slot = self.scrolling.lock().await;
        let task = current_task(&mut slot, id)?;
        if let Err(error) = self.update_scrolling_frame(task).await {
            task.status = ScrollingCaptureStatus::Error;
            task.hint = Some(format!("采集失败，已保留完成内容：{error}"));
        }
        task.revision += 1;
        if task.status == ScrollingCaptureStatus::Limit {
            match self
                .output_scrolling_task(task, ScrollingCaptureOutputAction::Complete)
                .await
            {
                Ok(()) => {
                    task.status = ScrollingCaptureStatus::Completed;
                    let view = task.view()?;
                    self.scrolling_host.publish(&view);
                    let completed = slot.take().unwrap();
                    drop(slot);
                    if let Err(error) = self.close_completed_scrolling_task(completed).await {
                        log::warn!("Scrolling image saved, but window cleanup failed: {error}");
                    }
                    return Ok(view);
                }
                Err(error) => {
                    task.status = ScrollingCaptureStatus::Error;
                    task.hint = Some(format!("自动输出失败，长图已保留：{error}"));
                }
            }
        }
        let view = task.view()?;
        self.scrolling_host.publish(&view);
        Ok(view)
    }

    pub async fn undo_scrolling_capture(&self, id: &str) -> crate::Result<ScrollingCaptureView> {
        let mut slot = self.scrolling.lock().await;
        let task = current_task(&mut slot, id)?;
        if task.image.undo() {
            task.preview_dirty = true;
        }
        task.status = ScrollingCaptureStatus::Capturing;
        task.hint = None;
        task.revision += 1;
        let view = task.view()?;
        self.scrolling_host.publish(&view);
        Ok(view)
    }

    pub async fn set_scrolling_preview_visible(
        &self,
        id: &str,
        visible: bool,
    ) -> crate::Result<bool> {
        let mut slot = self.scrolling.lock().await;
        let task = current_task(&mut slot, id)?;
        task.preview_expanded = self.scrolling_host.set_preview_visible(id, visible).await?;
        task.revision += 1;
        let view = task.view()?;
        self.scrolling_host.publish(&view);
        Ok(task.preview_expanded)
    }

    pub async fn finish_scrolling_capture(
        &self,
        id: &str,
        action: ScrollingCaptureOutputAction,
    ) -> crate::Result<()> {
        let mut slot = self.scrolling.lock().await;
        let task = current_task(&mut slot, id)?;
        // Include a final frame when the user ends immediately after scrolling.
        if !task.image.at_limit() {
            let _ = self.update_scrolling_frame(task).await;
        }
        self.output_scrolling_task(task, action).await?;
        task.status = ScrollingCaptureStatus::Completed;
        task.revision += 1;
        self.scrolling_host.publish(&task.view()?);
        let task = slot.take().unwrap();
        drop(slot);
        self.close_completed_scrolling_task(task).await
    }

    async fn output_scrolling_task(
        &self,
        task: &ScrollingCaptureTask,
        action: ScrollingCaptureOutputAction,
    ) -> crate::Result<()> {
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(task.image.image())
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|error| format!("Failed to encode scrolling capture: {error}"))?;
        match action {
            ScrollingCaptureOutputAction::Complete => {
                let settings = self
                    .settings
                    .as_ref()
                    .map(|settings| settings.snapshot())
                    .transpose()?
                    .unwrap_or_default()
                    .screenshot;
                let path = self.output.quick_capture_save_path(
                    Some(&settings.save_path),
                    "png",
                    "timestamp",
                    "SnapLingo",
                );
                let copied = self.output.copy_png(&png).await;
                let saved = self.output.save_png(&png, &path).await;
                match (copied, saved) {
                    (Ok(()), Ok(_)) => Ok(()),
                    (Err(copy), Ok(path)) => {
                        Err(format!("图片已保存到 {}，复制失败：{copy}", path.display()).into())
                    }
                    (Ok(()), Err(save)) => Err(format!("图片已复制，保存失败：{save}").into()),
                    (Err(copy), Err(save)) => {
                        Err(format!("复制失败：{copy}；保存失败：{save}").into())
                    }
                }
            }
            ScrollingCaptureOutputAction::Copy => self.output.copy_png(&png).await,
            ScrollingCaptureOutputAction::Save {
                path,
                format,
                quality,
                copy_after_save,
                remember_choice,
            } => {
                let path = self
                    .output
                    .save_image(&png, Path::new(&path), &format, quality)
                    .await?;
                if remember_choice {
                    if let Some(settings) = &self.settings {
                        let (_, format) =
                            super::output::capture_save_path_and_format(&path, &format)?;
                        if let Err(error) = settings.record_screenshot_save_choice(&path, format) {
                            log::warn!(
                                "Image saved, but could not remember the save choice: {error}"
                            );
                        }
                    }
                }
                if copy_after_save {
                    self.output.copy_png(&png).await?;
                }
                Ok(())
            }
            ScrollingCaptureOutputAction::Pin => {
                let rect = &task.geometry.viewport;
                let origin = match task.geometry.desktop_scale {
                    Some(scale) => DesktopPosition::Physical(PhysicalPoint {
                        x: (rect.x * scale).round() as i32,
                        y: (rect.y * scale).round() as i32,
                    }),
                    None => DesktopPosition::Logical(LogicalPoint {
                        x: rect.x,
                        y: rect.y,
                    }),
                };
                self.pin_output.pin_png(png, Some(origin)).await
            }
        }
    }

    async fn close_completed_scrolling_task(
        &self,
        task: ScrollingCaptureTask,
    ) -> crate::Result<()> {
        let close = self
            .scrolling_host
            .close(&task.id, &task.session_id, Some(true))
            .await;
        let cleanup = self.cancel_capture_session(&task.session_id).await;
        close.and(cleanup)
    }

    pub async fn cancel_scrolling_capture(&self, id: &str) -> crate::Result<()> {
        let mut slot = self.scrolling.lock().await;
        let task = if slot.as_ref().is_some_and(|task| task.id == id) {
            slot.take()
        } else {
            None
        };
        if let Some(task) = task {
            let close = self
                .scrolling_host
                .close(&task.id, &task.session_id, Some(false))
                .await;
            let cleanup = self.cancel_capture_session(&task.session_id).await;
            close.and(cleanup)
        } else {
            Ok(())
        }
    }

    pub(super) async fn cancel_active_scrolling_capture(&self) -> crate::Result<()> {
        let id = self
            .scrolling
            .lock()
            .await
            .as_ref()
            .map(|task| task.id.clone());
        if let Some(id) = id {
            self.cancel_scrolling_capture(&id).await?;
        }
        Ok(())
    }
}

fn current_task<'a>(
    slot: &'a mut Option<ScrollingCaptureTask>,
    id: &str,
) -> crate::Result<&'a mut ScrollingCaptureTask> {
    slot.as_mut()
        .filter(|task| task.id == id)
        .ok_or_else(|| "Scrolling capture is no longer active".into())
}

pub fn intersects(a: &LogicalRect, b: &LogicalRect) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}
