use std::path::Path;
use std::sync::Arc;

use crate::application::capture::{CaptureImageComposer, CaptureOutput, CapturePinOutput};

use super::{PinnedImageOpenRequest, PinnedImageState};
use crate::application::settings::SettingsConfiguration;
use crate::domain::capture::DesktopPosition;
use crate::domain::capture::PinnedImageView;
use crate::Result;

#[async_trait::async_trait]
pub(crate) trait PinnedImageRuntimeHost: Send + Sync {
    async fn open(&self, image: PinnedImageView, placement: PinnedImagePlacement) -> Result<()>;
    async fn show_or_open(&self, image: PinnedImageView) -> Result<()>;
    async fn hide(&self, image_id: String) -> Result<()>;
    async fn toggle_all(&self) -> Result<Option<bool>>;
    async fn apply_group_switch(
        &self,
        hide_image_ids: Vec<String>,
        show_image_ids: Vec<String>,
    ) -> Result<()>;
    async fn hide_group(&self, image_ids: Vec<String>) -> Result<()>;
    async fn close_group(&self, image_ids: Vec<String>) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PinnedImagePlacement {
    pub origin: Option<DesktopPosition>,
    pub centered: bool,
}

impl Default for PinnedImagePlacement {
    fn default() -> Self {
        Self {
            origin: None,
            centered: true,
        }
    }
}

/// Coordinates Pinned Image state, image output, and window effects.
pub struct PinnedImageRuntime {
    state: Arc<PinnedImageState>,
    image_composition: Arc<CaptureImageComposer>,
    output: Arc<CaptureOutput>,
    host: Arc<dyn PinnedImageRuntimeHost>,
    settings: Option<Arc<SettingsConfiguration>>,
}

#[async_trait::async_trait]
impl CapturePinOutput for PinnedImageRuntime {
    async fn pin_clipboard_image(&self) -> Result<()> {
        self.pin_clipboard().await
    }
    async fn pin_png(
        &self,
        png_data: Vec<u8>,
        source_position: Option<DesktopPosition>,
    ) -> Result<()> {
        let centered = source_position.is_none()
            || self
                .settings
                .as_ref()
                .map(|settings| settings.snapshot())
                .transpose()?
                .is_some_and(|settings| settings.screenshot.pin_position == "center");
        let image = self.state.pin_png_view(png_data)?;
        self.host
            .open(
                image,
                PinnedImagePlacement {
                    origin: source_position,
                    centered,
                },
            )
            .await
    }
}

impl PinnedImageRuntime {
    pub(crate) fn new(
        state: Arc<PinnedImageState>,
        image_composition: Arc<CaptureImageComposer>,
        output: Arc<CaptureOutput>,
        host: Arc<dyn PinnedImageRuntimeHost>,
    ) -> Self {
        Self {
            state,
            image_composition,
            output,
            host,
            settings: None,
        }
    }

    pub(crate) fn with_settings(mut self, settings: Arc<SettingsConfiguration>) -> Self {
        self.settings = Some(settings);
        self
    }

    pub async fn pin_clipboard(&self) -> Result<()> {
        let request = self
            .state
            .pin_clipboard_capture_output(&self.image_composition, &self.output)?;

        match request {
            PinnedImageOpenRequest::Reopen(image) => self.host.show_or_open(image).await,
            PinnedImageOpenRequest::Open(image) => {
                self.host.open(image, PinnedImagePlacement::default()).await
            }
        }
    }

    pub async fn pin_png_and_open(&self, png_data: Vec<u8>) -> Result<()> {
        let image = self.state.pin_png_view(png_data)?;
        self.host.open(image, PinnedImagePlacement::default()).await
    }

    pub async fn close(&self, image_id: &str) -> Result<()> {
        self.state.close_pinned_image(image_id)?;
        self.host.hide(image_id.to_string()).await
    }

    pub fn get(&self, image_id: &str) -> Result<PinnedImageView> {
        self.state.get_pinned_image(image_id)
    }

    pub fn remove(&self, image_id: &str) -> Result<()> {
        self.state.remove_pinned_image(image_id)
    }

    pub async fn copy(&self, image_id: &str) -> Result<()> {
        self.state
            .copy_pinned_png_to_clipboard(&self.output, image_id)
            .await
    }

    pub async fn copy_at_size(
        &self,
        image_id: &str,
        size: Option<crate::domain::capture::PinnedImageExportSize>,
    ) -> Result<()> {
        let png = self.state.get_pinned_png_at_size(image_id, size)?;
        self.output.copy_png(&png).await
    }

    pub fn replace_from_clipboard(&self, image_id: &str) -> Result<PinnedImageView> {
        self.state.replace_clipboard_capture_output_view(
            &self.image_composition,
            &self.output,
            image_id,
        )
    }

    pub async fn save(&self, image_id: &str, path: &Path) -> Result<()> {
        self.state
            .save_pinned_png_to_path(&self.output, image_id, path)
            .await
    }

    pub async fn save_at_size(
        &self,
        image_id: &str,
        path: &Path,
        size: Option<crate::domain::capture::PinnedImageExportSize>,
        remember_choice: bool,
    ) -> Result<()> {
        let png = self.state.get_pinned_png_at_size(image_id, size)?;
        let quality = self
            .settings
            .as_ref()
            .map(|settings| settings.snapshot())
            .transpose()?
            .map_or(90, |settings| settings.screenshot.quality);
        let saved = self.output.save_image(&png, path, "png", quality).await?;
        if remember_choice {
            if let Some(settings) = &self.settings {
                let format = saved
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or("png")
                    .to_ascii_lowercase();
                let format = if format == "jpeg" { "jpg" } else { &format };
                if let Err(error) = settings.record_screenshot_save_choice(&saved, format) {
                    log::warn!("Image saved, but could not remember the save choice: {error}");
                }
            }
        }
        Ok(())
    }

    pub async fn toggle_visibility(&self) -> Result<Option<bool>> {
        self.host.toggle_all().await
    }

    pub async fn switch_group(&self) -> Result<Option<u32>> {
        let Some(group_switch) = self.state.switch_to_next_group() else {
            return Ok(None);
        };

        self.host
            .apply_group_switch(group_switch.hide_image_ids, group_switch.show_image_ids)
            .await?;
        Ok(Some(group_switch.next_group))
    }

    pub async fn move_to_next_group(&self, image_id: &str) -> Result<u32> {
        let next_group = self.state.move_pinned_image_to_next_group(image_id)?;
        self.host.hide(image_id.to_string()).await?;
        Ok(next_group)
    }

    pub async fn hide_group(&self, image_id: &str) -> Result<Vec<String>> {
        let membership = self.state.pinned_image_group_containing(image_id)?;
        self.host.hide_group(membership.image_ids.clone()).await?;
        Ok(membership.image_ids)
    }

    pub async fn destroy_group(&self, image_id: &str) -> Result<Vec<String>> {
        let removal = self.state.remove_pinned_image_group_containing(image_id)?;
        self.host
            .close_group(removal.removed_image_ids.clone())
            .await?;
        Ok(removal.removed_image_ids)
    }

    pub async fn close_all(&self) -> Result<Vec<String>> {
        let ids = self.state.remove_all_pinned_images();
        self.host.close_group(ids.clone()).await?;
        Ok(ids)
    }
}
