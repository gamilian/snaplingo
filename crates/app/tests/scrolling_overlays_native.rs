// AppKit needs the process main thread; exercise actual native windows without a test harness.
#[cfg(target_os = "macos")]
mod domain {
    pub mod capture {
        #[derive(Clone, Debug)]
        pub struct LogicalRect {
            pub x: f64,
            pub y: f64,
            pub width: f64,
            pub height: f64,
        }
    }
}

#[cfg(target_os = "macos")]
mod scrolling_geometry {
    use crate::domain::capture::LogicalRect;

    pub(super) fn overlaps(a: &LogicalRect, b: &LogicalRect) -> bool {
        a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
    }
}

#[cfg(target_os = "macos")]
mod scrolling_overlays_macos {
    include!("../window/scrolling/macos/overlays.rs");
}

#[cfg(target_os = "macos")]
fn main() {
    use crate::domain::capture::LogicalRect;
    use base64::Engine;
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    let main = MainThreadMarker::new().expect("Native overlay test requires the main thread");
    let _app = NSApplication::sharedApplication(main);
    let overlays = scrolling_overlays_macos::ScrollingOverlays;
    let id = "native-scrolling-overlay-check";
    let viewport = LogicalRect {
        x: 80.0,
        y: 70.0,
        width: 240.0,
        height: 140.0,
    };
    let preview = LogicalRect {
        x: 360.0,
        y: 70.0,
        width: 110.0,
        height: 140.0,
    };
    overlays
        .open(id, &viewport, 1.0)
        .expect("native border opens");
    overlays
        .open_preview(id, &preview)
        .expect("native preview opens");
    assert_eq!(overlays.state(id), (Some(true), Some((true, false))));
    let png = image::DynamicImage::new_rgba8(2, 2);
    let mut bytes = std::io::Cursor::new(Vec::new());
    png.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    overlays.update_preview(
        id,
        &base64::engine::general_purpose::STANDARD.encode(bytes.into_inner()),
    );
    assert_eq!(overlays.state(id), (Some(true), Some((true, true))));
    assert!(overlays.hide_if_overlapping(id, &viewport));
    assert_eq!(overlays.state(id).0, Some(false));
    assert!(!overlays.hide_preview_if_overlapping(id, &viewport));
    overlays.show(id);
    assert_eq!(overlays.state(id).0, Some(true));
    assert!(!overlays.set_preview_visible(id, false));
    assert_eq!(overlays.state(id).1, Some((false, true)));
    assert!(overlays.set_preview_visible(id, true));
    assert_eq!(overlays.state(id).1, Some((true, true)));
    overlays.close_preview(id);
    overlays.close(id);
    assert_eq!(overlays.state(id), (None, None));
    println!("Native scrolling border and preview lifecycle passed");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
