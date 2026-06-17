use std::collections::{BTreeSet, HashMap};
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
    group: u32,
    source_text: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct PinnedImageGroupSwitch {
    pub previous_group: u32,
    pub next_group: u32,
    pub hide_image_ids: Vec<String>,
    pub show_image_ids: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct PinnedImageGroupRemoval {
    pub removed_group: u32,
    pub removed_image_ids: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct PinnedImageGroupMembership {
    pub group: u32,
    pub image_ids: Vec<String>,
}

pub struct PinnedImageService {
    images: Mutex<HashMap<String, PinnedImage>>,
    active_group: Mutex<u32>,
}

impl PinnedImageService {
    pub fn new() -> Self {
        Self {
            images: Mutex::new(HashMap::new()),
            active_group: Mutex::new(0),
        }
    }

    pub fn pin_png(&self, png_data: Vec<u8>) -> Result<String> {
        self.pin_png_with_source_text(png_data, None)
    }

    pub fn pin_png_with_source_text(
        &self,
        png_data: Vec<u8>,
        source_text: Option<String>,
    ) -> Result<String> {
        let image = image::load_from_memory(&png_data)
            .map_err(|e| AppError::System(format!("Failed to decode pinned PNG: {}", e)))?;
        let id = next_pinned_image_id();
        let group = *self.active_group.lock().unwrap();
        let pinned_image = PinnedImage {
            png_data,
            width: image.width(),
            height: image.height(),
            group,
            source_text,
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
            source_text: image.source_text,
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

    pub fn replace_pinned_png(&self, image_id: &str, png_data: Vec<u8>) -> Result<()> {
        let decoded = image::load_from_memory(&png_data)
            .map_err(|e| AppError::System(format!("Failed to decode pinned PNG: {}", e)))?;
        let mut images = self.images.lock().unwrap();
        let image = images
            .get_mut(image_id)
            .ok_or_else(|| AppError::System(format!("Pinned image not found: {}", image_id)))?;

        image.png_data = png_data;
        image.width = decoded.width();
        image.height = decoded.height();
        image.source_text = None;

        Ok(())
    }

    pub fn remove_pinned_image(&self, image_id: &str) -> Result<()> {
        self.images
            .lock()
            .unwrap()
            .remove(image_id)
            .map(|_| ())
            .ok_or_else(|| AppError::System(format!("Pinned image not found: {}", image_id)))
    }

    pub fn move_pinned_image_to_group(&self, image_id: &str, group: u32) -> Result<()> {
        let mut images = self.images.lock().unwrap();
        let image = images
            .get_mut(image_id)
            .ok_or_else(|| AppError::System(format!("Pinned image not found: {}", image_id)))?;

        image.group = group;

        Ok(())
    }

    pub fn move_pinned_image_to_next_group(&self, image_id: &str) -> Result<u32> {
        let mut images = self.images.lock().unwrap();
        let current_group = images
            .get(image_id)
            .ok_or_else(|| AppError::System(format!("Pinned image not found: {}", image_id)))?
            .group;
        let groups = images
            .values()
            .map(|image| image.group)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let next_group = if groups.len() == 1 {
            current_group
                .checked_add(1)
                .ok_or_else(|| AppError::System("No next pinned image group".to_string()))?
        } else {
            groups
                .iter()
                .copied()
                .find(|group| *group > current_group)
                .unwrap_or(groups[0])
        };

        let image = images
            .get_mut(image_id)
            .ok_or_else(|| AppError::System(format!("Pinned image not found: {}", image_id)))?;
        image.group = next_group;

        Ok(next_group)
    }

    pub fn remove_pinned_image_group_containing(
        &self,
        image_id: &str,
    ) -> Result<PinnedImageGroupRemoval> {
        let mut images = self.images.lock().unwrap();
        let removed_group = images
            .get(image_id)
            .ok_or_else(|| AppError::System(format!("Pinned image not found: {}", image_id)))?
            .group;
        let mut removed_image_ids = image_ids_in_group(&images, removed_group);

        for removed_image_id in &removed_image_ids {
            images.remove(removed_image_id);
        }

        let next_active_group = images
            .values()
            .map(|image| image.group)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .next()
            .unwrap_or(0);

        drop(images);
        if *self.active_group.lock().unwrap() == removed_group {
            *self.active_group.lock().unwrap() = next_active_group;
        }

        removed_image_ids.sort();

        Ok(PinnedImageGroupRemoval {
            removed_group,
            removed_image_ids,
        })
    }

    pub fn pinned_image_group_containing(
        &self,
        image_id: &str,
    ) -> Result<PinnedImageGroupMembership> {
        let images = self.images.lock().unwrap();
        let group = images
            .get(image_id)
            .ok_or_else(|| AppError::System(format!("Pinned image not found: {}", image_id)))?
            .group;
        let mut image_ids = image_ids_in_group(&images, group);

        image_ids.sort();

        Ok(PinnedImageGroupMembership { group, image_ids })
    }

    pub fn switch_to_next_group(&self) -> Option<PinnedImageGroupSwitch> {
        let previous_group = *self.active_group.lock().unwrap();
        let images = self.images.lock().unwrap();
        let groups = images
            .values()
            .map(|image| image.group)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        if groups.is_empty() || (groups.len() == 1 && groups[0] == previous_group) {
            return None;
        }

        let next_group = groups
            .iter()
            .copied()
            .find(|group| *group > previous_group)
            .unwrap_or(groups[0]);
        let mut hide_image_ids = image_ids_in_group(&images, previous_group);
        let mut show_image_ids = image_ids_in_group(&images, next_group);

        drop(images);
        *self.active_group.lock().unwrap() = next_group;

        hide_image_ids.sort();
        show_image_ids.sort();

        Some(PinnedImageGroupSwitch {
            previous_group,
            next_group,
            hide_image_ids,
            show_image_ids,
        })
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

fn image_ids_in_group(images: &HashMap<String, PinnedImage>, group: u32) -> Vec<String> {
    images
        .iter()
        .filter(|(_, image)| image.group == group)
        .map(|(image_id, _)| image_id.clone())
        .collect()
}
