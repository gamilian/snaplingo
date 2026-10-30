use crate::error::AppError;

#[allow(dead_code)]
pub fn rgba_image_to_png(image: image::RgbaImage) -> Result<Vec<u8>, AppError> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    use image::{ExtendedColorType, ImageEncoder};

    let mut buffer = Vec::new();
    // Frozen desktop pixels are decoded immediately by the WebView and OCR.
    // Sub avoids adaptive row analysis without inflating the full-screen buffer.
    PngEncoder::new_with_quality(&mut buffer, CompressionType::Fast, FilterType::Sub)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|error| AppError::System(format!("Failed to encode PNG: {error}")))?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_rgba_image_as_png() {
        let image = image::RgbaImage::from_fn(80, 60, |x, y| {
            image::Rgba([x as u8, y as u8, (x + y) as u8, (x * 3) as u8])
        });
        let png = rgba_image_to_png(image.clone()).unwrap();

        assert_eq!(
            &png[0..8],
            &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]
        );
        assert_eq!(image::load_from_memory(&png).unwrap().to_rgba8(), image);
    }
}
