use std::sync::{Arc, RwLock, Weak};

use crate::application::pinned_image::{PinnedImagePlacement, PinnedImageRuntimeHost};
use crate::domain::capture::PinnedImageView;
use crate::infrastructure::ui_thread;
use crate::{CaptureOutput, PinnedImageRuntime, SettingsConfiguration};

#[derive(Clone)]
pub(super) struct Services {
    pub(super) runtime: Weak<PinnedImageRuntime>,
    pub(super) settings: Arc<SettingsConfiguration>,
    pub(super) output: Arc<CaptureOutput>,
    pub(super) show_settings: Arc<dyn Fn() -> Result<(), String> + Send + Sync>,
}

#[derive(Default)]
pub(crate) struct NativePinnedImageRuntimeHost {
    services: RwLock<Option<Services>>,
}

impl NativePinnedImageRuntimeHost {
    pub(crate) fn bind(
        &self,
        state: &crate::AppState,
        show_settings: impl Fn() -> Result<(), String> + Send + Sync + 'static,
    ) {
        *self.services.write().unwrap() = Some(Services {
            runtime: Arc::downgrade(&state.capture.pinned_images),
            settings: state.settings.configuration.clone(),
            output: state.capture.output.clone(),
            show_settings: Arc::new(show_settings),
        });
    }

    fn services(&self) -> Result<Services, String> {
        self.services
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| "Native pinned image services are not initialized".into())
    }

    pub(crate) fn settings_changed(&self) {
        let Ok(services) = self.services() else {
            return;
        };
        ui_thread::dispatch(
            move || match super::macos::preferences(&services.settings) {
                Ok(preferences) => super::native_view::update_preferences(preferences),
                Err(error) => log::warn!("Failed to refresh native pinned preferences: {error}"),
            },
        );
    }
}

#[async_trait::async_trait]
impl PinnedImageRuntimeHost for NativePinnedImageRuntimeHost {
    async fn open(
        &self,
        image: PinnedImageView,
        placement: PinnedImagePlacement,
    ) -> crate::Result<()> {
        let services = self.services()?;
        ui_thread::run("open pinned image window", move || {
            super::macos::open_pinned_image_window(&services, &image, placement)
        })
        .await
    }

    async fn show_or_open(&self, image: PinnedImageView) -> crate::Result<()> {
        let services = self.services()?;
        ui_thread::run("show pinned image window", move || {
            super::macos::show_or_open_pinned_image_window(&services, &image)
        })
        .await
    }

    async fn hide(&self, image_id: String) -> crate::Result<()> {
        ui_thread::run("hide pinned image window", move || {
            super::macos::close_pinned_image_window(&image_id)
        })
        .await
    }

    async fn toggle_all(&self) -> crate::Result<Option<bool>> {
        ui_thread::run(
            "toggle pinned image windows",
            super::macos::toggle_pinned_image_windows_visibility,
        )
        .await
    }

    async fn apply_group_switch(&self, hide: Vec<String>, show: Vec<String>) -> crate::Result<()> {
        ui_thread::run("switch pinned image group", move || {
            super::macos::apply_pinned_group_window_switch(&hide, &show)
        })
        .await
    }

    async fn hide_group(&self, ids: Vec<String>) -> crate::Result<()> {
        ui_thread::run("hide pinned image group", move || {
            super::macos::hide_pinned_group_windows(&ids)
        })
        .await
    }

    async fn close_group(&self, ids: Vec<String>) -> crate::Result<()> {
        ui_thread::run("close pinned image group", move || {
            super::macos::close_pinned_group_windows(&ids)
        })
        .await
    }
}
