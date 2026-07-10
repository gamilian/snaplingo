use tauri::AppHandle;

use crate::application::pinned_image::PinnedImageRuntimeHost;
use crate::domain::capture::PinnedImageView;
use crate::Result;

use super::{
    apply_pinned_group_window_switch, close_pinned_group_windows, close_pinned_image_window,
    hide_pinned_group_windows, open_pinned_image_window, show_or_open_pinned_image_window,
    toggle_pinned_image_windows_visibility,
};

pub(crate) struct TauriPinnedImageRuntimeHost {
    app: AppHandle,
}

impl TauriPinnedImageRuntimeHost {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl PinnedImageRuntimeHost for TauriPinnedImageRuntimeHost {
    async fn open(&self, image: PinnedImageView) -> Result<()> {
        run_on_main_thread(&self.app, "open pinned image window", move |app| {
            open_pinned_image_window(&app, &image)
        })
        .await
    }

    async fn show_or_open(&self, image: PinnedImageView) -> Result<()> {
        run_on_main_thread(&self.app, "show pinned image window", move |app| {
            show_or_open_pinned_image_window(&app, &image)
        })
        .await
    }

    async fn hide(&self, image_id: String) -> Result<()> {
        run_on_main_thread(&self.app, "hide pinned image window", move |app| {
            close_pinned_image_window(&app, &image_id)
        })
        .await
    }

    async fn toggle_all(&self) -> Result<Option<bool>> {
        run_on_main_thread(&self.app, "toggle pinned image windows", move |app| {
            toggle_pinned_image_windows_visibility(&app)
        })
        .await
    }

    async fn apply_group_switch(
        &self,
        hide_image_ids: Vec<String>,
        show_image_ids: Vec<String>,
    ) -> Result<()> {
        run_on_main_thread(&self.app, "switch pinned image group", move |app| {
            apply_pinned_group_window_switch(&app, &hide_image_ids, &show_image_ids)
        })
        .await
    }

    async fn hide_group(&self, image_ids: Vec<String>) -> Result<()> {
        run_on_main_thread(&self.app, "hide pinned image group", move |app| {
            hide_pinned_group_windows(&app, &image_ids)
        })
        .await
    }

    async fn close_group(&self, image_ids: Vec<String>) -> Result<()> {
        run_on_main_thread(&self.app, "close pinned image group", move |app| {
            close_pinned_group_windows(&app, &image_ids)
        })
        .await
    }
}

async fn run_on_main_thread<T, F>(
    app: &AppHandle,
    operation_name: &'static str,
    operation: F,
) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(AppHandle) -> std::result::Result<T, String> + Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.run_on_main_thread({
        let app = app.clone();
        move || {
            let _ = sender.send(operation(app));
        }
    })
    .map_err(|error| format!("Failed to dispatch {operation_name}: {error}"))?;

    let result = receiver
        .await
        .map_err(|error| format!("Failed to receive {operation_name} result: {error}"))?;
    result.map_err(Into::into)
}
