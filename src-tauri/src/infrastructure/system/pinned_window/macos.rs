use std::sync::Arc;

use core_graphics::display::CGDisplay;
use tauri::{AppHandle, Listener, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use super::native_view::{self as native, Action, Preferences};
use super::presentation::Presentation;
use super::runtime_host::run_on_main_thread;
use crate::application::pinned_image::PinnedImagePlacement;
use crate::domain::capture::{
    DesktopPosition, LogicalRect, PinnedImageExportSize, PinnedImageView,
};
use crate::infrastructure::system::window_position::{cascaded_center, union};

fn preferences(app: &AppHandle) -> Result<Preferences, String> {
    let screenshot = app
        .state::<crate::AppState>()
        .settings
        .configuration
        .snapshot()
        .map_err(|error| error.to_string())?
        .screenshot;
    Ok(Preferences {
        opacity: screenshot.pin_opacity as f64 / 100.0,
        shadow: screenshot.pin_shadow,
        border: screenshot.pin_border,
        border_shortcut: screenshot.pin_border_shortcut,
    })
}

pub(super) fn listen_settings(app: &AppHandle) -> tauri::EventId {
    let event_app = app.clone();
    app.listen("settings-changed", move |_| {
        if event_app.try_state::<crate::AppState>().is_none() {
            return;
        }
        if let Ok(preferences) = preferences(&event_app) {
            let _ = event_app.run_on_main_thread(move || native::update_preferences(preferences));
        }
    })
}

pub fn close_pinned_image_window(_app: &AppHandle, image_id: &str) -> Result<(), String> {
    native::hide(image_id);
    Ok(())
}

pub fn show_or_open_pinned_image_window(
    app: &AppHandle,
    image: &PinnedImageView,
) -> Result<(), String> {
    if native::focus(&image.id)? {
        return Ok(());
    }
    open_pinned_image_window(app, image, PinnedImagePlacement::default())
}

pub fn open_pinned_image_window(
    app: &AppHandle,
    image: &PinnedImageView,
    placement: PinnedImagePlacement,
) -> Result<(), String> {
    let (width, height) = Presentation::default().window_size(image.width, image.height);
    let point = if placement.centered {
        let screens = CGDisplay::active_displays()
            .map_err(|error| format!("Display enumeration failed: {error}"))?
            .into_iter()
            .map(|id| {
                let bounds = CGDisplay::new(id).bounds();
                LogicalRect {
                    x: bounds.origin.x,
                    y: bounds.origin.y,
                    width: bounds.size.width,
                    height: bounds.size.height,
                }
            })
            .collect::<Vec<_>>();
        cascaded_center(&union(&screens)?, (width, height), native::ids().len())
    } else {
        match placement.origin {
            Some(DesktopPosition::Logical(point)) => point,
            Some(DesktopPosition::Physical(point)) => {
                let display = CGDisplay::main();
                let scale = display.pixels_wide() as f64 / display.bounds().size.width;
                crate::domain::capture::LogicalPoint {
                    x: point.x as f64 / scale,
                    y: point.y as f64 / scale,
                }
            }
            None => crate::domain::capture::LogicalPoint { x: 0.0, y: 0.0 },
        }
    };
    let rect = LogicalRect {
        x: point.x,
        y: point.y,
        width,
        height,
    };
    let event_app = app.clone();
    let id = image.id.clone();
    let gate = Arc::new(tokio::sync::Mutex::new(()));
    native::open(
        image.clone(),
        &rect,
        preferences(app)?,
        move |action, size| {
            dispatch(event_app.clone(), id.clone(), gate.clone(), action, size);
        },
    )
}

pub fn toggle_pinned_image_windows_visibility(_app: &AppHandle) -> Result<Option<bool>, String> {
    let ids = native::ids();
    if ids.is_empty() {
        return Ok(None);
    }
    let visible = ids.iter().any(|id| !native::visible(id));
    for id in ids {
        if visible {
            native::show(&id)?;
        } else {
            native::hide(&id);
        }
    }
    Ok(Some(visible))
}

pub fn apply_pinned_group_window_switch(
    _app: &AppHandle,
    hide: &[String],
    show: &[String],
) -> Result<(), String> {
    for id in hide {
        native::hide(id);
    }
    for id in show {
        native::show(id)?;
    }
    Ok(())
}

pub fn hide_moved_pinned_image_window(_app: &AppHandle, image_id: &str) -> Result<(), String> {
    native::hide(image_id);
    Ok(())
}

pub fn hide_pinned_group_windows(_app: &AppHandle, image_ids: &[String]) -> Result<(), String> {
    for id in image_ids {
        native::hide(id);
    }
    Ok(())
}

pub fn close_pinned_group_windows(_app: &AppHandle, image_ids: &[String]) -> Result<(), String> {
    for id in image_ids {
        native::close(id);
    }
    Ok(())
}

fn dispatch(
    app: AppHandle,
    id: String,
    gate: Arc<tokio::sync::Mutex<()>>,
    action: Action,
    size: PinnedImageExportSize,
) {
    tauri::async_runtime::spawn(async move {
        let _operation = gate.lock().await;
        if let Err(error) = run_action(&app, &id, action, size).await {
            app.dialog()
                .message(error.to_string())
                .kind(MessageDialogKind::Error)
                .title("贴图操作失败")
                .show(|_| {});
        }
    });
}

async fn run_action(
    app: &AppHandle,
    id: &str,
    action: Action,
    size: PinnedImageExportSize,
) -> crate::Result<()> {
    let state = app.state::<crate::AppState>();
    let runtime = &state.capture.pinned_images;
    match action {
        Action::Copy => runtime.copy_at_size(id, Some(size)).await,
        Action::Save => {
            if let Some(path) =
                crate::infrastructure::system::image_save_dialog::select_capture_save_path(app)
                    .await?
            {
                runtime.save_at_size(id, &path, Some(size), true).await?;
            }
            Ok(())
        }
        Action::QuickSave => {
            let screenshot = state.settings.configuration.snapshot()?.screenshot;
            let path = state.capture.output.quick_capture_save_path(
                Some(&screenshot.save_path),
                "png",
                "timestamp",
                "SnapLingo",
            );
            runtime.save_at_size(id, &path, Some(size), false).await
        }
        Action::Close => runtime.close(id).await,
        Action::Destroy => {
            runtime.remove(id)?;
            let id = id.to_owned();
            run_on_main_thread(app, "destroy pinned image", move |_| {
                native::close(&id);
                Ok(())
            })
            .await
        }
        Action::HideGroup => runtime.hide_group(id).await.map(|_| ()),
        Action::DestroyGroup => runtime.destroy_group(id).await.map(|_| ()),
        Action::CloseAll => runtime.close_all().await.map(|_| ()),
        Action::MoveGroup => runtime.move_to_next_group(id).await.map(|_| ()),
        Action::Preferences => {
            run_on_main_thread(app, "open pinned preferences", |app| {
                crate::settings_window::show_settings_window(&app)
            })
            .await
        }
        Action::Replace => {
            let image = runtime.replace_from_clipboard(id)?;
            run_on_main_thread(app, "replace pinned image", move |_| native::replace(image)).await
        }
        Action::ToggleBorder => {
            let mut screenshot = state.settings.configuration.snapshot()?.screenshot;
            screenshot.pin_border = !screenshot.pin_border;
            state.settings.configuration.update_screenshot(screenshot)?;
            Ok(())
        }
        Action::CopyText(text) => {
            crate::infrastructure::system::clipboard_text::copy_text(text).map_err(Into::into)
        }
    }
}
