use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;

use crate::domain::capture::PinnedImageView;
use crate::error::{AppError, Result};

#[derive(Clone)]
struct PinnedImage {
    png_data: Vec<u8>,
    width: u32,
    height: u32,
}

pub struct PinnedImageService {
    images: Mutex<HashMap<String, PinnedImage>>,
}

impl PinnedImageService {
    pub fn new() -> Self {
        Self {
            images: Mutex::new(HashMap::new()),
        }
    }

    pub fn pin_png(&self, png_data: Vec<u8>) -> Result<String> {
        let image = image::load_from_memory(&png_data)
            .map_err(|e| AppError::System(format!("Failed to decode pinned PNG: {}", e)))?;
        let id = next_pinned_image_id();
        let pinned_image = PinnedImage {
            png_data,
            width: image.width(),
            height: image.height(),
        };

        self.images.lock().unwrap().insert(id.clone(), pinned_image);

        Ok(id)
    }

    pub fn get_pinned_image(&self, image_id: &str) -> Result<PinnedImageView> {
        let image = self
            .images
            .lock()
            .unwrap()
            .get(image_id)
            .cloned()
            .ok_or_else(|| AppError::System(format!("Pinned image not found: {}", image_id)))?;

        Ok(PinnedImageView {
            id: image_id.to_string(),
            image_base64: base64::engine::general_purpose::STANDARD.encode(image.png_data),
            width: image.width,
            height: image.height,
        })
    }

    pub fn get_pinned_png(&self, image_id: &str) -> Result<Vec<u8>> {
        self.images
            .lock()
            .unwrap()
            .get(image_id)
            .map(|image| image.png_data.clone())
            .ok_or_else(|| AppError::System(format!("Pinned image not found: {}", image_id)))
    }

    pub fn remove_pinned_image(&self, image_id: &str) -> Result<()> {
        self.images
            .lock()
            .unwrap()
            .remove(image_id)
            .map(|_| ())
            .ok_or_else(|| AppError::System(format!("Pinned image not found: {}", image_id)))
    }
}

impl Default for PinnedImageService {
    fn default() -> Self {
        Self::new()
    }
}

fn next_pinned_image_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    format!("pin-{}-{}", std::process::id(), nanos)
}
