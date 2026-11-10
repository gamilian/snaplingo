use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation::ConcreteCFType;
use core_graphics::window::{
    copy_window_info, kCGNullWindowID, kCGWindowListExcludeDesktopElements,
    kCGWindowListOptionOnScreenOnly,
};

use crate::domain::capture::LogicalRect;
use crate::error::{AppError, Result};

type WindowInfo = CFDictionary<CFString, CFType>;

pub(super) struct VisibleWindow {
    pub id: u32,
    pub owner_pid: i32,
    pub title: String,
    pub app_name: String,
    pub bounds: LogicalRect,
}

pub(super) fn visible_windows() -> Result<Vec<VisibleWindow>> {
    // Query WindowServer once: xcap's individual property getters each repeat
    // this enumeration and can observe different geometry during the same scan.
    let info = copy_window_info(
        kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
        kCGNullWindowID,
    )
    .ok_or_else(|| AppError::System("Failed to enumerate visible windows".into()))?;
    // CoreGraphics returns an array of dictionaries with CFString keys and CFType values.
    let windows: CFArray<WindowInfo> =
        unsafe { CFArray::wrap_under_get_rule(info.as_concrete_TypeRef()) };
    Ok(windows
        .iter()
        .filter_map(|info| window_from_info(&info))
        .collect())
}

fn value<T: ConcreteCFType>(info: &WindowInfo, key: &str) -> Option<T> {
    info.find(&CFString::new(key))?.downcast::<T>()
}

fn window_from_info(info: &WindowInfo) -> Option<VisibleWindow> {
    if !bool::from(value::<CFBoolean>(info, "kCGWindowIsOnscreen")?)
        || value::<CFNumber>(info, "kCGWindowSharingState")?.to_i32()? == 0
        || value::<CFNumber>(info, "kCGWindowAlpha")
            .and_then(|alpha| alpha.to_f64())
            .unwrap_or(1.0)
            <= 0.0
    {
        return None;
    }
    let bounds = value::<CFDictionary>(info, "kCGWindowBounds")?;
    // CGRect dictionary representations contain CFString keys and numeric CFType values.
    let bounds: WindowInfo =
        unsafe { CFDictionary::wrap_under_get_rule(bounds.as_concrete_TypeRef()) };
    let bounds = LogicalRect {
        x: value::<CFNumber>(&bounds, "X")?.to_f64()?,
        y: value::<CFNumber>(&bounds, "Y")?.to_f64()?,
        width: value::<CFNumber>(&bounds, "Width")?.to_f64()?,
        height: value::<CFNumber>(&bounds, "Height")?.to_f64()?,
    };
    if bounds.width < 2.0 || bounds.height < 2.0 {
        return None;
    }
    let title = value::<CFString>(info, "kCGWindowName")
        .map(|name| name.to_string())
        .unwrap_or_default();
    let app_name = value::<CFString>(info, "kCGWindowOwnerName")
        .map(|name| name.to_string())
        .unwrap_or_default();
    if title == "StatusIndicator" && app_name == "Window Server" {
        return None;
    }
    Some(VisibleWindow {
        id: value::<CFNumber>(info, "kCGWindowNumber")?
            .to_i64()?
            .try_into()
            .ok()?,
        owner_pid: value::<CFNumber>(info, "kCGWindowOwnerPID")?.to_i32()?,
        title,
        app_name,
        bounds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_foundation::dictionary::CFMutableDictionary;

    fn window_info() -> CFMutableDictionary<CFString, CFType> {
        let bounds = CFDictionary::from_CFType_pairs(&[
            (CFString::new("X"), CFNumber::from(-1200).as_CFType()),
            (CFString::new("Y"), CFNumber::from(100).as_CFType()),
            (CFString::new("Width"), CFNumber::from(400).as_CFType()),
            (CFString::new("Height"), CFNumber::from(300).as_CFType()),
        ]);
        CFMutableDictionary::from_CFType_pairs(&[
            (
                CFString::new("kCGWindowNumber"),
                CFNumber::from(7).as_CFType(),
            ),
            (
                CFString::new("kCGWindowOwnerPID"),
                CFNumber::from(123).as_CFType(),
            ),
            (
                CFString::new("kCGWindowIsOnscreen"),
                CFBoolean::true_value().as_CFType(),
            ),
            (
                CFString::new("kCGWindowSharingState"),
                CFNumber::from(1).as_CFType(),
            ),
            (
                CFString::new("kCGWindowAlpha"),
                CFNumber::from(1.0).as_CFType(),
            ),
            (
                CFString::new("kCGWindowOwnerName"),
                CFString::new("Editor").as_CFType(),
            ),
            (CFString::new("kCGWindowBounds"), bounds.as_CFType()),
        ])
    }

    #[test]
    fn accepts_untitled_visible_windows_and_preserves_negative_coordinates() {
        let info = window_info();
        let window = window_from_info(&info.to_immutable()).unwrap();
        assert_eq!(window.id, 7);
        assert_eq!(window.owner_pid, 123);
        assert_eq!(window.title, "");
        assert_eq!(window.app_name, "Editor");
        assert_eq!(
            window.bounds,
            LogicalRect {
                x: -1200.0,
                y: 100.0,
                width: 400.0,
                height: 300.0
            }
        );
    }

    #[test]
    fn excludes_hidden_transparent_and_nonsharing_windows() {
        for (key, replacement) in [
            ("kCGWindowIsOnscreen", CFBoolean::false_value().as_CFType()),
            ("kCGWindowAlpha", CFNumber::from(0.0).as_CFType()),
            ("kCGWindowSharingState", CFNumber::from(0).as_CFType()),
        ] {
            let mut info = window_info();
            info.set(CFString::new(key), replacement);
            assert!(window_from_info(&info.to_immutable()).is_none(), "{key}");
        }
    }

    #[test]
    fn ignores_records_without_usable_identity_or_geometry() {
        for key in ["kCGWindowNumber", "kCGWindowBounds", "kCGWindowOwnerPID"] {
            let mut info = window_info();
            info.set(CFString::new(key), CFString::new("unavailable").as_CFType());
            assert!(window_from_info(&info.to_immutable()).is_none(), "{key}");
        }
    }
}
