use std::borrow::Cow;
use std::path::Path;

use arboard::{Clipboard, ImageData};
use chrono::{DateTime, Local};
use image::ImageEncoder;

use crate::application::capture::{CaptureOutputHost, CaptureOutputSystemPaths};
use crate::{AppError, Result};

pub(crate) struct SystemCaptureOutputHost;

impl CaptureOutputHost for SystemCaptureOutputHost {
    fn system_paths(&self) -> CaptureOutputSystemPaths {
        CaptureOutputSystemPaths {
            download_dir: dirs::download_dir(),
            picture_dir: dirs::picture_dir(),
            home_dir: dirs::home_dir(),
            temp_dir: std::env::temp_dir(),
        }
    }

    fn now(&self) -> DateTime<Local> {
        Local::now()
    }

    fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, data)?;
        Ok(())
    }

    fn copy_png(&self, data: &[u8]) -> Result<()> {
        let image = png_to_clipboard_image(data)?;
        let mut clipboard = Clipboard::new()
            .map_err(|error| AppError::System(format!("Failed to open clipboard: {error}")))?;
        clipboard.set_image(image).map_err(|error| {
            AppError::System(format!("Failed to copy image to clipboard: {error}"))
        })
    }

    fn read_clipboard_png(&self) -> Result<Vec<u8>> {
        let mut clipboard = Clipboard::new()
            .map_err(|error| AppError::System(format!("Failed to open clipboard: {error}")))?;
        let image = clipboard.get_image().map_err(|error| {
            AppError::System(format!("Failed to read image from clipboard: {error}"))
        })?;

        clipboard_image_to_png(image)
    }

    fn read_clipboard_text(&self) -> Result<String> {
        let mut clipboard = Clipboard::new()
            .map_err(|error| AppError::System(format!("Failed to open clipboard: {error}")))?;

        clipboard.get_text().map_err(|error| {
            AppError::System(format!("Failed to read text from clipboard: {error}"))
        })
    }
}

fn png_to_clipboard_image(data: &[u8]) -> Result<ImageData<'static>> {
    let image = image::load_from_memory(data)
        .map_err(|error| AppError::System(format!("Failed to decode PNG for clipboard: {error}")))?
        .to_rgba8();
    let (width, height) = image.dimensions();

    Ok(ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(image.into_raw()),
    })
}

fn clipboard_image_to_png(image: ImageData<'_>) -> Result<Vec<u8>> {
    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png);
    encoder
        .write_image(
            &image.bytes,
            image.width as u32,
            image.height as u32,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|error| AppError::System(format!("Failed to encode clipboard image: {error}")))?;

    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_png(width: u32, height: u32) -> Vec<u8> {
        let pixels = vec![255; (width * height * 4) as usize];
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&pixels, width, height, image::ExtendedColorType::Rgba8)
            .unwrap();
        png
    }

    #[test]
    fn converts_png_to_clipboard_image_data() {
        let image = png_to_clipboard_image(&make_test_png(3, 2)).unwrap();

        assert_eq!(image.width, 3);
        assert_eq!(image.height, 2);
        assert_eq!(image.bytes.len(), 3 * 2 * 4);
    }

    #[test]
    fn converts_clipboard_image_data_to_png() {
        let image = ImageData {
            width: 2,
            height: 1,
            bytes: Cow::Owned(vec![255, 0, 0, 255, 0, 255, 0, 255]),
        };

        let png = clipboard_image_to_png(image).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();

        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 1);
        assert_eq!(decoded.into_raw(), vec![255, 0, 0, 255, 0, 255, 0, 255]);
    }
}
