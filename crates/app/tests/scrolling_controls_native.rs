// AppKit callbacks and window lifecycle must run on the process main thread.
#[cfg(target_os = "macos")]
mod domain {
    pub mod config {
        #[derive(Clone, Default)]
        pub struct GeneralSettings {
            pub theme: String,
            pub interface_theme: String,
        }
    }
    pub mod capture {
        #![allow(dead_code, unused_imports)]
        pub use snaplingo_core::domain::capture::*;
    }
}

#[cfg(target_os = "macos")]
#[path = "../src/infrastructure/system/native_theme.rs"]
mod native_theme;

#[cfg(target_os = "macos")]
mod scrolling_geometry {
    use crate::domain::capture::LogicalRect;
    pub(super) fn overlaps(a: &LogicalRect, b: &LogicalRect) -> bool {
        a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
    }
}

#[cfg(target_os = "macos")]
mod scrolling_controls_macos {
    include!("../src/window/scrolling/macos/controls.rs");
}

#[cfg(target_os = "macos")]
mod infrastructure {
    pub mod system {
        pub mod native_view_snapshot {
            include!("../src/infrastructure/system/native_view_snapshot.rs");
        }
    }
}

#[cfg(target_os = "macos")]
fn main() {
    use domain::capture::{LogicalRect, ScrollingCaptureStatus, ScrollingCaptureView};
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;
    use scrolling_controls_macos::{self as controls, Action};
    use std::{cell::RefCell, rc::Rc};

    let mtm = MainThreadMarker::new().unwrap();
    let _app = NSApplication::sharedApplication(mtm);
    let id = "native-scrolling-controls-check";
    let rect = LogicalRect {
        x: 200.0,
        y: 100.0,
        width: 510.0,
        height: 165.0,
    };
    let mut view = ScrollingCaptureView {
        id: id.into(),
        revision: 0,
        width: 400,
        height: 700,
        can_undo: false,
        preview_base64: String::new(),
        status: ScrollingCaptureStatus::Capturing,
        hint: None,
        shortcut_actions: vec![],
        preview_expanded: false,
        preview_available: false,
        ui_scale: 1.25,
    };
    let actions = Rc::new(RefCell::new(Vec::new()));
    let clicked = actions.clone();
    controls::open(
        view.clone(),
        &rect,
        domain::config::GeneralSettings {
            theme: "light".into(),
            interface_theme: "native".into(),
        },
        move |action| clicked.borrow_mut().push(action),
    )
    .unwrap();
    assert_eq!(controls::rect(id), Some(rect.clone()));
    let state = controls::test_state(id).unwrap();
    assert_eq!(state.0, "400 × 700");
    assert!(!state.2[0]);
    assert!(!state.2[1]);
    assert!(state.3);
    if let Ok(directory) = std::env::var("SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join("scrolling-controls.png"),
            controls::test_snapshot(id),
        )
        .unwrap();
    }
    view.revision = 2;
    view.height = 1400;
    view.can_undo = true;
    view.preview_available = true;
    controls::update(view.clone());
    for index in 0..7 {
        controls::test_click(id, index);
    }
    assert_eq!(
        *actions.borrow(),
        vec![
            Action::Undo,
            Action::Preview,
            Action::Copy,
            Action::Save,
            Action::Pin,
            Action::Complete,
            Action::Cancel
        ]
    );
    let mut stale = view.clone();
    stale.revision = 1;
    stale.height = 100;
    controls::update(stale);
    assert_eq!(controls::test_state(id).unwrap().0, "400 × 1400");
    controls::set_working(id, true, None);
    let state = controls::test_state(id).unwrap();
    assert_eq!(state.1, "正在处理…");
    assert!(state.2.iter().all(|enabled| !enabled));
    controls::set_working(id, false, Some("保存失败，长图已保留".into()));
    assert_eq!(controls::test_state(id).unwrap().1, "保存失败，长图已保留");
    controls::test_click(id, 7);
    assert_eq!(actions.borrow().last(), Some(&Action::Resume));
    view.revision = 3;
    view.status = ScrollingCaptureStatus::Error;
    view.hint = Some("重试后采集仍失败，已保留长图".into());
    controls::update(view);
    controls::set_working(id, false, None);
    assert!(controls::test_retry_visible(id));
    if let Ok(directory) = std::env::var("SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR") {
        std::fs::write(
            std::path::Path::new(&directory).join("scrolling-error.png"),
            controls::test_snapshot(id),
        )
        .unwrap();
    }
    assert!(controls::hide_if_overlapping(id, &rect));
    assert!(!controls::test_state(id).unwrap().3);
    controls::show(id);
    assert!(controls::test_state(id).unwrap().3);
    for (theme, dark, surface, accent) in [
        ("native", false, [255, 255, 255], [57, 117, 220]),
        ("native", true, [41, 50, 62], [129, 171, 239]),
        ("studio", false, [255, 254, 250], [156, 109, 54]),
        ("studio", true, [37, 41, 44], [215, 167, 109]),
        ("paper", false, [253, 252, 247], [83, 115, 92]),
        ("paper", true, [41, 52, 43], [168, 191, 145]),
    ] {
        controls::update_theme(domain::config::GeneralSettings {
            theme: if dark { "dark" } else { "light" }.into(),
            interface_theme: theme.into(),
        });
        let bytes = controls::test_snapshot(id);
        let pixels = image::load_from_memory(&bytes).unwrap().into_rgba8();
        // AppKit snapshots use the display's backing scale, not necessarily
        // one bitmap pixel per logical point.
        let sample = |x: f64, y: f64| {
            let x = (x * f64::from(pixels.width()) / rect.width) as u32;
            let y = (y * f64::from(pixels.height()) / rect.height) as u32;
            pixels.get_pixel(x, y).0
        };
        // An 8-bit display-profile -> sRGB round trip can round by one level.
        for (actual, expected) in [
            (sample(100.0, 10.0), surface),
            (sample(317.0, 56.0), accent),
        ] {
            assert!(
                actual[..3]
                    .iter()
                    .zip(expected)
                    .all(|(actual, expected)| actual.abs_diff(expected) <= 1),
                "Rendered {theme} color {actual:?} differs from CSS {expected:?}"
            );
        }
        if let Ok(directory) = std::env::var("SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR") {
            std::fs::write(
                std::path::Path::new(&directory).join(format!(
                    "scrolling-{theme}-{}.png",
                    if dark { "dark" } else { "light" }
                )),
                bytes,
            )
            .unwrap();
        }
    }
    controls::close(id);
    assert!(controls::test_state(id).is_none());
    controls::close(id);
    println!("Native scrolling controls, action dispatch, status recovery and lifecycle passed");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
