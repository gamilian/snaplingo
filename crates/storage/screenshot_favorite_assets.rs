use std::io::Cursor;
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use image::{GenericImageView, ImageFormat};

use crate::application::favorites::{FavoriteAssetStore, StoredFavoriteAssets};
use crate::application::history::{OcrHistoryAssetStore, StoredOcrHistoryAssets};
use crate::application::screenshot_favorites::{
    ScreenshotFavoriteAssetStore, StoredScreenshotAssets,
};
use crate::{AppError, Result};

pub struct FilesystemScreenshotFavoriteAssets {
    root: PathBuf,
}

pub struct FilesystemOcrHistoryAssets {
    inner: FilesystemScreenshotFavoriteAssets,
}

impl FilesystemOcrHistoryAssets {
    pub fn new(root: PathBuf) -> Self {
        Self {
            inner: FilesystemScreenshotFavoriteAssets::new(root.join("ocr")),
        }
    }
}

impl OcrHistoryAssetStore for FilesystemOcrHistoryAssets {
    fn store(&self, image_data: &[u8]) -> Result<StoredOcrHistoryAssets> {
        let image = image::load_from_memory(image_data)
            .map_err(|error| AppError::Other(format!("Invalid OCR source image: {error}")))?;
        let mut png = Cursor::new(Vec::new());
        image
            .write_to(&mut png, ImageFormat::Png)
            .map_err(|error| AppError::Other(format!("Failed to encode OCR source: {error}")))?;
        let stored = ScreenshotFavoriteAssetStore::store(&self.inner, &png.into_inner())?;
        Ok(StoredOcrHistoryAssets {
            source_path: stored.asset_path,
            thumbnail_path: stored.thumbnail_path,
        })
    }

    fn read(&self, relative_path: &str) -> Result<Vec<u8>> {
        ScreenshotFavoriteAssetStore::read(&self.inner, relative_path)
    }

    fn delete(&self, relative_path: &str) -> Result<()> {
        ScreenshotFavoriteAssetStore::delete(&self.inner, relative_path)
    }
}

impl FavoriteAssetStore for FilesystemOcrHistoryAssets {
    fn store_ocr(&self, image_data: &[u8]) -> Result<StoredFavoriteAssets> {
        let stored = OcrHistoryAssetStore::store(self, image_data)?;
        Ok(StoredFavoriteAssets {
            source_path: stored.source_path,
            thumbnail_path: stored.thumbnail_path,
        })
    }

    fn read(&self, relative_path: &str) -> Result<Vec<u8>> {
        OcrHistoryAssetStore::read(self, relative_path)
    }

    fn delete(&self, relative_path: &str) -> Result<()> {
        OcrHistoryAssetStore::delete(self, relative_path)
    }
}

impl FilesystemScreenshotFavoriteAssets {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn resolve(&self, relative_path: &str) -> Result<PathBuf> {
        let path = Path::new(relative_path);
        if path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(AppError::Other("Invalid screenshot asset path".into()));
        }
        Ok(self.root.join(path))
    }
}

impl ScreenshotFavoriteAssetStore for FilesystemScreenshotFavoriteAssets {
    fn store(&self, png_data: &[u8]) -> Result<StoredScreenshotAssets> {
        let image = image::load_from_memory_with_format(png_data, ImageFormat::Png)
            .map_err(|error| AppError::Other(format!("Invalid screenshot PNG: {error}")))?;
        let (width, height) = image.dimensions();
        let digest = format!("{:x}", md5::compute(png_data));
        let filename = format!("{}-{}.png", Utc::now().timestamp_millis(), digest);
        let asset_path = format!("screenshots/{filename}");
        let thumbnail_path = format!("thumbnails/{filename}");
        let absolute_asset = self.resolve(&asset_path)?;
        let absolute_thumbnail = self.resolve(&thumbnail_path)?;
        std::fs::create_dir_all(absolute_asset.parent().unwrap())?;
        std::fs::create_dir_all(absolute_thumbnail.parent().unwrap())?;

        std::fs::write(&absolute_asset, png_data)?;
        let thumbnail = image.thumbnail(480, 320);
        let mut thumbnail_png = Cursor::new(Vec::new());
        if let Err(error) = thumbnail.write_to(&mut thumbnail_png, ImageFormat::Png) {
            let _ = std::fs::remove_file(&absolute_asset);
            return Err(AppError::Other(format!(
                "Failed to encode screenshot thumbnail: {error}"
            )));
        }
        if let Err(error) = std::fs::write(&absolute_thumbnail, thumbnail_png.into_inner()) {
            let _ = std::fs::remove_file(&absolute_asset);
            return Err(error.into());
        }

        Ok(StoredScreenshotAssets {
            asset_path,
            thumbnail_path,
            width,
            height,
        })
    }

    fn read(&self, relative_path: &str) -> Result<Vec<u8>> {
        Ok(std::fs::read(self.resolve(relative_path)?)?)
    }

    fn delete(&self, relative_path: &str) -> Result<()> {
        let path = self.resolve(relative_path)?;
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    fn absolute_path(&self, relative_path: &str) -> Result<String> {
        Ok(self.resolve(relative_path)?.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use tempfile::tempdir;

    #[test]
    fn stores_original_and_bounded_thumbnail() {
        let dir = tempdir().unwrap();
        let store = FilesystemScreenshotFavoriteAssets::new(dir.path().to_path_buf());
        let image = ImageBuffer::from_pixel(800, 600, Rgba([10u8, 20, 30, 255]));
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut png, ImageFormat::Png)
            .unwrap();

        let stored = store.store(&png.into_inner()).unwrap();
        let thumbnail =
            image::load_from_memory(&store.read(&stored.thumbnail_path).unwrap()).unwrap();

        assert_eq!((stored.width, stored.height), (800, 600));
        assert!(thumbnail.width() <= 480);
        assert!(thumbnail.height() <= 320);
        assert!(store.absolute_path("../escape.png").is_err());
    }
}
