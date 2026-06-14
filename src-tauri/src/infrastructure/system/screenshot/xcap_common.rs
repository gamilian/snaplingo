use crate::error::AppError;
use super::backend::{rgba_image_to_png, ScreenRegion};
use xcap::Monitor;

/// Get the primary monitor (not just the first enumerated one).
fn get_primary_monitor() -> Result<Monitor, AppError> {
    Monitor::all()
        .map_err(|e| with_platform_hint(format!("Failed to enumerate monitors: {}", e)))?
        .into_iter()
        .find(|m| m.is_primary())
        .ok_or_else(|| AppError::System("No primary monitor found".to_string()))
}

/// Wrap an error message with a platform-specific troubleshooting hint.
fn with_platform_hint(msg: String) -> AppError {
    let hint = if cfg!(target_os = "linux") {
        " (Linux: ensure xdg-desktop-portal is running for Wayland support)"
    } else {
        ""
    };
    AppError::System(format!("{}{}", msg, hint))
}

/// Capture the primary monitor's full screen as PNG bytes.
pub fn capture_full_screen_png() -> Result<Vec<u8>, AppError> {
    let primary = get_primary_monitor()?;
    let image = primary
        .capture_image()
        .map_err(|e| with_platform_hint(format!("Screenshot failed: {}", e)))?;
    rgba_image_to_png(image)
}

/// Capture a region of the primary monitor as PNG bytes.
///
/// Uses XCap's native `capture_region`, which handles global-to-local
/// coordinate mapping internally.
pub fn capture_region_png(region: ScreenRegion) -> Result<Vec<u8>, AppError> {
    let primary = get_primary_monitor()?;
    let image = primary
        .capture_region(
            region.x.max(0) as u32,
            region.y.max(0) as u32,
            region.width,
            region.height,
        )
        .map_err(|e| with_platform_hint(format!("Region capture failed: {}", e)))?;
    rgba_image_to_png(image)
}
