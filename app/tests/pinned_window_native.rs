#[cfg(target_os = "macos")]
mod native_view {
    #![allow(dead_code)]
    include!("../src/features/pinned/macos/view.rs");
}

#[cfg(target_os = "macos")]
mod integrations {
    pub mod system {
        pub mod native_view_snapshot {
            include!("../src/integrations/system/native_view_snapshot.rs");
        }
    }
}

#[cfg(target_os = "macos")]
fn main() {
    use base64::Engine;
    use native_view::{self as native, Action, Preferences};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags as Flags, NSEventType};
    use objc2_foundation::{NSPoint, NSString};
    use snaplingo_core::capture::model::LogicalRect;
    use snaplingo_core::capture::model::PinnedImageView;
    use std::{cell::RefCell, io::Cursor, rc::Rc};

    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    let image = |id: &str, width, height| {
        let pixels = image::RgbaImage::from_pixel(width, height, image::Rgba([180, 60, 30, 255]));
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(pixels)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        PinnedImageView {
            id: id.into(),
            image_base64: base64::engine::general_purpose::STANDARD.encode(png.into_inner()),
            width,
            height,
            source_text: Some("native pin text".into()),
        }
    };
    let key = |text: &str, flags, code| {
        NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            NSEventType::KeyDown, NSPoint::new(0.0, 0.0), flags, 0.0, 0, None,
            &NSString::from_str(text), &NSString::from_str(text), false, code,
        ).unwrap()
    };
    let id = "native-pin-check";
    let actions = Rc::new(RefCell::new(Vec::new()));
    let clicked = actions.clone();
    native::open(
        image(id, 600, 300),
        &LogicalRect {
            x: 180.0,
            y: 100.0,
            width: 600.0,
            height: 300.0,
        },
        Preferences {
            opacity: 0.75,
            shadow: true,
            border: true,
            border_shortcut: "b".into(),
        },
        move |action, size| clicked.borrow_mut().push((action, size)),
    )
    .unwrap();
    assert!(native::visible(id));
    assert!(native::focus(id).unwrap());
    let (toolbar, buttons) = native::test_toolbar_frames(id);
    assert_eq!(toolbar.origin, NSPoint::new(460., 4.));
    assert_eq!(toolbar.size, objc2_foundation::NSSize::new(136., 30.));
    assert_eq!(
        buttons
            .iter()
            .map(|rect| (
                rect.origin.x,
                rect.origin.y,
                rect.size.width,
                rect.size.height
            ))
            .collect::<Vec<_>>(),
        [
            (2., 2., 50., 26.),
            (52., 2., 50., 26.),
            (102., 2., 30., 26.),
        ]
    );
    native::test_key(id, &key(" ", Flags::empty(), 49));
    if let Ok(directory) = std::env::var("SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join("pin-toolbar.png"),
            native::test_snapshot(id),
        )
        .unwrap();
    }
    native::test_key(id, &key("\u{1b}", Flags::empty(), 53));
    assert!(native::visible(id));
    // Background test processes do not own the desktop keyboard.
    if app.isActive() {
        assert!(native::has_key_window());
    }
    native::update_preferences(Preferences {
        opacity: 1.0,
        shadow: false,
        border: false,
        border_shortcut: "b".into(),
    });
    assert_eq!(native::test_presentation(id).opacity, 0.75);
    let before = native::test_frame(id);
    native::test_key(id, &key("+", Flags::empty(), 24));
    assert_eq!(native::test_presentation(id).zoom, 1.1);
    let after = native::test_frame(id);
    assert_eq!(after.size.width, 660.0);
    assert_eq!(
        before.origin.x + before.size.width / 2.0,
        after.origin.x + after.size.width / 2.0
    );
    native::test_key(id, &key("1", Flags::CapsLock, 18));
    assert_eq!(native::test_presentation(id).rotation, 90);
    assert_eq!(native::test_frame(id).size.width, 330.0);
    native::test_key(id, &key("3", Flags::empty(), 20));
    assert!(native::test_presentation(id).flip_x);
    native::test_key(id, &key("5", Flags::empty(), 23));
    native::test_key(id, &key("6", Flags::empty(), 22));
    assert!(native::test_presentation(id).grayscale);
    assert!(native::test_presentation(id).inverted);
    native::test_key(id, &key("c", Flags::Command, 8));
    native::test_key(id, &key("c", Flags::Command | Flags::Shift, 8));
    native::test_key(id, &key("s", Flags::Command | Flags::Shift, 1));
    native::test_key(id, &key("b", Flags::empty(), 11));
    let commands = actions.borrow();
    assert!(matches!(commands[0].0, Action::Copy));
    assert_eq!(commands[0].1.width, 660);
    assert!(matches!(&commands[1].0, Action::CopyText(text) if text == "native pin text"));
    assert!(matches!(commands[2].0, Action::QuickSave));
    assert!(matches!(commands[3].0, Action::ToggleBorder));
    drop(commands);
    assert_eq!(
        native::test_inspection(id, Some(NSPoint::new(20.0, 20.0))),
        Some([180, 60, 30])
    );
    assert_eq!(native::test_inspection(id, None), None);
    native::replace(image(id, 200, 100)).unwrap();
    let presentation = native::test_presentation(id);
    assert_eq!(presentation.zoom, 1.0);
    assert_eq!(presentation.rotation, 0);
    assert_eq!(presentation.opacity, 0.75);
    assert!(presentation.grayscale && presentation.inverted);
    assert_eq!(native::test_frame(id).size.width, 200.0);
    native::hide(id);
    assert_eq!(native::test_decoded_images(), 0);
    assert!(!native::visible(id));
    assert!(native::show(id).unwrap());
    assert_eq!(native::test_decoded_images(), 1);
    assert!(native::visible(id));
    native::close(id);
    assert!(!native::show(id).unwrap());
    assert!(native::ids().is_empty());
    let id = "native-pin-render-check";
    let pixels = image::RgbaImage::from_fn(600, 300, |x, y| match (x < 300, y < 150) {
        (true, true) => image::Rgba([240, 40, 40, 255]),
        (false, true) => image::Rgba([40, 240, 40, 255]),
        (true, false) => image::Rgba([40, 40, 240, 255]),
        (false, false) => image::Rgba([240, 240, 40, 255]),
    });
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(pixels)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    native::open(
        PinnedImageView {
            id: id.into(),
            image_base64: base64::engine::general_purpose::STANDARD.encode(png.into_inner()),
            width: 600,
            height: 300,
            source_text: None,
        },
        &LogicalRect {
            x: 180.0,
            y: 100.0,
            width: 600.0,
            height: 300.0,
        },
        Preferences {
            opacity: 1.0,
            shadow: true,
            border: false,
            border_shortcut: "b".into(),
        },
        |_, _| {},
    )
    .unwrap();
    let snapshot = |name: &str, expected: [&str; 4]| {
        let bytes = native::test_snapshot(id);
        let pixels = image::load_from_memory(&bytes).unwrap().to_rgba8();
        let points = [(0.1, 0.1), (0.9, 0.1), (0.1, 0.9), (0.9, 0.9)];
        for ((x, y), expected) in points.into_iter().zip(expected) {
            let pixel = pixels.get_pixel(
                (pixels.width() as f64 * x) as u32,
                (pixels.height() as f64 * y) as u32,
            );
            let actual = if pixel[0] > 150 && pixel[1] > 150 {
                "yellow"
            } else if pixel[0] > 150 {
                "red"
            } else if pixel[1] > 150 {
                "green"
            } else {
                "blue"
            };
            assert_eq!(actual, expected, "Native rendered corner in {name}");
            assert!(pixel[3] > 200);
        }
        if let Ok(directory) = std::env::var("SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(std::path::Path::new(&directory).join(name), bytes).unwrap();
        }
    };
    snapshot("pin-original.png", ["red", "green", "blue", "yellow"]);
    native::test_key(id, &key("1", Flags::empty(), 18));
    snapshot("pin-rotated.png", ["blue", "red", "yellow", "green"]);
    native::test_key(id, &key("3", Flags::empty(), 20));
    snapshot("pin-flipped.png", ["yellow", "green", "blue", "red"]);
    native::close(id);
    println!("Native pinned image rendering, transforms, input, inspection, replacement and lifecycle passed");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
