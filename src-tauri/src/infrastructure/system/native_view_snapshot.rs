use objc2_app_kit::{NSBitmapImageFileType, NSView};
use objc2_foundation::NSDictionary;

pub(crate) fn snapshot(view: &NSView) -> Vec<u8> {
    let bounds = view.bounds();
    let bitmap = view
        .bitmapImageRepForCachingDisplayInRect(bounds)
        .expect("Native view can render to a bitmap");
    view.cacheDisplayInRect_toBitmapImageRep(bounds, &bitmap);
    let bytes = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }
    .expect("Native bitmap can encode as PNG");
    // The immutable data was created here and has no mutable aliases.
    unsafe { bytes.as_bytes_unchecked() }.to_vec()
}
