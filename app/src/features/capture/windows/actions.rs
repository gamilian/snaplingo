use super::{view, Services};
use crate::features::capture::completion;
use crate::integrations::ui_thread;
use async_trait::async_trait;
use snaplingo_core::capture::model::AnnotationCommand;
use snaplingo_core::capture::model::CaptureSessionId;
use snaplingo_core::capture::model::LogicalRect;
use snaplingo_core::capture::CaptureEditorStyleChange;
use snaplingo_core::capture::CaptureSelectionMode;
use snaplingo_ui::capture::CaptureCommand;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Mutex,
};

struct Pending {
    services: Services,
    id: CaptureSessionId,
    mode: CaptureSelectionMode,
    action: Action,
}
static PREVIEWS: std::sync::LazyLock<Mutex<HashMap<(String, bool), Option<Pending>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

pub(super) enum Action {
    Cancel,
    Refresh,
    FocusLost,
    CopyText(String),
    MoveCursor(i32, i32),
    Output {
        command: CaptureCommand,
        rect: LogicalRect,
        annotations: Vec<AnnotationCommand>,
        include_cursor: bool,
    },
    Inspect {
        point: snaplingo_core::capture::model::LogicalPoint,
        revision: u64,
    },
    Render {
        rect: LogicalRect,
        annotations: Vec<AnnotationCommand>,
        include_cursor: bool,
        revision: u64,
    },
    Style(CaptureEditorStyleChange),
    Presets(Vec<[u8; 4]>),
    RememberTool(String),
    SystemColor([u8; 4]),
}
pub(super) fn dispatch(
    services: Services,
    id: CaptureSessionId,
    mode: CaptureSelectionMode,
    action: Action,
) {
    let lane = match &action {
        Action::Inspect { .. } => Some(false),
        Action::Render { .. } => Some(true),
        _ => None,
    };
    let pending = Pending {
        services,
        id,
        mode,
        action,
    };
    if let Some(lane) = lane {
        let key = (pending.id.0.clone(), lane);
        match PREVIEWS.lock().unwrap().entry(key.clone()) {
            Entry::Occupied(mut entry) => {
                *entry.get_mut() = Some(pending);
                return;
            }
            Entry::Vacant(entry) => {
                entry.insert(None);
            }
        }
        // One worker per session and lane; pointer/annotation bursts retain only
        // the newest pending revision while the current operation completes.
        crate::integrations::runtime::spawn(async move {
            let mut pending = pending;
            loop {
                perform(pending).await;
                let next = {
                    let mut previews = PREVIEWS.lock().unwrap();
                    let next = previews.get_mut(&key).and_then(Option::take);
                    if next.is_none() {
                        previews.remove(&key);
                    }
                    next
                };
                match next {
                    Some(next) => pending = next,
                    None => break,
                }
            }
        });
    } else {
        crate::integrations::runtime::spawn(perform(pending));
    }
}
async fn perform(pending: Pending) {
    let Pending {
        services,
        id,
        mode,
        action,
    } = pending;
    if let Err(error) = run(&services, &id, mode, action).await {
        let id = id.0;
        let message = error.to_string();
        let _ = ui_thread::run("capture action failed", move || {
            view::action_finished(&id, Some(message));
            Ok(())
        })
        .await;
    }
}
async fn current(id: &CaptureSessionId) -> crate::Result<bool> {
    let session = id.0.clone();
    ui_thread::run("validate capture action", move || {
        Ok(view::is_current(&session))
    })
    .await
}

async fn run(
    services: &Services,
    id: &CaptureSessionId,
    mode: CaptureSelectionMode,
    action: Action,
) -> crate::Result<()> {
    if !current(id).await? {
        return Ok(());
    }
    let runtime = services.runtime()?;
    match action {
        Action::Cancel => {
            let session = id.0.clone();
            ui_thread::run("cancel capture overlay", move || {
                view::close_session(&session);
                Ok(())
            })
            .await?;
            runtime.cancel_capture_session(id).await
        }
        Action::Refresh => {
            let session = id.0.clone();
            let current = ui_thread::run("refresh capture overlay", move || {
                let current = view::is_current(&session);
                if current {
                    view::close_session(&session);
                }
                Ok(current)
            })
            .await?;
            if current {
                runtime.cancel_capture_session(id).await?;
                runtime.open_capture_window_for_mode(mode.as_str()).await?;
            }
            Ok(())
        }
        Action::FocusLost => {
            let session = id.0.clone();
            let close = ui_thread::run("check capture focus", move || {
                Ok(view::is_current(&session)
                    && !view::owns_focus(&session)
                    && !view::is_busy(&session))
            })
            .await?;
            if close {
                let session = id.0.clone();
                ui_thread::run("close unfocused capture", move || {
                    view::close_session(&session);
                    Ok(())
                })
                .await?;
                runtime.cancel_capture_session(id).await?;
            }
            Ok(())
        }
        Action::CopyText(text) => {
            snaplingo_platform::clipboard_text::copy_text(text).map_err(Into::into)
        }
        Action::MoveCursor(x, y) => {
            services.cursor.move_relative(x, y);
            Ok(())
        }
        Action::Inspect { point, revision } => {
            let result = runtime
                .control_candidate_at(id, &point)
                .await
                .map(|candidate| candidate.map(|c| c.rect))
                .map_err(|e| e.to_string());
            let session = id.0.clone();
            ui_thread::run("update capture element", move || {
                view::control_result(&session, revision, result);
                Ok(())
            })
            .await
        }
        Action::Render {
            rect,
            annotations,
            include_cursor,
            revision,
        } => {
            let pixels = runtime.render_rgba(id, &rect, &annotations, include_cursor);
            let session = id.0.clone();
            ui_thread::run("render capture annotations", move || match pixels {
                Ok(pixels) => view::rendered(&session, revision, rect, pixels),
                Err(error) => {
                    view::render_failed(&session, revision, error.to_string());
                    Ok(())
                }
            })
            .await
        }
        Action::RememberTool(tool) => runtime.remember_annotation_tool(tool),
        Action::Style(style) => {
            let mut settings = services.settings.snapshot()?.screenshot;
            match style {
                CaptureEditorStyleChange::StrokeWidth(value) => {
                    settings.default_stroke_width = value as u8
                }
                CaptureEditorStyleChange::FontSize(value) => {
                    settings.default_font_size = value as u8
                }
                _ => return Ok(()),
            };
            services.settings.update_screenshot(settings)?;
            Ok(())
        }
        Action::Presets(colors) => {
            let saved = services.settings.update_annotation_colors(colors)?;
            let session = id.0.clone();
            ui_thread::run("confirm annotation colors", move || {
                view::presets(&session, saved.screenshot.annotation_colors);
                Ok(())
            })
            .await
        }
        Action::SystemColor(color) => {
            let session = id.0.clone();
            let selected =
                ui_thread::run("select annotation color", move || view::choose_color(color))
                    .await?;
            if let Some(color) = selected {
                ui_thread::run("apply annotation color", move || {
                    view::apply_color(&session, color);
                    Ok(())
                })
                .await?;
            } else {
                ui_thread::run("cancel annotation color", move || {
                    view::action_finished(&session, None);
                    Ok(())
                })
                .await?;
            }
            Ok(())
        }
        Action::Output {
            command,
            rect,
            annotations,
            include_cursor,
        } => {
            completion::complete(
                services,
                completion::Request {
                    id,
                    mode,
                    command,
                    rect: &rect,
                    annotations: &annotations,
                    include_cursor,
                },
                &NativeCompletionEffects,
            )
            .await
        }
    }
}

struct NativeCompletionEffects;

#[async_trait]
impl completion::Effects for NativeCompletionEffects {
    async fn is_current(&self, id: &CaptureSessionId) -> crate::Result<bool> {
        current(id).await
    }
    async fn save_path(
        &self,
        id: &CaptureSessionId,
        default: std::path::PathBuf,
        _format: &str,
    ) -> crate::Result<Option<std::path::PathBuf>> {
        let session = id.0.clone();
        let future = ui_thread::run("capture save file dialog", move || {
            view::suspend(&session);
            Ok(rfd::AsyncFileDialog::new()
                .set_title("保存截图")
                .set_directory(default.parent().unwrap_or(std::path::Path::new(".")))
                .set_file_name(default.file_name().unwrap_or_default().to_string_lossy())
                .add_filter("PNG", &["png"])
                .add_filter("JPEG", &["jpg", "jpeg"])
                .add_filter("WebP", &["webp"])
                .save_file())
        })
        .await?;
        Ok(future.await.map(|file| file.path().to_path_buf()))
    }
    async fn print(&self, id: &CaptureSessionId, png: Vec<u8>) -> crate::Result<bool> {
        print_png(id, &png).await
    }
    async fn resume(&self, id: &CaptureSessionId) -> crate::Result<()> {
        let session = id.0.clone();
        ui_thread::run("resume capture completion", move || {
            view::action_finished(&session, None);
            Ok(())
        })
        .await
    }
    async fn close(&self, id: &CaptureSessionId) -> crate::Result<()> {
        let session = id.0.clone();
        ui_thread::run("close completed capture", move || {
            view::close_session(&session);
            Ok(())
        })
        .await
    }
}

async fn print_png(id: &CaptureSessionId, png: &[u8]) -> crate::Result<bool> {
    // The system print path finishes before returning, so its unique spool
    // file is removed immediately afterward, including cancellation or errors.
    use std::io::Write;
    let mut file = tempfile::Builder::new()
        .prefix("SnapLingo-print-")
        .suffix(".png")
        .tempfile()?;
    file.write_all(png)?;
    file.flush()?;
    let path = file.path().to_path_buf();
    let session = id.0.clone();
    ui_thread::run("print screenshot", move || {
        if !view::is_current(&session) {
            return Ok(false);
        }
        view::suspend(&session);
        view::print_file(&path)
    })
    .await
}
