use std::path::Path;
use std::sync::Arc;

use crate::application::services::{
    CaptureOutputService, ImageCompositionService, PinnedImageOpenRequest, PinnedImageService,
};
use crate::domain::capture::PinnedImageView;
use crate::Result;

#[async_trait::async_trait]
pub(crate) trait PinnedImageRuntimeHost: Send + Sync {
    async fn open(&self, image: PinnedImageView) -> Result<()>;
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

/// Coordinates Pinned Image state, image output, and window effects.
pub struct PinnedImageRuntime {
    service: Arc<PinnedImageService>,
    image_composition: Arc<ImageCompositionService>,
    output: Arc<CaptureOutputService>,
    host: Arc<dyn PinnedImageRuntimeHost>,
}

impl PinnedImageRuntime {
    pub(crate) fn new(
        service: Arc<PinnedImageService>,
        image_composition: Arc<ImageCompositionService>,
        output: Arc<CaptureOutputService>,
        host: Arc<dyn PinnedImageRuntimeHost>,
    ) -> Self {
        Self {
            service,
            image_composition,
            output,
            host,
        }
    }

    pub async fn pin_clipboard(&self) -> Result<()> {
        let request = self
            .service
            .pin_clipboard_capture_output(&self.image_composition, &self.output)?;

        match request {
            PinnedImageOpenRequest::Reopen(image) => self.host.show_or_open(image).await,
            PinnedImageOpenRequest::Open(image) => self.host.open(image).await,
        }
    }

    pub async fn pin_png_and_open(&self, png_data: Vec<u8>) -> Result<()> {
        let image = self.service.pin_png_view(png_data)?;
        self.host.open(image).await
    }

    pub async fn close(&self, image_id: &str) -> Result<()> {
        self.service.close_pinned_image(image_id)?;
        self.host.hide(image_id.to_string()).await
    }

    pub fn get(&self, image_id: &str) -> Result<PinnedImageView> {
        self.service.get_pinned_image(image_id)
    }

    pub fn remove(&self, image_id: &str) -> Result<()> {
        self.service.remove_pinned_image(image_id)
    }

    pub async fn copy(&self, image_id: &str) -> Result<()> {
        self.service
            .copy_pinned_png_to_clipboard(&self.output, image_id)
            .await
    }

    pub fn replace_from_clipboard(&self, image_id: &str) -> Result<PinnedImageView> {
        self.service.replace_clipboard_capture_output_view(
            &self.image_composition,
            &self.output,
            image_id,
        )
    }

    pub async fn save(&self, image_id: &str, path: &Path) -> Result<()> {
        self.service
            .save_pinned_png_to_path(&self.output, image_id, path)
            .await
    }

    pub async fn toggle_visibility(&self) -> Result<Option<bool>> {
        self.host.toggle_all().await
    }

    pub async fn switch_group(&self) -> Result<Option<u32>> {
        let Some(group_switch) = self.service.switch_to_next_group() else {
            return Ok(None);
        };

        self.host
            .apply_group_switch(group_switch.hide_image_ids, group_switch.show_image_ids)
            .await?;
        Ok(Some(group_switch.next_group))
    }

    pub async fn move_to_next_group(&self, image_id: &str) -> Result<u32> {
        let next_group = self.service.move_pinned_image_to_next_group(image_id)?;
        self.host.hide(image_id.to_string()).await?;
        Ok(next_group)
    }

    pub async fn hide_group(&self, image_id: &str) -> Result<Vec<String>> {
        let membership = self.service.pinned_image_group_containing(image_id)?;
        self.host.hide_group(membership.image_ids.clone()).await?;
        Ok(membership.image_ids)
    }

    pub async fn destroy_group(&self, image_id: &str) -> Result<Vec<String>> {
        let removal = self
            .service
            .remove_pinned_image_group_containing(image_id)?;
        self.host
            .close_group(removal.removed_image_ids.clone())
            .await?;
        Ok(removal.removed_image_ids)
    }
}
