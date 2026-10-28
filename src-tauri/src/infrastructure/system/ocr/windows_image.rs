use crate::{AppError, Result};

pub(super) fn bgra_pixels_from_image_data_with_limit(
    image_data: &[u8],
    max_dimension: u32,
) -> Result<(Vec<u8>, i32, i32)> {
    let image = image::load_from_memory(image_data)
        .map_err(|error| AppError::Other(format!("Invalid Windows OCR image data: {error}")))?;
    let max_dimension = max_dimension.max(1);
    let image = if image.width().max(image.height()) > max_dimension {
        image.thumbnail(max_dimension, max_dimension)
    } else {
        image
    };
    let width = i32::try_from(image.width())
        .map_err(|_| AppError::Other("Windows OCR image width is too large".to_string()))?;
    let height = i32::try_from(image.height())
        .map_err(|_| AppError::Other("Windows OCR image height is too large".to_string()))?;
    if width == 0 || height == 0 {
        return Err(AppError::Other(
            "Invalid Windows OCR image data: image dimensions must be non-zero".to_string(),
        ));
    }

    let rgba = image.into_rgba8();
    let mut bgra = rgba.into_raw();
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }

    Ok((bgra, width, height))
}

#[cfg(test)]
mod tests {
    use image::{ImageBuffer, ImageFormat, Rgba};

    use super::bgra_pixels_from_image_data_with_limit;

    #[test]
    fn converts_rgba_pixels_to_bgra() {
        let image = ImageBuffer::from_pixel(1, 1, Rgba::<u8>([1, 2, 3, 4]));
        let mut encoded = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut encoded), ImageFormat::Png)
            .unwrap();

        let (pixels, width, height) = bgra_pixels_from_image_data_with_limit(&encoded, 4).unwrap();

        assert_eq!((width, height), (1, 1));
        assert_eq!(pixels, vec![3, 2, 1, 4]);
    }

    #[test]
    fn downsizes_large_images_to_the_windows_ocr_limit() {
        let image = ImageBuffer::from_pixel(12, 6, Rgba::<u8>([1, 2, 3, 255]));
        let mut encoded = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut encoded), ImageFormat::Png)
            .unwrap();

        let (pixels, width, height) = bgra_pixels_from_image_data_with_limit(&encoded, 4).unwrap();

        assert_eq!((width, height), (4, 2));
        assert_eq!(pixels.len(), 4 * 2 * 4);
    }

    #[test]
    fn rejects_invalid_encoded_images() {
        assert!(bgra_pixels_from_image_data_with_limit(b"not an image", 4096).is_err());
    }
}
