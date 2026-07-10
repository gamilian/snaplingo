use crate::error::AppError;

#[allow(dead_code)]
pub fn rgba_image_to_png(image: image::RgbaImage) -> Result<Vec<u8>, AppError> {
    use std::io::Cursor;

    let mut buffer = Cursor::new(Vec::new());
    image
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|error| AppError::System(format!("Failed to encode PNG: {error}")))?;
    Ok(buffer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_rgba_image_as_png() {
        let png = rgba_image_to_png(image::RgbaImage::new(10, 10)).unwrap();

        assert_eq!(
            &png[0..8],
            &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]
        );
    }
}
