use std::sync::Arc;

use super::Services;
use crate::infrastructure::ui_thread;
use core_graphics::display::CGDisplay;

use super::view::{self as native, Action, Preferences};
use snaplingo_ui::pinned::Presentation;

use super::super::position::{cascaded_center, union};
use snaplingo_core::application::pinned_image::PinnedImagePlacement;
use snaplingo_core::domain::capture::{
    DesktopPosition, LogicalRect, PinnedImageExportSize, PinnedImageView,
};

pub(super) fn preferences(
    settings: &snaplingo_core::application::SettingsConfiguration,
) -> Result<Preferences, String> {
    let screenshot = settings
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

pub(super) fn close_pinned_image_window(image_id: &str) -> Result<(), String> {
    native::hide(image_id);
    Ok(())
}

pub(super) fn show_or_open_pinned_image_window(
    services: &Services,
    image: &PinnedImageView,
) -> Result<(), String> {
    if native::focus(&image.id)? {
        return Ok(());
    }
    open_pinned_image_window(services, image, PinnedImagePlacement::default())
}

pub(super) fn open_pinned_image_window(
    services: &Services,
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
                snaplingo_core::domain::capture::LogicalPoint {
                    x: point.x as f64 / scale,
                    y: point.y as f64 / scale,
                }
            }
            None => snaplingo_core::domain::capture::LogicalPoint { x: 0.0, y: 0.0 },
        }
    };
    let rect = LogicalRect {
        x: point.x,
        y: point.y,
        width,
        height,
    };
    let event_services = services.clone();
    let id = image.id.clone();
    let gate = Arc::new(tokio::sync::Mutex::new(()));
    native::open(
        image.clone(),
        &rect,
        preferences(&services.settings)?,
        move |action, size| {
            dispatch(
                event_services.clone(),
                id.clone(),
                gate.clone(),
                action,
                size,
            );
        },
    )
}

pub(super) fn toggle_pinned_image_windows_visibility() -> Result<Option<bool>, String> {
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

pub(super) fn apply_pinned_group_window_switch(
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

pub(super) fn hide_pinned_group_windows(image_ids: &[String]) -> Result<(), String> {
    for id in image_ids {
        native::hide(id);
    }
    Ok(())
}

pub(super) fn close_pinned_group_windows(image_ids: &[String]) -> Result<(), String> {
    for id in image_ids {
        native::close(id);
    }
    Ok(())
}

fn dispatch(
    services: Services,
    id: String,
    gate: Arc<tokio::sync::Mutex<()>>,
    action: Action,
    size: PinnedImageExportSize,
) {
    crate::infrastructure::runtime::spawn(async move {
        let _operation = gate.lock().await;
        if let Err(error) = run_action(&services, &id, action, size).await {
            crate::infrastructure::system::dialogs::show_native_error(
                "贴图操作失败",
                &error.to_string(),
            );
        }
    });
}

async fn run_action(
    services: &Services,
    id: &str,
    action: Action,
    size: PinnedImageExportSize,
) -> crate::Result<()> {
    let runtime = services
        .runtime
        .upgrade()
        .ok_or("Pinned image runtime has stopped")?;
    match action {
        Action::Copy => runtime.copy_at_size(id, Some(size)).await,
        Action::Save => {
            if let Some(path) =
                crate::infrastructure::system::image_save_dialog::select_native_capture_save_path(
                    &services.settings,
                    &services.output,
                )
                .await?
            {
                runtime.save_at_size(id, &path, Some(size), true).await?;
            }
            Ok(())
        }
        Action::QuickSave => {
            let screenshot = services.settings.snapshot()?.screenshot;
            let path = services.output.quick_capture_save_path(
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
            ui_thread::run("destroy pinned image", move || {
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
            let show_settings = services.show_settings.clone();
            ui_thread::run("open pinned preferences", move || show_settings()).await
        }
        Action::Replace => {
            let image = runtime.replace_from_clipboard(id)?;
            ui_thread::run("replace pinned image", move || native::replace(image)).await
        }
        Action::ToggleBorder => {
            let mut screenshot = services.settings.snapshot()?.screenshot;
            screenshot.pin_border = !screenshot.pin_border;
            services.settings.update_screenshot(screenshot)?;
            Ok(())
        }
        Action::CopyText(text) => {
            crate::infrastructure::system::clipboard_text::copy_text(text).map_err(Into::into)
        }
    }
}
