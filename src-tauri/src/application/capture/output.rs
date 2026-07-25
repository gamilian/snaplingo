use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Local};
use image::{DynamicImage, ImageEncoder};

use crate::error::{AppError, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaptureOutputSystemPaths {
    pub(crate) download_dir: Option<PathBuf>,
    pub(crate) picture_dir: Option<PathBuf>,
    pub(crate) home_dir: Option<PathBuf>,
    pub(crate) temp_dir: PathBuf,
}

pub(crate) trait CaptureOutputHost: Send + Sync {
    fn system_paths(&self) -> CaptureOutputSystemPaths;
    fn now(&self) -> DateTime<Local>;
    fn path_exists(&self, path: &Path) -> bool;
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()>;
    fn copy_png(&self, data: &[u8]) -> Result<()>;
    fn read_clipboard_png(&self) -> Result<Vec<u8>>;
    fn read_clipboard_text(&self) -> Result<String>;
}

pub struct CaptureOutput {
    host: Arc<dyn CaptureOutputHost>,
}

pub enum ClipboardCaptureOutput {
    Png(Vec<u8>),
    Text(String),
}

impl CaptureOutput {
    pub(crate) fn with_host(host: Arc<dyn CaptureOutputHost>) -> Self {
        Self { host }
    }

    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::with_host(Arc::new(TestCaptureOutputHost))
    }

    pub fn default_capture_save_path(
        &self,
        directory: Option<&str>,
        format: &str,
        naming_rule: &str,
        custom_file_name: &str,
    ) -> PathBuf {
        let paths = self.host.system_paths();
        let base_dir = directory
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(|path| configured_capture_save_dir_for_system(path, paths.home_dir.as_deref()))
            .or(paths.download_dir)
            .or(paths.picture_dir)
            .or(paths.home_dir)
            .unwrap_or(paths.temp_dir);
        configured_capture_save_path(
            &base_dir,
            format,
            naming_rule,
            custom_file_name,
            self.host.now(),
            &|path| self.host.path_exists(path),
        )
    }

    pub fn quick_capture_save_path(
        &self,
        directory: Option<&str>,
        format: &str,
        naming_rule: &str,
        custom_file_name: &str,
    ) -> PathBuf {
        let paths = self.host.system_paths();
        let base_dir = directory
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(|path| configured_capture_save_dir_for_system(path, paths.home_dir.as_deref()))
            .unwrap_or_else(|| default_quick_capture_save_dir(&paths));
        configured_capture_save_path(
            &base_dir,
            format,
            naming_rule,
            custom_file_name,
            self.host.now(),
            &|path| self.host.path_exists(path),
        )
    }

    pub async fn save_png(&self, data: &[u8], path: &Path) -> Result<PathBuf> {
        self.host.write_file(path, data)?;

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
        self.host.write_file(path, &encoded)?;
        Ok(path.to_path_buf())
    }

    pub async fn copy_png(&self, data: &[u8]) -> Result<()> {
        self.host.copy_png(data)
    }

    pub fn read_clipboard_png(&self) -> Result<Vec<u8>> {
        self.host.read_clipboard_png()
    }

    pub fn read_clipboard_text(&self) -> Result<String> {
        self.host.read_clipboard_text()
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
}

pub(crate) fn configured_capture_save_path(
    base_dir: &Path,
    format: &str,
    naming_rule: &str,
    custom_file_name: &str,
    now: DateTime<Local>,
    path_exists: &dyn Fn(&Path) -> bool,
) -> PathBuf {
    let extension = capture_extension(format);
    let base_name = match naming_rule {
        "date" => format!("SnapLingo-{}", now.format("%Y-%m-%d")),
        "counter" => "Screenshot".to_string(),
        "custom" => sanitize_file_name(custom_file_name),
        _ => format!("SnapLingo-{}", now.format("%Y%m%d-%H%M%S")),
    };

    if naming_rule == "counter" {
        return first_available_capture_path(base_dir, &base_name, extension, 1, 3, path_exists);
    }

    let direct = base_dir.join(format!("{base_name}.{extension}"));
    if !path_exists(&direct) {
        return direct;
    }
    first_available_capture_path(base_dir, &base_name, extension, 2, 0, path_exists)
}

fn first_available_capture_path(
    base_dir: &Path,
    base_name: &str,
    extension: &str,
    start: u32,
    width: usize,
    path_exists: &dyn Fn(&Path) -> bool,
) -> PathBuf {
    for counter in start..u32::MAX {
        let suffix = if width == 0 {
            counter.to_string()
        } else {
            format!("{counter:0width$}")
        };
        let path = base_dir.join(format!("{base_name}_{suffix}.{extension}"));
        if !path_exists(&path) {
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

fn default_quick_capture_save_dir(paths: &CaptureOutputSystemPaths) -> PathBuf {
    paths
        .picture_dir
        .clone()
        .or_else(|| paths.home_dir.clone())
        .unwrap_or_else(|| paths.temp_dir.clone())
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

fn configured_capture_save_dir_for_system(configured: &str, home_dir: Option<&Path>) -> PathBuf {
    if configured == "~" || configured.starts_with("~/") {
        if let Some(home) = home_dir {
            return configured_capture_save_dir(configured, home);
        }
    }

    PathBuf::from(configured)
}

#[cfg(test)]
impl Default for CaptureOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
struct TestCaptureOutputHost;

#[cfg(test)]
impl CaptureOutputHost for TestCaptureOutputHost {
    fn system_paths(&self) -> CaptureOutputSystemPaths {
        let temp_dir = std::env::temp_dir();
        CaptureOutputSystemPaths {
            download_dir: Some(temp_dir.join("Downloads")),
            picture_dir: Some(temp_dir.join("Pictures")),
            home_dir: Some(temp_dir.join("Home")),
            temp_dir,
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

    fn copy_png(&self, _data: &[u8]) -> Result<()> {
        Err(AppError::System(
            "Clipboard is unavailable in a test host".into(),
        ))
    }

    fn read_clipboard_png(&self) -> Result<Vec<u8>> {
        Err(AppError::System(
            "Clipboard image is unavailable in a test host".into(),
        ))
    }

    fn read_clipboard_text(&self) -> Result<String> {
        Err(AppError::System(
            "Clipboard text is unavailable in a test host".into(),
        ))
    }
}
