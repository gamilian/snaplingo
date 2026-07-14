use std::borrow::Cow;
use std::path::{Path, PathBuf};

use arboard::{Clipboard, ImageData};
use image::{DynamicImage, ImageEncoder};

use crate::error::{AppError, Result};

pub struct CaptureOutput;

pub enum ClipboardCaptureOutput {
    Png(Vec<u8>),
    Text(String),
}

impl CaptureOutput {
    pub fn new() -> Self {
        Self
    }

    pub fn default_capture_save_path(
        &self,
        directory: Option<&str>,
        format: &str,
        naming_rule: &str,
        custom_file_name: &str,
    ) -> PathBuf {
        let base_dir = directory
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(configured_capture_save_dir_for_system)
            .or_else(dirs::download_dir)
            .or_else(dirs::picture_dir)
            .or_else(dirs::home_dir)
            .unwrap_or_else(std::env::temp_dir);
        configured_capture_save_path(
            &base_dir,
            format,
            naming_rule,
            custom_file_name,
            chrono::Local::now(),
        )
    }

    pub fn quick_capture_save_path(
        &self,
        directory: Option<&str>,
        format: &str,
        naming_rule: &str,
        custom_file_name: &str,
    ) -> PathBuf {
        let base_dir = directory
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(configured_capture_save_dir_for_system)
            .unwrap_or_else(default_quick_capture_save_dir);
        configured_capture_save_path(
            &base_dir,
            format,
            naming_rule,
            custom_file_name,
            chrono::Local::now(),
        )
    }

    pub async fn save_png(&self, data: &[u8], path: &Path) -> Result<PathBuf> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, data)?;

        Ok(path.to_path_buf())
    }

    pub async fn save_image(
        &self,
        png_data: &[u8],
        path: &Path,
        format: &str,
        quality: u8,
    ) -> Result<PathBuf> {
        let encoded = encode_capture_image(png_data, format, quality)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, encoded)?;
        Ok(path.to_path_buf())
    }

    pub async fn copy_png(&self, data: &[u8]) -> Result<()> {
        let image = Self::png_to_clipboard_image(data)?;
        let mut clipboard = Clipboard::new()
            .map_err(|e| AppError::System(format!("Failed to open clipboard: {}", e)))?;
        clipboard
            .set_image(image)
            .map_err(|e| AppError::System(format!("Failed to copy image to clipboard: {}", e)))
    }

    pub fn read_clipboard_png(&self) -> Result<Vec<u8>> {
        let mut clipboard = Clipboard::new()
            .map_err(|e| AppError::System(format!("Failed to open clipboard: {}", e)))?;
        let image = clipboard
            .get_image()
            .map_err(|e| AppError::System(format!("Failed to read image from clipboard: {}", e)))?;

        Self::clipboard_image_to_png(image)
    }

    pub fn read_clipboard_text(&self) -> Result<String> {
        let mut clipboard = Clipboard::new()
            .map_err(|e| AppError::System(format!("Failed to open clipboard: {}", e)))?;

        clipboard
            .get_text()
            .map_err(|e| AppError::System(format!("Failed to read text from clipboard: {}", e)))
    }

    pub fn read_clipboard_capture_output(&self) -> Result<ClipboardCaptureOutput> {
        match self.read_clipboard_png() {
            Ok(png_data) => Ok(ClipboardCaptureOutput::Png(png_data)),
            Err(image_error) => {
                let text = self.read_clipboard_text().map_err(|text_error| {
                    AppError::System(format!(
                        "{}; also failed to read text from clipboard: {}",
                        image_error, text_error
                    ))
                })?;

                Ok(ClipboardCaptureOutput::Text(text))
            }
        }
    }

    pub fn png_to_clipboard_image(data: &[u8]) -> Result<ImageData<'static>> {
        let image = image::load_from_memory(data)
            .map_err(|e| AppError::System(format!("Failed to decode PNG for clipboard: {}", e)))?
            .to_rgba8();
        let (width, height) = image.dimensions();

        Ok(ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(image.into_raw()),
        })
    }

    pub fn clipboard_image_to_png(image: ImageData<'_>) -> Result<Vec<u8>> {
        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        encoder
            .write_image(
                &image.bytes,
                image.width as u32,
                image.height as u32,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| AppError::System(format!("Failed to encode clipboard image: {}", e)))?;

        Ok(png)
    }
}

pub(crate) fn configured_capture_save_path(
    base_dir: &Path,
    format: &str,
    naming_rule: &str,
    custom_file_name: &str,
    now: chrono::DateTime<chrono::Local>,
) -> PathBuf {
    let extension = capture_extension(format);
    let base_name = match naming_rule {
        "date" => format!("SnapLingo-{}", now.format("%Y-%m-%d")),
        "counter" => "Screenshot".to_string(),
        "custom" => sanitize_file_name(custom_file_name),
        _ => format!("SnapLingo-{}", now.format("%Y%m%d-%H%M%S")),
    };

    if naming_rule == "counter" {
        return first_available_capture_path(base_dir, &base_name, extension, 1, 3);
    }

    let direct = base_dir.join(format!("{base_name}.{extension}"));
    if !direct.exists() {
        return direct;
    }
    first_available_capture_path(base_dir, &base_name, extension, 2, 0)
}

fn first_available_capture_path(
    base_dir: &Path,
    base_name: &str,
    extension: &str,
    start: u32,
    width: usize,
) -> PathBuf {
    for counter in start..u32::MAX {
        let suffix = if width == 0 {
            counter.to_string()
        } else {
            format!("{counter:0width$}")
        };
        let path = base_dir.join(format!("{base_name}_{suffix}.{extension}"));
        if !path.exists() {
            return path;
        }
    }
    base_dir.join(format!("{base_name}.{extension}"))
}

fn capture_extension(format: &str) -> &'static str {
    match format {
        "jpg" | "jpeg" => "jpg",
        "webp" => "webp",
        _ => "png",
    }
}

fn sanitize_file_name(value: &str) -> String {
    let sanitized: String = value
        .trim()
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => character,
        })
        .collect();
    if sanitized.is_empty() {
        "SnapLingo".to_string()
    } else {
        sanitized
    }
}

fn encode_capture_image(png_data: &[u8], format: &str, quality: u8) -> Result<Vec<u8>> {
    if format == "png" {
        return Ok(png_data.to_vec());
    }

    let image = image::load_from_memory(png_data)
        .map_err(|error| AppError::System(format!("Failed to decode capture image: {error}")))?;
    match format {
        "jpg" | "jpeg" => encode_jpeg(&image, quality),
        "webp" => encode_webp(&image, quality),
        _ => Ok(png_data.to_vec()),
    }
}

fn encode_jpeg(image: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let rgb = image.to_rgb8();
    let mut encoded = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, quality.clamp(1, 100))
        .write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|error| AppError::System(format!("Failed to encode JPEG: {error}")))?;
    Ok(encoded)
}

fn encode_webp(image: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let rgba = image.to_rgba8();
    Ok(
        webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height())
            .encode(quality.clamp(1, 100) as f32)
            .to_vec(),
    )
}

fn default_quick_capture_save_dir() -> PathBuf {
    dirs::picture_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("SnapLingo")
}

pub(crate) fn configured_capture_save_dir(configured: &str, home_dir: &Path) -> PathBuf {
    if configured == "~" {
        return home_dir.to_path_buf();
    }

    if let Some(relative) = configured.strip_prefix("~/") {
        return home_dir.join(relative);
    }

    PathBuf::from(configured)
}

fn configured_capture_save_dir_for_system(configured: &str) -> PathBuf {
    if configured == "~" || configured.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return configured_capture_save_dir(configured, &home);
        }
    }

    PathBuf::from(configured)
}

impl Default for CaptureOutput {
    fn default() -> Self {
        Self::new()
    }
}
