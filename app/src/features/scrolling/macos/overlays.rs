use std::cell::RefCell;
use std::collections::HashMap;

use base64::Engine;
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSBox, NSBoxType, NSColor, NSImage, NSImageAlignment, NSImageScaling,
    NSImageView, NSScreenSaverWindowLevel, NSTitlePosition, NSWindow, NSWindowCollectionBehavior,
    NSWindowSharingType, NSWindowStyleMask,
};
use objc2_foundation::{NSData, NSPoint, NSRect, NSSize};

use snaplingo_core::capture::model::LogicalRect;
use snaplingo_ui::scrolling::{preview_rect, VIEWPORT_BORDER, VIEWPORT_COLOR};
use snaplingo_ui::Size;

thread_local! {
    static WINDOWS: RefCell<HashMap<String, objc2::rc::Retained<NSWindow>>> = RefCell::new(HashMap::new());
    static PREVIEWS: RefCell<HashMap<String, (objc2::rc::Retained<NSWindow>, objc2::rc::Retained<NSImageView>)>> = RefCell::new(HashMap::new());
}

/// Static viewport border and preview are AppKit windows, without WebViews.
pub(super) struct ScrollingOverlays;

impl ScrollingOverlays {
    pub(super) fn open(&self, id: &str, viewport: &LogicalRect, scale: f64) -> Result<(), String> {
        let mtm =
            MainThreadMarker::new().ok_or("Scrolling outline must open on the main thread")?;
        let border = f64::from(VIEWPORT_BORDER) * scale;
        let frame = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(
                viewport.width + border * 2.0,
                viewport.height + border * 2.0,
            ),
        );
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                frame,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        window.setOpaque(false);
        window.setBackgroundColor(Some(&NSColor::clearColor()));
        window.setHasShadow(false);
        window.setIgnoresMouseEvents(true);
        window.setCanHide(false);
        window.setHidesOnDeactivate(false);
        unsafe { window.setReleasedWhenClosed(false) };
        window.setSharingType(NSWindowSharingType::None);
        window.setLevel(NSScreenSaverWindowLevel);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        window.setExcludedFromWindowsMenu(true);

        let box_view = NSBox::new(mtm);
        box_view.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), frame.size));
        box_view.setBoxType(NSBoxType::Custom);
        box_view.setTitlePosition(NSTitlePosition::NoTitle);
        box_view.setContentViewMargins(NSSize::new(0.0, 0.0));
        box_view.setBorderWidth(border);
        box_view.setBorderColor(&NSColor::colorWithSRGBRed_green_blue_alpha(
            f64::from(VIEWPORT_COLOR[0]) / 255.,
            f64::from(VIEWPORT_COLOR[1]) / 255.,
            f64::from(VIEWPORT_COLOR[2]) / 255.,
            1.,
        ));
        box_view.setFillColor(&NSColor::clearColor());
        window.setContentView(Some(&box_view));
        let main_height = core_graphics::display::CGDisplay::main()
            .bounds()
            .size
            .height;
        window.setFrame_display(
            NSRect::new(
                NSPoint::new(
                    viewport.x - border,
                    main_height - viewport.y - viewport.height - border,
                ),
                frame.size,
            ),
            false,
        );
        window.orderFront(None);
        WINDOWS.with(|windows| windows.borrow_mut().insert(id.to_owned(), window));
        Ok(())
    }

    pub(super) fn hide_if_overlapping(&self, id: &str, viewport: &LogicalRect) -> bool {
        if MainThreadMarker::new().is_none() {
            return false;
        }
        WINDOWS.with(|windows| {
            let windows = windows.borrow();
            let Some(window) = windows.get(id) else {
                return false;
            };
            if !window.isVisible() {
                return false;
            }
            let frame = window.frame();
            let main_height = core_graphics::display::CGDisplay::main()
                .bounds()
                .size
                .height;
            let bounds = LogicalRect {
                x: frame.origin.x,
                y: main_height - frame.origin.y - frame.size.height,
                width: frame.size.width,
                height: frame.size.height,
            };
            if super::scrolling_geometry::overlaps(&bounds, viewport) {
                window.orderOut(None);
                true
            } else {
                false
            }
        })
    }

    pub(super) fn show(&self, id: &str) {
        if MainThreadMarker::new().is_none() {
            return;
        }
        WINDOWS.with(|windows| {
            if let Some(window) = windows.borrow().get(id) {
                window.orderFront(None);
            }
        });
    }

    pub(super) fn close(&self, id: &str) {
        if MainThreadMarker::new().is_none() {
            return;
        }
        WINDOWS.with(|windows| {
            if let Some(window) = windows.borrow_mut().remove(id) {
                window.orderOut(None);
                window.close();
            }
        });
    }

    pub(super) fn open_preview(&self, id: &str, rect: &LogicalRect) -> Result<(), String> {
        let mtm =
            MainThreadMarker::new().ok_or("Scrolling preview must open on the main thread")?;
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(rect.width, rect.height));
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                frame,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        window.setOpaque(false);
        window.setBackgroundColor(Some(&NSColor::clearColor()));
        window.setHasShadow(false);
        window.setIgnoresMouseEvents(true);
        window.setCanHide(false);
        window.setHidesOnDeactivate(false);
        unsafe { window.setReleasedWhenClosed(false) };
        window.setSharingType(NSWindowSharingType::None);
        window.setLevel(NSScreenSaverWindowLevel);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        window.setExcludedFromWindowsMenu(true);
        let image = NSImageView::new(mtm);
        image.setFrame(frame);
        image.setImageAlignment(NSImageAlignment::AlignBottom);
        image.setImageScaling(NSImageScaling::ScaleProportionallyDown);
        window.setContentView(Some(&image));
        let main_height = core_graphics::display::CGDisplay::main()
            .bounds()
            .size
            .height;
        window.setFrame_display(
            NSRect::new(
                NSPoint::new(rect.x, main_height - rect.y - rect.height),
                frame.size,
            ),
            false,
        );
        window.orderFront(None);
        PREVIEWS.with(|previews| previews.borrow_mut().insert(id.to_owned(), (window, image)));
        Ok(())
    }

    pub(super) fn update_preview(&self, id: &str, preview_base64: &str) {
        if MainThreadMarker::new().is_none() {
            return;
        }
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(preview_base64) else {
            return;
        };
        let data = NSData::with_bytes(&bytes);
        let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
            return;
        };
        PREVIEWS.with(|previews| {
            if let Some((window, view)) = previews.borrow().get(id) {
                let size = window.frame().size;
                let pixels = image.size();
                let rect = preview_rect(
                    Size {
                        width: size.width as f32,
                        height: size.height as f32,
                    },
                    Size {
                        width: pixels.width as f32,
                        height: pixels.height as f32,
                    },
                );
                view.setFrame(NSRect::new(
                    NSPoint::new(
                        rect.x as f64,
                        size.height - rect.y as f64 - rect.height as f64,
                    ),
                    NSSize::new(rect.width as f64, rect.height as f64),
                ));
                view.setImage(Some(&image));
            }
        });
    }

    pub(super) fn hide_preview_if_overlapping(&self, id: &str, viewport: &LogicalRect) -> bool {
        if MainThreadMarker::new().is_none() {
            return false;
        }
        PREVIEWS.with(|previews| {
            let previews = previews.borrow();
            let Some((window, _)) = previews.get(id) else {
                return false;
            };
            if !window.isVisible() {
                return false;
            }
            let frame = window.frame();
            let main_height = core_graphics::display::CGDisplay::main()
                .bounds()
                .size
                .height;
            let bounds = LogicalRect {
                x: frame.origin.x,
                y: main_height - frame.origin.y - frame.size.height,
                width: frame.size.width,
                height: frame.size.height,
            };
            if super::scrolling_geometry::overlaps(&bounds, viewport) {
                window.orderOut(None);
                true
            } else {
                false
            }
        })
    }

    pub(super) fn set_preview_visible(&self, id: &str, visible: bool) -> bool {
        if MainThreadMarker::new().is_none() {
            return false;
        }
        PREVIEWS.with(|previews| {
            let previews = previews.borrow();
            let Some((window, _)) = previews.get(id) else {
                return false;
            };
            if visible {
                window.orderFront(None);
            } else {
                window.orderOut(None);
            }
            visible
        })
    }

    pub(super) fn close_preview(&self, id: &str) {
        if MainThreadMarker::new().is_none() {
            return;
        }
        PREVIEWS.with(|previews| {
            if let Some((window, _)) = previews.borrow_mut().remove(id) {
                window.orderOut(None);
                window.close();
            }
        });
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(super) fn preview_frame(&self, id: &str) -> NSRect {
        PREVIEWS.with(|previews| previews.borrow().get(id).unwrap().1.frame())
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(super) fn state(&self, id: &str) -> (Option<bool>, Option<(bool, bool)>) {
        let outline =
            WINDOWS.with(|windows| windows.borrow().get(id).map(|window| window.isVisible()));
        let preview = PREVIEWS.with(|previews| {
            previews
                .borrow()
                .get(id)
                .map(|(window, view)| (window.isVisible(), view.image().is_some()))
        });
        (outline, preview)
    }
}
