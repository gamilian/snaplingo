use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;

use base64::Engine;
use objc2::rc::Retained;
use objc2::{
    define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSAffineTransformNSAppKitAdditions, NSBackingStoreType, NSBezierPath, NSBitmapFormat,
    NSBitmapImageRep, NSBox, NSBoxType, NSButton, NSColor, NSCompositingOperation,
    NSDeviceRGBColorSpace, NSEvent, NSEventModifierFlags, NSFloatingWindowLevel, NSFont,
    NSGraphicsContext, NSImage, NSImageInterpolation, NSMenu, NSMenuItem, NSPanel, NSTextField,
    NSTitlePosition, NSTrackingArea, NSTrackingAreaOptions, NSView, NSWindowCollectionBehavior,
    NSWindowStyleMask,
};
use objc2_foundation::{
    NSAffineTransform, NSData, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use crate::domain::capture::{LogicalRect, PinnedImageExportSize, PinnedImageView};
use snaplingo_ui::pinned::{color_text, Presentation, State};
pub(super) use snaplingo_ui::pinned::{Action, Preferences};
use snaplingo_ui::{Key, Modifiers, Point};

struct ImageState {
    ui: State,
    data: Retained<NSData>,
    original: Option<Retained<NSImage>>,
    filtered: Option<Retained<NSImage>>,
}

impl std::ops::Deref for ImageState {
    type Target = State;
    fn deref(&self) -> &State {
        &self.ui
    }
}
impl std::ops::DerefMut for ImageState {
    fn deref_mut(&mut self) -> &mut State {
        &mut self.ui
    }
}

impl ImageState {
    fn new(image: PinnedImageView, preferences: Preferences) -> Result<Self, String> {
        let png = base64::engine::general_purpose::STANDARD
            .decode(&image.image_base64)
            .map_err(|error| error.to_string())?;
        let data = NSData::with_bytes(&png);
        let original = NSImage::initWithData(NSImage::alloc(), &data)
            .ok_or("Could not decode pinned image")?;
        original.setSize(NSSize::new(image.width as f64, image.height as f64));
        Ok(Self {
            ui: State::new(image.width, image.height, image.source_text, preferences),
            data,
            original: Some(original),
            filtered: None,
        })
    }

    fn update_filter(&mut self) -> Result<(), String> {
        let presentation = &self.presentation;
        if !presentation.grayscale && !presentation.inverted {
            self.filtered = None;
            return Ok(());
        }
        // This immutable NSData is owned by the view and never exposed for mutation.
        let mut pixels = image::load_from_memory(unsafe { self.data.as_bytes_unchecked() })
            .map_err(|error| error.to_string())?
            .to_rgba8();
        for pixel in pixels.pixels_mut() {
            if presentation.grayscale {
                let gray = (0.2126 * f64::from(pixel[0])
                    + 0.7152 * f64::from(pixel[1])
                    + 0.0722 * f64::from(pixel[2]))
                .round() as u8;
                pixel[0] = gray;
                pixel[1] = gray;
                pixel[2] = gray;
            }
            if presentation.inverted {
                for channel in &mut pixel.0[..3] {
                    *channel = 255 - *channel;
                }
            }
        }
        let bitmap = unsafe {
            NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
                NSBitmapImageRep::alloc(), std::ptr::null_mut(), self.width as isize, self.height as isize,
                8, 4, true, false, NSDeviceRGBColorSpace, NSBitmapFormat::AlphaNonpremultiplied,
                self.width as isize * 4, 32,
            )
        }.ok_or("Could not allocate filtered image")?;
        let destination = bitmap.bitmapData();
        if destination.is_null() {
            return Err("Filtered image has no pixels".into());
        }
        // AppKit allocated a tightly packed RGBA buffer of the dimensions above.
        unsafe {
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), destination, pixels.len());
        }
        let filtered = NSImage::initWithSize(
            NSImage::alloc(),
            NSSize::new(self.width as f64, self.height as f64),
        );
        filtered.addRepresentation(&bitmap);
        self.filtered = Some(filtered);
        Ok(())
    }

    fn restore(&mut self) -> Result<(), String> {
        if self.original.is_none() {
            let image = NSImage::initWithData(NSImage::alloc(), &self.data)
                .ok_or("Could not restore pinned image")?;
            image.setSize(NSSize::new(self.width as f64, self.height as f64));
            self.original = Some(image);
            self.update_filter()?;
        }
        Ok(())
    }
}

struct LensState {
    image: Retained<NSImage>,
    point: NSPoint,
    color: [u8; 3],
}

struct LensIvars {
    state: RefCell<Option<LensState>>,
    label: OnceCell<Retained<NSTextField>>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoPinnedImageLens"]
    #[thread_kind = MainThreadOnly]
    #[ivars = LensIvars]
    struct LensView;

    unsafe impl NSObjectProtocol for LensView {}

    impl LensView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }

        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView { std::ptr::null_mut() }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let state = self.ivars().state.borrow();
            let Some(state) = state.as_ref() else { return; };
            NSColor::blackColor().setFill();
            NSBezierPath::bezierPathWithRect(self.bounds()).fill();
            NSGraphicsContext::saveGraphicsState_class();
            if let Some(context) = NSGraphicsContext::currentContext() { context.setImageInterpolation(NSImageInterpolation::None); }
            let source = NSRect::new(
                NSPoint::new(state.point.x - 10.0, state.image.size().height - state.point.y - 7.0),
                NSSize::new(20.0, 14.0),
            );
            unsafe { state.image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                self.bounds(), source, NSCompositingOperation::SourceOver, 1.0, true, None,
            ); }
            NSGraphicsContext::restoreGraphicsState_class();
            NSColor::whiteColor().setStroke();
            let crosshair = NSBezierPath::bezierPath();
            crosshair.moveToPoint(NSPoint::new(80.0, 0.0));
            crosshair.lineToPoint(NSPoint::new(80.0, 112.0));
            crosshair.moveToPoint(NSPoint::new(0.0, 56.0));
            crosshair.lineToPoint(NSPoint::new(160.0, 56.0));
            crosshair.setLineWidth(1.0);
            crosshair.stroke();
        }
    }
);

struct PinIvars {
    image: RefCell<ImageState>,
    dispatch: Box<dyn Fn(Action, PinnedImageExportSize)>,
    toolbar: OnceCell<Retained<NSBox>>,
    lens: OnceCell<Retained<LensView>>,
    inspection: RefCell<Option<image::RgbaImage>>,
}

define_class!(
    #[unsafe(super(NSPanel))]
    #[name = "SnapLingoPinnedImagePanel"]
    #[thread_kind = MainThreadOnly]
    struct PinPanel;

    unsafe impl NSObjectProtocol for PinPanel {}

    impl PinPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool { true }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool { false }
    }
);

define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoPinnedImageView"]
    #[thread_kind = MainThreadOnly]
    #[ivars = PinIvars]
    struct PinView;

    unsafe impl NSObjectProtocol for PinView {}

    impl PinView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool { true }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool { true }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let state = self.ivars().image.borrow();
            let Some(original) = state.original.as_ref() else { return; };
            let presentation = &state.presentation;
            let size = presentation.export_size(state.width, state.height);
            let bounds = self.bounds();
            NSGraphicsContext::saveGraphicsState_class();
            let transform = NSAffineTransform::new();
            transform.translateXBy_yBy(bounds.size.width / 2.0, bounds.size.height / 2.0);
            transform.rotateByDegrees(presentation.rotation as f64);
            transform.scaleXBy_yBy(if presentation.flip_x { -1.0 } else { 1.0 }, if presentation.flip_y { -1.0 } else { 1.0 });
            transform.concat();
            let image = state.filtered.as_ref().unwrap_or(original);
            unsafe { image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                NSRect::new(NSPoint::new(-(size.width as f64) / 2.0, -(size.height as f64) / 2.0), NSSize::new(size.width as f64, size.height as f64)),
                NSRect::new(NSPoint::new(0.0, 0.0), image.size()),
                NSCompositingOperation::SourceOver, presentation.opacity, true, None,
            ); }
            NSGraphicsContext::restoreGraphicsState_class();
            if self.ivars().image.borrow().preferences.border {
                NSColor::colorWithSRGBRed_green_blue_alpha(0.23, 0.51, 0.96, 1.0).setStroke();
                let border = NSBezierPath::bezierPathWithRect(NSRect::new(NSPoint::new(0.5, 0.5), NSSize::new((bounds.size.width - 1.0).max(0.0), (bounds.size.height - 1.0).max(0.0))));
                border.setLineWidth(1.0);
                border.stroke();
            }
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            if event.modifierFlags().contains(NSEventModifierFlags::Control) { self.open_menu(event); return; }
            if let Some(window) = self.window() {
                window.makeKeyAndOrderFront(None);
                window.makeFirstResponder(Some(self));
                let flags = input_modifiers(event);
                if event.clickCount() >= 2 {
                    if flags == NSEventModifierFlags::Shift { self.change(|view| view.thumbnail = !view.thumbnail); return; }
                    if flags.is_empty() { self.emit(Action::Close); return; }
                }
                window.performWindowDragWithEvent(event);
            }
        }

        #[unsafe(method(otherMouseDown:))]
        fn other_mouse_down(&self, event: &NSEvent) {
            if event.buttonNumber() == 2 && input_modifiers(event).is_empty() {
                let opacity = self.ivars().image.borrow().preferences.opacity;
                self.change(|view| { view.zoom = 1.0; view.thumbnail = false; view.opacity = opacity; });
            }
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) { self.open_menu(event); }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            self.ivars().image.borrow_mut().modifiers(modifiers(event));
            let changed = self.ivars().image.borrow_mut().scroll(event.scrollingDeltaY());
            if changed {
                self.change(|_| {});
            }
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, event: &NSEvent) {
            self.ivars().image.borrow_mut().hovered = true;
            self.update_toolbar();
            self.ivars().image.borrow_mut().modifiers(modifiers(event));
            self.inspect(self.convertPoint_fromView(event.locationInWindow(), None));
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.ivars().image.borrow_mut().hovered = false;
            self.update_toolbar();
            self.hide_lens();
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            if self.ivars().image.borrow().inspecting() { self.inspect(self.convertPoint_fromView(event.locationInWindow(), None)); }
        }

        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &NSEvent) {
            self.ivars().image.borrow_mut().modifiers(modifiers(event));
            let requested = self.ivars().image.borrow().inspecting();
            if requested {
                if let Some(window) = self.window() {
                    let point = window.convertPointFromScreen(NSEvent::mouseLocation());
                    self.inspect(self.convertPoint_fromView(point, None));
                }
            } else { self.hide_lens(); }
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) { self.handle_key(event); }

        #[unsafe(method(toolbarAction:))]
        fn toolbar_action(&self, sender: &NSButton) {
            self.ivars().image.borrow_mut().forced_toolbar = false;
            self.update_toolbar();
            self.emit(match sender.tag() { 0 => Action::Copy, 1 => Action::Save, _ => Action::Close });
        }

        #[unsafe(method(menuAction:))]
        fn menu_action(&self, sender: &NSMenuItem) {
            let action = self.ivars().image.borrow_mut().menu(sender.tag() as u32);
            if let Some(action) = action { self.emit(action); }
            else { self.change(|_| {}); }
        }
    }
);

impl PinView {
    fn emit(&self, action: Action) {
        let size = {
            let image = self.ivars().image.borrow();
            image.presentation.export_size(image.width, image.height)
        };
        (self.ivars().dispatch)(
            action,
            PinnedImageExportSize {
                width: size.width,
                height: size.height,
            },
        );
    }

    fn change(&self, operation: impl FnOnce(&mut Presentation)) {
        let (width, height) = {
            let mut image = self.ivars().image.borrow_mut();
            operation(&mut image.presentation);
            image.presentation.window_size(image.width, image.height)
        };
        if let Some(window) = self.window() {
            let frame = window.frame();
            if frame.size.width != width || frame.size.height != height {
                window.setFrame_display(
                    NSRect::new(
                        NSPoint::new(
                            frame.origin.x + (frame.size.width - width) / 2.0,
                            frame.origin.y + (frame.size.height - height) / 2.0,
                        ),
                        NSSize::new(width, height),
                    ),
                    true,
                );
            }
        }
        if let Some(toolbar) = self.ivars().toolbar.get() {
            let rect = self.ivars().image.borrow().toolbar_rect();
            toolbar.setFrameOrigin(NSPoint::new(rect.x as f64, rect.y as f64));
        }
        self.hide_lens();
        self.setNeedsDisplay(true);
    }

    fn handle_key(&self, event: &NSEvent) {
        let Some(key) = key(event) else {
            return;
        };
        let color = self.ivars().lens.get().and_then(|lens| {
            lens.ivars()
                .state
                .borrow()
                .as_ref()
                .map(|state| state.color)
        });
        let (action, movement) = {
            let mut image = self.ivars().image.borrow_mut();
            image.modifiers(modifiers(event));
            let filters = (image.presentation.grayscale, image.presentation.inverted);
            let action = image.ui.key(key, event.isARepeat(), color);
            let movement = if action.is_none() {
                image.movement(key)
            } else {
                None
            };
            if filters != (image.presentation.grayscale, image.presentation.inverted) {
                if let Err(error) = image.update_filter() {
                    log::warn!("Could not filter pinned image: {error}");
                }
            }
            (action, movement)
        };
        if let Some(action) = action {
            self.emit(action);
            return;
        }
        if let Some((x, y)) = movement {
            if let Some(window) = self.window() {
                let frame = window.frame();
                window.setFrameOrigin(NSPoint::new(frame.origin.x + x, frame.origin.y - y));
            }
            return;
        }
        self.change(|_| {});
        self.update_toolbar();
    }

    fn update_toolbar(&self) {
        if let Some(toolbar) = self.ivars().toolbar.get() {
            let hidden =
                !self.ivars().image.borrow().forced_toolbar && !self.ivars().image.borrow().hovered;
            if toolbar.isHidden() != hidden {
                toolbar.setHidden(hidden);
            }
        }
    }

    fn hide_lens(&self) {
        self.ivars().inspection.borrow_mut().take();
        if let Some(lens) = self.ivars().lens.get() {
            lens.setHidden(true);
            lens.ivars().state.borrow_mut().take();
        }
    }

    fn inspect(&self, point: NSPoint) {
        let bounds = self.bounds();
        if !self.ivars().image.borrow().inspecting()
            || point.x < 0.0
            || point.y < 0.0
            || point.x >= bounds.size.width
            || point.y >= bounds.size.height
        {
            self.hide_lens();
            return;
        }
        self.ivars().image.borrow_mut().cursor = Point {
            x: point.x as f32,
            y: point.y as f32,
        };
        let image = self.ivars().image.borrow();
        let Some(original) = image.original.as_ref() else {
            return;
        };
        let mut inspection = self.ivars().inspection.borrow_mut();
        if inspection.is_none() {
            // Read original PNG channels; display color conversion would change copied RGB values.
            *inspection = image::load_from_memory(unsafe { image.data.as_bytes_unchecked() })
                .ok()
                .map(|image| image.to_rgba8());
        }
        let Some(pixels) = inspection.as_ref() else {
            return;
        };
        let (x, y) = image
            .presentation
            .sample_point((point.x, point.y), image.width, image.height);
        let pixel = pixels.get_pixel(x, y);
        let color = [pixel[0], pixel[1], pixel[2]];
        if let Some(lens) = self.ivars().lens.get() {
            let rect = image.lens_rect();
            lens.setFrameOrigin(NSPoint::new(rect.x as f64, rect.y as f64));
            let rgb = image.rgb;
            if let Some(label) = lens.ivars().label.get() {
                let text = color_text(color, rgb);
                if label.stringValue().to_string() != text {
                    label.setStringValue(&NSString::from_str(&text));
                }
            }
            *lens.ivars().state.borrow_mut() = Some(LensState {
                image: original.clone(),
                point: NSPoint::new(x as f64, y as f64),
                color,
            });
            lens.setHidden(false);
            lens.setNeedsDisplay(true);
        }
    }

    fn open_menu(&self, event: &NSEvent) {
        let mtm = self.mtm();
        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);
        let border = self.ivars().image.borrow().preferences.border;
        for (tag, title) in [
            (1, "重置大小"),
            (2, "透明度 100%"),
            (3, "透明度 75%"),
            (4, "透明度 50%"),
            (5, "移到另一组"),
            (
                6,
                if border {
                    "隐藏边框"
                } else {
                    "显示边框"
                },
            ),
            (7, "收起贴图"),
            (8, "隐藏当前组"),
            (9, "复制"),
            (10, "另存为"),
            (11, "快速保存 PNG"),
            (12, "关闭当前贴图"),
            (13, "关闭当前组"),
            (14, "关闭全部贴图"),
        ] {
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &NSString::from_str(title),
                    Some(sel!(menuAction:)),
                    &NSString::new(),
                )
            };
            item.setTag(tag);
            unsafe {
                item.setTarget(Some(self));
            }
            menu.addItem(&item);
        }
        NSMenu::popUpContextMenu_withEvent_forView(&menu, event, self);
    }
}

fn key(event: &NSEvent) -> Option<Key> {
    Some(match event.keyCode() {
        53 => Key::Escape,
        49 => Key::Space,
        36 => Key::Enter,
        48 => Key::Tab,
        51 => Key::Backspace,
        117 => Key::Delete,
        123 => Key::ArrowLeft,
        124 => Key::ArrowRight,
        125 => Key::ArrowDown,
        126 => Key::ArrowUp,
        _ => {
            let text = event.charactersIgnoringModifiers()?.to_string();
            let c = text.chars().next()?;
            if ('\u{f704}'..='\u{f71b}').contains(&c) {
                Key::Function((c as u32 - 0xf704 + 1) as u8)
            } else {
                Key::Character(c.to_ascii_lowercase())
            }
        }
    })
}
fn modifiers(event: &NSEvent) -> Modifiers {
    let flags = event.modifierFlags();
    Modifiers {
        control: flags.contains(NSEventModifierFlags::Control),
        meta: flags.contains(NSEventModifierFlags::Command),
        alt: flags.contains(NSEventModifierFlags::Option),
        shift: flags.contains(NSEventModifierFlags::Shift),
    }
}

fn input_modifiers(event: &NSEvent) -> NSEventModifierFlags {
    event.modifierFlags()
        & (NSEventModifierFlags::Command
            | NSEventModifierFlags::Control
            | NSEventModifierFlags::Option
            | NSEventModifierFlags::Shift)
}

thread_local! {
    static WINDOWS: RefCell<HashMap<String, (Retained<PinPanel>, Retained<PinView>)>> = RefCell::new(HashMap::new());
}

pub(super) fn open(
    image: PinnedImageView,
    rect: &LogicalRect,
    preferences: Preferences,
    dispatch: impl Fn(Action, PinnedImageExportSize) + 'static,
) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("Pinned images must open on the main thread")?;
    let id = image.id.clone();
    let state = ImageState::new(image, preferences.clone())?;
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(rect.width, rect.height));
    let window = PinPanel::alloc(mtm).set_ivars(());
    let window: Retained<PinPanel> = unsafe {
        msg_send![super(window), initWithContentRect: frame, styleMask: NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel, backing: NSBackingStoreType::Buffered, defer: false]
    };
    window.setTitle(&NSString::from_str("SnapLingo Pin"));
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setFloatingPanel(true);
    window.setHidesOnDeactivate(false);
    window.setCanHide(false);
    window.setHasShadow(preferences.shadow);
    window.setAcceptsMouseMovedEvents(true);
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    window.setLevel(NSFloatingWindowLevel);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    window.setExcludedFromWindowsMenu(true);
    let root = PinView::alloc(mtm).set_ivars(PinIvars {
        image: RefCell::new(state),
        dispatch: Box::new(dispatch),
        toolbar: OnceCell::new(),
        lens: OnceCell::new(),
        inspection: RefCell::new(None),
    });
    let root: Retained<PinView> = unsafe { msg_send![super(root), initWithFrame: frame] };
    let tracking = unsafe {
        NSTrackingArea::initWithRect_options_owner_userInfo(
            NSTrackingArea::alloc(),
            frame,
            NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::MouseMoved
                | NSTrackingAreaOptions::ActiveAlways
                | NSTrackingAreaOptions::InVisibleRect,
            Some(&root),
            None,
        )
    };
    root.addTrackingArea(&tracking);
    let toolbar = NSBox::new(mtm);
    let panel = root.ivars().image.borrow().toolbar_rect();
    toolbar.setFrame(NSRect::new(
        NSPoint::new(panel.x as f64, panel.y as f64),
        NSSize::new(panel.width as f64, panel.height as f64),
    ));
    toolbar.setBoxType(NSBoxType::Custom);
    toolbar.setTitlePosition(NSTitlePosition::NoTitle);
    toolbar.setFillColor(&NSColor::windowBackgroundColor());
    toolbar.setContentViewMargins(NSSize::new(0.0, 0.0));
    for (index, (_, rect, title)) in root
        .ivars()
        .image
        .borrow()
        .toolbar_layout()
        .into_iter()
        .enumerate()
    {
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(title),
                Some(&root),
                Some(sel!(toolbarAction:)),
                mtm,
            )
        };
        button.setTag(index as isize);
        button.setFont(Some(&NSFont::systemFontOfSize(11.0)));
        button.setFrame(NSRect::new(
            NSPoint::new((rect.x - panel.x) as f64, (rect.y - panel.y) as f64),
            NSSize::new(rect.width as f64, rect.height as f64),
        ));
        if let Some(content) = toolbar.contentView() {
            content.addSubview(&button);
        }
    }
    toolbar.setHidden(true);
    root.addSubview(&toolbar);
    root.ivars().toolbar.set(toolbar).unwrap();
    let lens = LensView::alloc(mtm).set_ivars(LensIvars {
        state: RefCell::new(None),
        label: OnceCell::new(),
    });
    let lens: Retained<LensView> = unsafe {
        msg_send![super(lens), initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(160.0, 112.0))]
    };
    let label = NSTextField::labelWithString(&NSString::new(), mtm);
    label.setFrame(NSRect::new(
        NSPoint::new(4.0, 90.0),
        NSSize::new(152.0, 20.0),
    ));
    label.setFont(Some(&NSFont::systemFontOfSize(11.0)));
    label.setTextColor(Some(&NSColor::whiteColor()));
    label.setBackgroundColor(Some(&NSColor::blackColor()));
    label.setDrawsBackground(true);
    lens.addSubview(&label);
    lens.ivars().label.set(label).unwrap();
    lens.setHidden(true);
    root.addSubview(&lens);
    assert!(root.ivars().lens.set(lens).is_ok());
    window.setContentView(Some(&root));
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
    window.makeKeyAndOrderFront(None);
    window.makeFirstResponder(Some(&root));
    WINDOWS.with(|windows| windows.borrow_mut().insert(id, (window, root)));
    Ok(())
}

pub(super) fn show(id: &str) -> Result<bool, String> {
    WINDOWS.with(|windows| {
        let windows = windows.borrow();
        let Some((window, root)) = windows.get(id) else {
            return Ok(false);
        };
        root.ivars().image.borrow_mut().restore()?;
        root.setNeedsDisplay(true);
        window.orderFront(None);
        Ok(true)
    })
}

pub(super) fn focus(id: &str) -> Result<bool, String> {
    WINDOWS.with(|windows| {
        let windows = windows.borrow();
        let Some((window, root)) = windows.get(id) else {
            return Ok(false);
        };
        root.ivars().image.borrow_mut().restore()?;
        root.setNeedsDisplay(true);
        window.makeKeyAndOrderFront(None);
        window.makeFirstResponder(Some(root));
        Ok(true)
    })
}

pub(super) fn hide(id: &str) {
    WINDOWS.with(|windows| {
        if let Some((window, root)) = windows.borrow().get(id) {
            window.orderOut(None);
            root.hide_lens();
            let mut image = root.ivars().image.borrow_mut();
            image.original = None;
            image.filtered = None;
            drop(image);
            root.ivars().image.borrow_mut().hovered = false;
            root.update_toolbar();
        }
    });
}

pub(super) fn close(id: &str) {
    WINDOWS.with(|windows| {
        if let Some((window, _)) = windows.borrow_mut().remove(id) {
            window.orderOut(None);
            window.close();
        }
    });
}

pub(super) fn ids() -> Vec<String> {
    WINDOWS.with(|windows| windows.borrow().keys().cloned().collect())
}

pub(super) fn visible(id: &str) -> bool {
    WINDOWS.with(|windows| {
        windows
            .borrow()
            .get(id)
            .is_some_and(|(window, _)| window.isVisible())
    })
}

pub(super) fn update_preferences(preferences: Preferences) {
    WINDOWS.with(|windows| {
        for (window, root) in windows.borrow().values() {
            let changed = root.ivars().image.borrow().preferences.border != preferences.border;
            root.ivars().image.borrow_mut().preferences = preferences.clone();
            window.setHasShadow(preferences.shadow);
            if changed {
                root.setNeedsDisplay(true);
            }
        }
    });
}

pub(super) fn replace(image: PinnedImageView) -> Result<(), String> {
    WINDOWS.with(|windows| {
        if let Some((window, root)) = windows.borrow().get(&image.id) {
            let opacity = root.ivars().image.borrow().presentation.opacity;
            let filters = {
                let state = root.ivars().image.borrow();
                (state.presentation.grayscale, state.presentation.inverted)
            };
            let mut state =
                ImageState::new(image, root.ivars().image.borrow().preferences.clone())?;
            state.presentation.opacity = opacity;
            state.presentation.grayscale = filters.0;
            state.presentation.inverted = filters.1;
            state.update_filter()?;
            *root.ivars().image.borrow_mut() = state;
            root.change(|_| {});
            if !window.isVisible() {
                let mut image = root.ivars().image.borrow_mut();
                image.original = None;
                image.filtered = None;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn test_presentation(id: &str) -> Presentation {
    WINDOWS.with(|windows| {
        windows
            .borrow()
            .get(id)
            .unwrap()
            .1
            .ivars()
            .image
            .borrow()
            .presentation
            .clone()
    })
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn test_key(id: &str, event: &NSEvent) {
    let root = WINDOWS.with(|windows| windows.borrow().get(id).unwrap().1.clone());
    root.handle_key(event);
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn test_inspection(id: &str, point: Option<NSPoint>) -> Option<[u8; 3]> {
    let root = WINDOWS.with(|windows| windows.borrow().get(id).unwrap().1.clone());
    if let Some(point) = point {
        root.ivars().image.borrow_mut().modifiers.alt = true;
        root.inspect(point);
        root.ivars()
            .lens
            .get()
            .unwrap()
            .ivars()
            .state
            .borrow()
            .as_ref()
            .map(|state| state.color)
    } else {
        root.ivars().image.borrow_mut().modifiers.alt = false;
        root.hide_lens();
        assert!(root.ivars().inspection.borrow().is_none());
        None
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn has_key_window() -> bool {
    WINDOWS.with(|windows| {
        windows
            .borrow()
            .values()
            .any(|(window, _)| window.isKeyWindow())
    })
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn test_frame(id: &str) -> NSRect {
    WINDOWS.with(|windows| windows.borrow().get(id).unwrap().0.frame())
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn test_toolbar_frames(id: &str) -> (NSRect, Vec<NSRect>) {
    WINDOWS.with(|windows| {
        let windows = windows.borrow();
        let toolbar = windows.get(id).unwrap().1.ivars().toolbar.get().unwrap();
        let buttons = toolbar
            .contentView()
            .unwrap()
            .subviews()
            .iter()
            .map(|button| button.frame())
            .collect();
        (toolbar.frame(), buttons)
    })
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn test_snapshot(id: &str) -> Vec<u8> {
    WINDOWS.with(|windows| {
        crate::infrastructure::system::native_view_snapshot::snapshot(
            &windows.borrow().get(id).unwrap().1,
        )
    })
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_decoded_images() -> usize {
    WINDOWS.with(|windows| {
        windows
            .borrow()
            .values()
            .filter(|(_, root)| root.ivars().image.borrow().original.is_some())
            .count()
    })
}

#[cfg(test)]
pub(crate) fn test_ids() -> Vec<String> {
    ids()
}
