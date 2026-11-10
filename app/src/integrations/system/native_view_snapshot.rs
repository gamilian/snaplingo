use objc2_app_kit::{NSBitmapImageFileType, NSColorRenderingIntent, NSColorSpace, NSView};
use objc2_foundation::NSDictionary;

pub(crate) fn try_snapshot(view: &NSView) -> Result<Vec<u8>, String> {
    let bounds = view.bounds();
    let bitmap = view
        .bitmapImageRepForCachingDisplayInRect(bounds)
        .ok_or("Native view could not render to a bitmap")?;
    view.cacheDisplayInRect_toBitmapImageRep(bounds, &bitmap);
    // Caching uses the display profile (for example Display P3). Encode sRGB
    // so raster comparisons and ordinary PNG viewers see the UI's CSS colors.
    let bitmap = bitmap
        .bitmapImageRepByConvertingToColorSpace_renderingIntent(
            &NSColorSpace::sRGBColorSpace(),
            NSColorRenderingIntent::RelativeColorimetric,
        )
        .ok_or("Native snapshot could not convert to sRGB")?;
    let bytes = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }
    .ok_or("Native bitmap could not encode as PNG")?;
    // The immutable data was created here and has no mutable aliases.
    Ok(unsafe { bytes.as_bytes_unchecked() }.to_vec())
}

#[cfg(any(test, feature = "test-support"))]
pub(crate) fn snapshot(view: &NSView) -> Vec<u8> {
    try_snapshot(view).expect("Native view can render a PNG snapshot")
}
