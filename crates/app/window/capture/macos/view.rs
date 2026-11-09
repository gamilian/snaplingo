use objc2_app_kit::NSUserInterfaceItemIdentification;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplicationActivationOptions, NSBackingStoreType, NSBezierPath, NSBitmapFormat,
    NSBitmapImageRep, NSColor, NSCompositingOperation, NSCursor, NSEvent, NSEventModifierFlags,
    NSFont, NSGraphicsContext, NSImage, NSImageCacheMode, NSImageInterpolation, NSImageScaling,
    NSImageView, NSPanel, NSPrintInfo, NSPrintOperation, NSPrintingPaginationMode,
    NSRunningApplication, NSScreen, NSScreenSaverWindowLevel, NSTextAlignment, NSTextField,
    NSTrackingArea, NSTrackingAreaOptions, NSView, NSWindow, NSWindowAnimationBehavior,
    NSWindowCollectionBehavior, NSWindowDelegate, NSWindowSharingType, NSWindowStyleMask,
};
use objc2_foundation::{
    NSCopying, NSData, NSNotification, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use crate::application::capture::{
    contains, restore_selection, CaptureEditor, CaptureEditorControlAction, CaptureSelectionMode,
    CaptureSession, CaptureWindowGeometry, Selection,
};
use crate::domain::capture::{CapturedCursor, LogicalPoint, LogicalRect, MonitorSnapshot};
use crate::domain::config::ScreenshotSettings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Edit,
    Primary,
    Copy,
    Save,
    QuickSave,
    Pin,
    Print,
}

#[derive(Clone, Debug)]
pub(crate) enum Action {
    Complete(Operation, LogicalRect, bool),
    Cancel,
    Refresh,
    Inspect(LogicalPoint, u64),
    MoveCursor(i32, i32),
    CopyText(String),
    FocusLost,
    Editor(super::canvas::Request),
}

struct Raster {
    bounds: LogicalRect,
    scale: f64,
    image: Retained<NSImage>,
    bitmap: Retained<NSBitmapImageRep>,
}
impl Raster {
    fn new(snapshot: MonitorSnapshot) -> Result<Self, String> {
        let bitmap = NSBitmapImageRep::imageRepWithData(&NSData::with_bytes(&snapshot.png_data))
            .ok_or("Could not decode frozen capture image")?;
        let size = NSSize::new(bitmap.pixelsWide() as f64, bitmap.pixelsHigh() as f64);
        bitmap.setSize(size);
        let image = NSImage::initWithSize(NSImage::alloc(), size);
        image.addRepresentation(&bitmap);
        image.setCacheMode(NSImageCacheMode::Never);
        Ok(Self {
            bounds: snapshot.logical_bounds,
            scale: snapshot.scale_factor,
            image,
            bitmap,
        })
    }
    fn sample(&self, point: &LogicalPoint) -> Option<([u8; 3], NSPoint)> {
        if !contains(&self.bounds, point) {
            return None;
        }
        let width = self.bitmap.pixelsWide();
        let height = self.bitmap.pixelsHigh();
        let x = (((point.x - self.bounds.x) / self.bounds.width * width as f64).floor() as isize)
            .clamp(0, width - 1);
        let y = (((point.y - self.bounds.y) / self.bounds.height * height as f64).floor() as isize)
            .clamp(0, height - 1);
        let mut channels = [0usize; 4];
        // Capture sources encode RGBA8 PNGs. Read their original channels;
        // colorAtX:y: returns a calibrated NSColor and changes copied RGB values.
        unsafe {
            self.bitmap
                .getPixel_atX_y(std::ptr::NonNull::from(&mut channels[0]), x, y);
        }
        let offset = usize::from(
            self.bitmap
                .bitmapFormat()
                .contains(NSBitmapFormat::AlphaFirst),
        );
        Some((
            [
                channels[offset] as u8,
                channels[offset + 1] as u8,
                channels[offset + 2] as u8,
            ],
            NSPoint::new(x as f64, y as f64),
        ))
    }
}

struct CursorImage {
    image: Retained<NSImage>,
    bounds: LogicalRect,
}
impl CursorImage {
    fn new(cursor: CapturedCursor) -> Option<Self> {
        let image = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(&cursor.png_data))?;
        let scale = cursor.scale_factor.max(1.0);
        Some(Self {
            image,
            bounds: LogicalRect {
                x: cursor.logical_position.x - cursor.hotspot.x,
                y: cursor.logical_position.y - cursor.hotspot.y,
                width: cursor.image_width as f64 / scale,
                height: cursor.image_height as f64 / scale,
            },
        })
    }
}

define_class!(
    #[unsafe(super(NSPanel))]
    #[name = "SnapLingoQuickCapturePanel"]
    #[thread_kind = MainThreadOnly]
    struct CapturePanel;
    unsafe impl NSObjectProtocol for CapturePanel {}
    impl CapturePanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool { true }
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool { false }
    }
);

struct OverlayIvars {
    selection: Rc<RefCell<Selection>>,
    preferences: ScreenshotSettings,
    pixel_scale: Option<f64>,
    monitors: Vec<(LogicalRect, f64)>,
    ui_scale: f64,
    previous_rect: RefCell<Option<LogicalRect>>,
    previous_tinted: Cell<bool>,
    label: OnceCell<Retained<NSTextField>>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoQuickCaptureOverlay"]
    #[thread_kind = MainThreadOnly]
    #[ivars = OverlayIvars]
    struct OverlayView;
    unsafe impl NSObjectProtocol for OverlayView {}
    impl OverlayView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView { std::ptr::null_mut() }
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let state = self.ivars();
            let selection = state.selection.borrow();
            let Some(selected) = selection.active_rect() else { return; };
            let local = local_rect(selected, &selection.bounds);
            let bounds = self.bounds();
            let left = local.origin.x.clamp(0.0, bounds.size.width);
            let top = local.origin.y.clamp(0.0, bounds.size.height);
            let right = (local.origin.x + local.size.width).clamp(left, bounds.size.width);
            let bottom = (local.origin.y + local.size.height).clamp(top, bounds.size.height);
            rgba(state.preferences.selection_mask_color).setFill();
            for mask in [
                rect(0.0, 0.0, bounds.size.width, top),
                rect(0.0, bottom, bounds.size.width, bounds.size.height - bottom),
                rect(0.0, top, left, bottom - top),
                rect(right, top, bounds.size.width - right, bottom - top),
            ] { NSBezierPath::bezierPathWithRect(mask).fill(); }
            if selection.anchor.is_some() || selection.rect.is_none() {
                rgba([255, 255, 255, 13]).setFill();
                NSBezierPath::bezierPathWithRect(local).fill();
            }
            rgba(state.preferences.selection_border_color).setStroke();
            let outline = NSBezierPath::bezierPathWithRect(rect(local.origin.x + 0.5, local.origin.y + 0.5, local.size.width - 1.0, local.size.height - 1.0));
            outline.setLineWidth(state.preferences.selection_border_width.clamp(1, 8) as f64);
            outline.stroke();
        }
    }
);
impl OverlayView {
    fn refresh(&self) {
        let state = self.ivars();
        let selection = state.selection.borrow();
        let selected = selection.active_rect().cloned();
        let tinted = selection.anchor.is_some() || selection.rect.is_none();
        let tint_changed = state.previous_tinted.replace(tinted) != tinted;
        if *state.previous_rect.borrow() != selected || tint_changed {
            *state.previous_rect.borrow_mut() = selected.clone();
            self.setNeedsDisplay(true);
        }
        let Some(label) = state.label.get() else {
            return;
        };
        label.setHidden(selected.is_none() || !state.preferences.show_selection_size);
        let Some(selected) = selected else {
            return;
        };
        let scale = state.pixel_scale.unwrap_or_else(|| {
            state
                .monitors
                .iter()
                .filter(|(monitor, _)| intersects(&selected, monitor))
                .map(|(_, scale)| *scale)
                .fold(1.0, f64::max)
        });
        let text = format!(
            "{} x {} px",
            (selected.width * scale).ceil() as u32,
            (selected.height * scale).ceil() as u32
        );
        label.setStringValue(&NSString::from_str(&text));
        label.sizeToFit();
        let size = label.frame().size;
        let height = 20.0 * state.ui_scale;
        let width = size.width + 16.0 * state.ui_scale;
        let bounds = self.bounds();
        let local = local_rect(&selected, &selection.bounds);
        let above = local.origin.y - height - 6.0 * state.ui_scale;
        label.setFrame(rect(
            local
                .origin
                .x
                .clamp(0.0, (bounds.size.width - width).max(0.0))
                .round(),
            (if above >= 0.0 { above } else { local.origin.y })
                .clamp(0.0, (bounds.size.height - height).max(0.0))
                .round(),
            width,
            height,
        ));
    }
}

struct LensState {
    image: Retained<NSImage>,
    source: NSRect,
    destination: NSRect,
}
struct LensIvars {
    state: RefCell<Option<LensState>>,
    ui_scale: f64,
    label: OnceCell<Retained<NSTextField>>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoQuickCaptureLens"]
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
            rgba([22, 25, 32, 215]).setFill();
            NSBezierPath::bezierPathWithRect(self.bounds()).fill();
            let state = self.ivars().state.borrow();
            let Some(state) = state.as_ref() else { return; };
            NSGraphicsContext::saveGraphicsState_class();
            if let Some(context) = NSGraphicsContext::currentContext() { context.setImageInterpolation(NSImageInterpolation::None); }
            unsafe { state.image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                state.destination, state.source, NSCompositingOperation::SourceOver, 1.0, true, None,
            ); }
            NSGraphicsContext::restoreGraphicsState_class();
            let scale = self.ivars().ui_scale;
            rgba([96, 165, 250, 180]).setStroke();
            let lines = NSBezierPath::bezierPath();
            lines.moveToPoint(NSPoint::new(114.0 * scale, 0.0));
            lines.lineToPoint(NSPoint::new(114.0 * scale, 132.0 * scale));
            lines.moveToPoint(NSPoint::new(0.0, 66.0 * scale));
            lines.lineToPoint(NSPoint::new(228.0 * scale, 66.0 * scale));
            lines.setLineWidth(1.0 * scale);
            lines.stroke();
            NSColor::whiteColor().setStroke();
            let center = NSBezierPath::bezierPathWithRect(rect(108.0 * scale, 60.0 * scale, 12.0 * scale, 12.0 * scale));
            center.setLineWidth(2.0 * scale);
            center.stroke();
            rgba([255, 255, 255, 90]).setStroke();
            let border = NSBezierPath::bezierPathWithRect(self.bounds());
            border.setLineWidth(1.0);
            border.stroke();
        }
    }
);

struct CaptureIvars {
    mode: CaptureSelectionMode,
    selection: Rc<RefCell<Selection>>,
    rasters: Vec<Raster>,
    cursor: Option<CursorImage>,
    include_cursor: Cell<bool>,
    preferences: ScreenshotSettings,
    ui_scale: f64,
    history: Vec<LogicalRect>,
    dispatch: Box<dyn Fn(Action)>,
    overlay: OnceCell<Retained<OverlayView>>,
    lens: OnceCell<Retained<LensView>>,
    hint: OnceCell<Retained<NSTextField>>,
    busy: Cell<bool>,
    opening_editor: Cell<bool>,
    suspended: Cell<bool>,
    updating: Cell<bool>,
    magnifier: Cell<bool>,
    shift: Cell<bool>,
    rgb: Cell<bool>,
    editor: RefCell<Option<Retained<super::canvas::EditorView>>>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoQuickCaptureView"]
    #[thread_kind = MainThreadOnly]
    #[ivars = CaptureIvars]
    struct CaptureView;
    unsafe impl NSObjectProtocol for CaptureView {}
    unsafe impl NSWindowDelegate for CaptureView {
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, _window: &NSWindow) -> bool { self.cancel(); false }
        #[unsafe(method(windowDidResignKey:))]
        fn resigned_key(&self, _notification: &NSNotification) {
            if !self.ivars().updating.get() && !self.ivars().busy.get() && !self.ivars().suspended.get() {
                if self.ivars().editor.borrow().is_some() { (self.ivars().dispatch)(Action::FocusLost); }
                else { self.cancel(); }
            }
        }
    }
    impl CaptureView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(isOpaque))]
        fn is_opaque(&self) -> bool { true }
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool { true }
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool { true }
        #[unsafe(method(resetCursorRects))]
        fn reset_cursor_rects(&self) { self.addCursorRect_cursor(self.bounds(), &NSCursor::crosshairCursor()); }
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            NSColor::blackColor().setFill();
            NSBezierPath::bezierPathWithRect(self.bounds()).fill();
            let selection = self.ivars().selection.borrow();
            for raster in &self.ivars().rasters {
                unsafe { raster.image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                    local_rect(&raster.bounds, &selection.bounds), rect(0.0, 0.0, raster.image.size().width, raster.image.size().height),
                    NSCompositingOperation::Copy, 1.0, true, None,
                ); }
            }
            if self.ivars().include_cursor.get() {
                if let Some(cursor) = &self.ivars().cursor {
                    unsafe { cursor.image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                        local_rect(&cursor.bounds, &selection.bounds), rect(0.0, 0.0, cursor.image.size().width, cursor.image.size().height),
                        NSCompositingOperation::SourceOver, 1.0, true, None,
                    ); }
                }
            }
        }
        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) { self.move_pointer(event); }
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, event: &NSEvent) { self.move_pointer(event); }
        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            if let Some(lens) = self.ivars().lens.get() { lens.setHidden(true); }
        }
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            if self.ivars().editor.borrow().is_some() {
                self.selection_input(self.desktop_point(event), "down", false);
                return;
            }
            if self.ivars().busy.get() { return; }
            let point = self.desktop_point(event);
            self.ivars().selection.borrow_mut().press(point, input_modifiers(event).is_empty());
            self.refresh();
        }
        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            if self.ivars().editor.borrow().is_some() {
                self.selection_input(self.desktop_point(event), "move", event.modifierFlags().contains(NSEventModifierFlags::Shift));
            } else { self.move_pointer(event); }
        }
        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            if self.ivars().editor.borrow().is_some() {
                self.selection_input(self.desktop_point(event), "up", event.modifierFlags().contains(NSEventModifierFlags::Shift));
                return;
            }
            if self.ivars().busy.get() { return; }
            let point = self.desktop_point(event);
            let completed = self.ivars().selection.borrow_mut().release(point, event.modifierFlags().contains(NSEventModifierFlags::Shift));
            self.refresh();
            if let Some(rect) = completed { self.complete_rect(self.selection_operation(), rect); }
        }
        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, _event: &NSEvent) {
            if editor_cancel_selection(&session_id().unwrap_or_default()) { return; }
            if let Some(editor) = self.ivars().editor.borrow().as_ref() {
                unsafe { let _: () = msg_send![editor, rightMouseDown: _event]; }
                return;
            }
            if self.ivars().busy.get() { return; }
            let has_selection = self.ivars().selection.borrow().rect.is_some();
            if has_selection { self.ivars().selection.borrow_mut().reset(); self.refresh(); }
            else { self.cancel(); }
        }
        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) { self.handle_key(event); }
        #[unsafe(method(performKeyEquivalent:))]
        fn perform_key_equivalent(&self, event: &NSEvent) -> bool {
            if event.keyCode() == 53 && self.ivars().selection.borrow().editing() {
                self.selection_input(self.desktop_point(event), "cancel", false);
                if let Some(editor) = self.ivars().editor.borrow().as_ref() { editor.cancel_selection_tracking(); }
                return true.into();
            }
            if let Some(editor) = self.ivars().editor.borrow().as_ref() { return editor.key(event).into(); }
            let flags = input_modifiers(event);
            let key = event.charactersIgnoringModifiers().map(|key| key.to_string().to_lowercase()).unwrap_or_default();
            if flags.intersects(NSEventModifierFlags::Command | NSEventModifierFlags::Control)
                && !flags.contains(NSEventModifierFlags::Option)
                && matches!(key.as_str(), "c" | "s" | "t" | "p" | "a") {
                self.handle_key(event);
                true
            } else { false }
        }
        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &NSEvent) {
            if self.ivars().busy.get() { return; }
            let flags = input_modifiers(event);
            let shift = flags.contains(NSEventModifierFlags::Shift);
            if shift && !self.ivars().shift.get() && self.lens_shown()
                && !flags.intersects(NSEventModifierFlags::Command | NSEventModifierFlags::Control) {
                self.ivars().rgb.set(!self.ivars().rgb.get());
            }
            self.ivars().shift.set(shift);
            self.ivars().magnifier.set(flags.contains(NSEventModifierFlags::Option)
                && !flags.intersects(NSEventModifierFlags::Command | NSEventModifierFlags::Control));
            if self.ivars().selection.borrow().anchor.is_some() {
                let point = self.ivars().selection.borrow().point.clone();
                if let Some(point) = point { self.ivars().selection.borrow_mut().move_to(point, shift); }
            }
            self.refresh();
        }
        #[unsafe(method(cancelOperation:))]
        fn cancel_operation(&self, _sender: Option<&objc2::runtime::AnyObject>) { self.cancel(); }
    }
);

impl CaptureView {
    fn selection_input(&self, point: LogicalPoint, phase: &str, constrained: bool) {
        if self.ivars().busy.get() {
            return;
        }
        {
            let mut selection = self.ivars().selection.borrow_mut();
            match phase {
                "down" => {
                    selection.begin_edit(point, true);
                }
                "move" => {
                    selection.update_edit(point, constrained);
                }
                "up" => {
                    selection.update_edit(point, constrained);
                    selection.finish_edit();
                }
                "cancel" => {
                    selection.cancel_edit();
                }
                _ => return,
            }
        }
        self.refresh_editor_selection();
    }
    fn refresh_editor_selection(&self) {
        let selection = self.ivars().selection.borrow();
        let Some(rect) = selection.rect.clone() else {
            return;
        };
        let frame = local_rect(&rect, &selection.bounds);
        drop(selection);
        let editor = self.ivars().editor.borrow().clone();
        if let Some(editor) = editor {
            editor.selection_changed(rect, frame);
        }
        self.refresh();
    }
    fn desktop_point(&self, event: &NSEvent) -> LogicalPoint {
        let local = self.convertPoint_fromView(event.locationInWindow(), None);
        let selection = self.ivars().selection.borrow();
        LogicalPoint {
            x: local.x + selection.bounds.x,
            y: local.y + selection.bounds.y,
        }
    }
    fn move_pointer(&self, event: &NSEvent) {
        if self.ivars().busy.get() {
            return;
        }
        let point = self.desktop_point(event);
        let changed = self.ivars().selection.borrow_mut().move_to(
            point,
            event.modifierFlags().contains(NSEventModifierFlags::Shift),
        );
        if changed {
            self.refresh();
            self.inspect_control();
        }
    }
    fn inspect_control(&self) {
        let state = self.ivars().selection.borrow();
        if state.control_detection && state.anchor.is_none() {
            if let Some(point) = &state.point {
                (self.ivars().dispatch)(Action::Inspect(point.clone(), state.revision));
            }
        }
    }
    fn refresh(&self) {
        if let Some(overlay) = self.ivars().overlay.get() {
            overlay.refresh();
        }
        self.refresh_lens();
    }
    fn lens_shown(&self) -> bool {
        self.ivars().lens.get().is_some_and(|lens| !lens.isHidden())
    }
    fn sampled_color(&self) -> Option<[u8; 3]> {
        let state = self.ivars().selection.borrow();
        let point = state.point.as_ref()?;
        self.ivars()
            .rasters
            .iter()
            .find_map(|raster| raster.sample(point).map(|(color, _)| color))
    }
    fn refresh_lens(&self) {
        let state = self.ivars();
        let Some(lens) = state.lens.get() else {
            return;
        };
        if state.busy.get()
            || state.editor.borrow().is_some()
            || !(state.preferences.show_magnifier || state.magnifier.get())
        {
            lens.setHidden(true);
            return;
        }
        let selection = state.selection.borrow();
        let Some(point) = selection.point.as_ref() else {
            lens.setHidden(true);
            return;
        };
        let sample = state
            .rasters
            .iter()
            .find_map(|raster| raster.sample(point).map(|sample| (raster, sample)));
        let Some((raster, (color, pixel))) = sample else {
            lens.setHidden(true);
            return;
        };
        let scale = state.ui_scale;
        let zoom = state.preferences.magnifier_zoom.clamp(4, 20) as f64;
        let width = raster.image.size().width;
        let height = raster.image.size().height;
        let sample_width = (228.0 / zoom).round().max(1.0).min(width);
        let sample_height = (132.0 / zoom).round().max(1.0).min(height);
        let desired_x = pixel.x - (sample_width / 2.0).floor();
        let desired_y = pixel.y - (sample_height / 2.0).floor();
        let left = desired_x.max(0.0);
        let top = desired_y.max(0.0);
        let right = (desired_x + sample_width).min(width);
        let bottom = (desired_y + sample_height).min(height);
        *lens.ivars().state.borrow_mut() = Some(LensState {
            image: raster.image.clone(),
            source: rect(left, height - bottom, right - left, bottom - top),
            destination: rect(
                (left - desired_x) * zoom * scale,
                (top - desired_y) * zoom * scale,
                (right - left) * zoom * scale,
                (bottom - top) * zoom * scale,
            ),
        });
        let local = NSPoint::new(point.x - selection.bounds.x, point.y - selection.bounds.y);
        let size = lens.frame().size;
        let bounds = self.bounds();
        let gap = 14.0 * scale;
        let x = if local.x + gap + size.width <= bounds.size.width {
            local.x + gap
        } else {
            local.x - size.width - gap
        };
        let y = if local.y + gap + size.height <= bounds.size.height {
            local.y + gap
        } else {
            local.y - size.height - gap
        };
        lens.setFrameOrigin(NSPoint::new(
            x.clamp(0.0, (bounds.size.width - size.width).max(0.0)),
            y.clamp(0.0, (bounds.size.height - size.height).max(0.0)),
        ));
        if let Some(label) = lens.ivars().label.get() {
            label.setStringValue(&NSString::from_str(&format!(
                "({:.0}, {:.0})\n{}\n按 C 复制颜色值\n按 Shift 切换 RGB / HEX",
                point.x,
                point.y,
                color_text(color, state.rgb.get())
            )));
        }
        lens.setHidden(false);
        lens.setNeedsDisplay(true);
    }
    fn complete_rect(&self, operation: Operation, rect: LogicalRect) {
        if self.ivars().busy.replace(true) {
            return;
        }
        self.ivars()
            .opening_editor
            .set(operation == Operation::Edit);
        self.set_hint(if operation == Operation::Edit {
            "正在打开标注编辑器… · Esc 取消"
        } else {
            "正在处理选区…"
        });
        self.refresh_lens();
        (self.ivars().dispatch)(Action::Complete(
            operation,
            rect,
            self.ivars().include_cursor.get(),
        ));
    }
    fn complete(&self, operation: Operation) {
        let state = self.ivars().selection.borrow();
        if state.anchor.is_some() {
            return;
        }
        let rect = state.active_rect().cloned();
        drop(state);
        if let Some(rect) = rect {
            self.complete_rect(operation, rect);
        }
    }
    fn cancel(&self) {
        if self.ivars().opening_editor.replace(false) || !self.ivars().busy.replace(true) {
            (self.ivars().dispatch)(Action::Cancel);
        }
    }
    fn selection_operation(&self) -> Operation {
        if self.ivars().mode == CaptureSelectionMode::Edit {
            Operation::Edit
        } else {
            Operation::Primary
        }
    }
    fn set_hint(&self, text: &str) {
        let Some(hint) = self.ivars().hint.get() else {
            return;
        };
        hint.setStringValue(&NSString::from_str(text));
        let bounds = self.bounds();
        let scale = self.ivars().ui_scale;
        let maximum_width = (bounds.size.width - 32.0 * scale).max(0.0);
        let size =
            hint.cell()
                .unwrap()
                .cellSizeForBounds(rect(0.0, 0.0, maximum_width, 64.0 * scale));
        let width = size.width.min(maximum_width);
        let height = size.height.min(64.0 * scale);
        hint.setFrame(rect(
            ((bounds.size.width - width) / 2.0).max(0.0),
            bounds.size.height - height - 20.0 * scale,
            width,
            height,
        ));
    }
    fn handle_key(&self, event: &NSEvent) {
        if event.keyCode() == 53 && self.ivars().selection.borrow().editing() {
            self.selection_input(self.desktop_point(event), "cancel", false);
            return;
        }
        if let Some(editor) = self.ivars().editor.borrow().as_ref() {
            editor.key(event);
            return;
        }
        if self.ivars().busy.get() {
            if self.ivars().opening_editor.get() && event.keyCode() == 53 {
                self.cancel();
            }
            return;
        }
        let flags = input_modifiers(event);
        let command =
            flags.intersects(NSEventModifierFlags::Command | NSEventModifierFlags::Control);
        let shift = flags.contains(NSEventModifierFlags::Shift);
        let alt = flags.contains(NSEventModifierFlags::Option);
        let key = event
            .charactersIgnoringModifiers()
            .map(|key| key.to_string().to_lowercase())
            .unwrap_or_default();
        if event.keyCode() == 53 {
            self.cancel();
            return;
        }
        if event.keyCode() == 96 && flags.is_empty() {
            self.ivars().busy.set(true);
            (self.ivars().dispatch)(Action::Refresh);
            return;
        }
        if command && !alt {
            match (key.as_str(), shift) {
                ("c", false) => self.complete(Operation::Copy),
                ("s", false) => self.complete(Operation::Save),
                ("s", true) => self.complete(Operation::QuickSave),
                ("t", false) => self.complete(Operation::Pin),
                ("p", false) => self.complete(Operation::Print),
                ("a", _) => {
                    let selection = self.ivars().selection.borrow();
                    let rect = if shift {
                        selection.bounds.clone()
                    } else {
                        selection
                            .point
                            .as_ref()
                            .and_then(|point| {
                                self.ivars()
                                    .rasters
                                    .iter()
                                    .find(|raster| contains(&raster.bounds, point))
                            })
                            .map(|raster| raster.bounds.clone())
                            .unwrap_or_else(|| selection.bounds.clone())
                    };
                    drop(selection);
                    self.complete_rect(self.selection_operation(), rect);
                }
                _ => {}
            }
            return;
        }
        if !command && key == "c" && self.lens_shown() {
            if let Some(color) = self.sampled_color() {
                (self.ivars().dispatch)(Action::CopyText(color_text(
                    color,
                    self.ivars().rgb.get(),
                )));
            }
            return;
        }
        if !command
            && !alt
            && self.ivars().cursor.is_some()
            && ((!shift && key == "`")
                || (shift
                    && event
                        .characters()
                        .is_some_and(|text| text.to_string() == "!")))
        {
            self.ivars()
                .include_cursor
                .set(!self.ivars().include_cursor.get());
            self.setNeedsDisplay(true);
            return;
        }
        if !flags.is_empty() {
            return;
        }
        match event.keyCode() {
            36 | 76 => self.complete(if self.ivars().mode == CaptureSelectionMode::Edit {
                Operation::Copy
            } else {
                Operation::Primary
            }),
            99 => self.complete(Operation::Pin),
            48 => {
                self.ivars().selection.borrow_mut().toggle_detection();
                self.refresh();
                self.inspect_control();
            }
            _ => match key.as_str() {
                "r" | "," | "." => {
                    let selection = self.ivars().selection.borrow();
                    let history = &self.ivars().history;
                    let index = if key == "r" {
                        Some(0)
                    } else {
                        selection
                            .rect
                            .as_ref()
                            .and_then(|rect| history.iter().position(|entry| entry == rect))
                            .map(|index| {
                                if key == "," {
                                    (index + 1) % history.len()
                                } else {
                                    (index + history.len() - 1) % history.len()
                                }
                            })
                            .or(Some(0))
                    };
                    let rect = index
                        .and_then(|index| history.get(index))
                        .and_then(|rect| restore_selection(rect, &selection.bounds));
                    drop(selection);
                    if let Some(rect) = rect {
                        self.complete_rect(self.selection_operation(), rect);
                    }
                }
                "w" | "a" | "s" | "d" => {
                    let (dx, dy) = match key.as_str() {
                        "w" => (0.0, -1.0),
                        "a" => (-1.0, 0.0),
                        "s" => (0.0, 1.0),
                        _ => (1.0, 0.0),
                    };
                    let delta = self.ivars().selection.borrow_mut().nudge_cursor(dx, dy);
                    self.refresh();
                    if let Some(delta) = delta {
                        (self.ivars().dispatch)(Action::MoveCursor(
                            delta.x.round() as i32,
                            delta.y.round() as i32,
                        ));
                        self.inspect_control();
                    }
                }
                _ => {}
            },
        }
    }
}

fn color_text(color: [u8; 3], rgb: bool) -> String {
    if rgb {
        format!("rgb({}, {}, {})", color[0], color[1], color[2])
    } else {
        format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
    }
}
fn rgba(value: [u8; 4]) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(
        value[0] as f64 / 255.0,
        value[1] as f64 / 255.0,
        value[2] as f64 / 255.0,
        value[3] as f64 / 255.0,
    )
}
fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(x, y),
        NSSize::new(width.max(0.0), height.max(0.0)),
    )
}
fn local_rect(value: &LogicalRect, bounds: &LogicalRect) -> NSRect {
    rect(
        value.x - bounds.x,
        value.y - bounds.y,
        value.width,
        value.height,
    )
}
fn intersects(a: &LogicalRect, b: &LogicalRect) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}
fn input_modifiers(event: &NSEvent) -> NSEventModifierFlags {
    event.modifierFlags()
        & (NSEventModifierFlags::Command
            | NSEventModifierFlags::Control
            | NSEventModifierFlags::Option
            | NSEventModifierFlags::Shift)
}
fn text_label(parent: &NSView, font: f64) -> Retained<NSTextField> {
    let label = NSTextField::wrappingLabelWithString(&NSString::new(), parent.mtm());
    label.setFont(Some(&NSFont::systemFontOfSize(font)));
    label.setTextColor(Some(&NSColor::whiteColor()));
    label.setAlignment(NSTextAlignment::Center);
    label.setDrawsBackground(true);
    label.setBackgroundColor(Some(&rgba([0, 0, 0, 210])));
    parent.addSubview(&label);
    label
}

thread_local! {
    static WINDOW: RefCell<Option<(String, Retained<CapturePanel>, Retained<CaptureView>)>> = const { RefCell::new(None) };
}

pub(crate) fn open(
    session: CaptureSession,
    mode: CaptureSelectionMode,
    geometry: CaptureWindowGeometry,
    preferences: ScreenshotSettings,
    history: Vec<LogicalRect>,
    dispatch: impl Fn(Action) + 'static,
) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("Native captures must open on the main thread")?;
    let bounds = geometry.bounds;
    let monitors = session
        .snapshots
        .into_iter()
        .filter(|monitor| {
            session
                .selection_monitor_id
                .as_ref()
                .is_none_or(|id| id == &monitor.id)
        })
        .map(Raster::new)
        .collect::<Result<Vec<_>, _>>()?;
    if monitors.is_empty() {
        return Err("Capture session has no frozen image for this display".into());
    }
    let ui_scale =
        crate::application::capture::interface_scale(&preferences.capture_ui_scale, Some(&bounds));
    let selection = Rc::new(RefCell::new(Selection::new(
        bounds.clone(),
        monitors
            .iter()
            .map(|raster| raster.bounds.clone())
            .collect(),
        session.candidates,
    )));
    let cursor_point = session
        .captured_cursor
        .as_ref()
        .map(|cursor| cursor.logical_position.clone());
    let cursor = session.captured_cursor.and_then(CursorImage::new);
    let frame = rect(0.0, 0.0, bounds.width, bounds.height);
    let window = CapturePanel::alloc(mtm).set_ivars(());
    let window: Retained<CapturePanel> = unsafe {
        msg_send![super(window), initWithContentRect: frame, styleMask: NSWindowStyleMask::Borderless, backing: NSBackingStoreType::Buffered, defer: false]
    };
    window.setTitle(&NSString::from_str("SnapLingo Capture"));
    window.setIdentifier(Some(&NSString::from_str(
        super::backend::CAPTURE_WINDOW_LABEL,
    )));
    window.setOpaque(true);
    window.setHasShadow(false);
    window.setHidesOnDeactivate(false);
    window.setCanHide(false);
    window.setAcceptsMouseMovedEvents(true);
    window.setAnimationBehavior(NSWindowAnimationBehavior::None);
    window.setSharingType(NSWindowSharingType::ReadOnly);
    window.setLevel(NSScreenSaverWindowLevel);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    let overlay_monitors = monitors
        .iter()
        .map(|raster| (raster.bounds.clone(), raster.scale))
        .collect();
    let root = CaptureView::alloc(mtm).set_ivars(CaptureIvars {
        mode,
        selection: selection.clone(),
        rasters: monitors,
        cursor,
        include_cursor: Cell::new(false),
        preferences: preferences.clone(),
        ui_scale,
        history,
        dispatch: Box::new(dispatch),
        overlay: OnceCell::new(),
        lens: OnceCell::new(),
        hint: OnceCell::new(),
        busy: Cell::new(false),
        opening_editor: Cell::new(false),
        suspended: Cell::new(false),
        updating: Cell::new(true),
        magnifier: Cell::new(false),
        shift: Cell::new(false),
        rgb: Cell::new(false),
        editor: RefCell::new(None),
    });
    let root: Retained<CaptureView> = unsafe { msg_send![super(root), initWithFrame: frame] };
    // The frozen desktop is a separate cached layer. Pointer input redraws only the overlay/lens.
    root.setWantsLayer(true);
    let overlay = OverlayView::alloc(mtm).set_ivars(OverlayIvars {
        selection,
        preferences,
        pixel_scale: geometry.desktop_scale,
        monitors: overlay_monitors,
        ui_scale,
        previous_rect: RefCell::new(None),
        previous_tinted: Cell::new(false),
        label: OnceCell::new(),
    });
    let overlay: Retained<OverlayView> = unsafe { msg_send![super(overlay), initWithFrame: frame] };
    overlay.setWantsLayer(true);
    let size_label = text_label(&overlay, 12.0 * ui_scale);
    overlay.ivars().label.set(size_label).unwrap();
    root.addSubview(&overlay);
    assert!(root.ivars().overlay.set(overlay).is_ok());
    let lens = LensView::alloc(mtm).set_ivars(LensIvars {
        state: RefCell::new(None),
        ui_scale,
        label: OnceCell::new(),
    });
    let lens: Retained<LensView> = unsafe {
        msg_send![super(lens), initWithFrame: rect(0.0, 0.0, 228.0 * ui_scale, 226.0 * ui_scale)]
    };
    let label = text_label(&lens, 12.0 * ui_scale);
    label.setDrawsBackground(false);
    label.setFrame(rect(
        4.0 * ui_scale,
        140.0 * ui_scale,
        220.0 * ui_scale,
        82.0 * ui_scale,
    ));
    lens.ivars().label.set(label).unwrap();
    lens.setWantsLayer(true);
    lens.setHidden(true);
    root.addSubview(&lens);
    assert!(root.ivars().lens.set(lens).is_ok());
    let hint = text_label(root.ivars().overlay.get().unwrap(), 12.0 * ui_scale);
    root.ivars().hint.set(hint).unwrap();
    let primary = match mode {
        CaptureSelectionMode::Edit => "复制截图",
        CaptureSelectionMode::Copy => "复制截图",
        CaptureSelectionMode::Ocr => "识别文字",
        CaptureSelectionMode::SilentOcr => "复制文字",
        CaptureSelectionMode::Translate => "截图翻译",
    };
    root.set_hint(&format!(
        "拖动选区或点击窗口 · Enter {primary} · Tab 界面元素 · Alt 放大镜 · Esc 取消"
    ));
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
    window.setContentView(Some(&root));
    window.setDelegate(Some(ProtocolObject::from_ref(&*root)));
    let height = NSScreen::screens(mtm)
        .firstObject()
        .ok_or("Capture window has no primary display")?
        .frame()
        .size
        .height;
    window.setFrame_display(
        rect(
            bounds.x,
            height - bounds.y - bounds.height,
            bounds.width,
            bounds.height,
        ),
        false,
    );
    close();
    WINDOW.with_borrow_mut(|current| *current = Some((session.id.0, window.clone(), root.clone())));
    super::presentation::suppress_capture_window_activation()?;
    NSRunningApplication::currentApplication()
        .activateWithOptions(NSApplicationActivationOptions::empty());
    window.makeKeyAndOrderFront(None);
    window.makeFirstResponder(Some(&root));
    super::presentation::push_native_crosshair_cursor();
    let native_point = window.convertPointFromScreen(NSEvent::mouseLocation());
    let local = root.convertPoint_fromView(native_point, None);
    let point = if contains(
        &bounds,
        &LogicalPoint {
            x: local.x + bounds.x,
            y: local.y + bounds.y,
        },
    ) {
        LogicalPoint {
            x: local.x + bounds.x,
            y: local.y + bounds.y,
        }
    } else {
        cursor_point.unwrap_or(LogicalPoint {
            x: bounds.x + bounds.width / 2.0,
            y: bounds.y + bounds.height / 2.0,
        })
    };
    root.ivars().selection.borrow_mut().move_to(point, false);
    root.refresh();
    root.ivars().updating.set(false);
    Ok(())
}

fn current_view(id: &str) -> Option<Retained<CaptureView>> {
    WINDOW.with_borrow(|current| {
        current
            .as_ref()
            .filter(|(current, _, _)| current == id)
            .map(|(_, _, view)| view.clone())
    })
}
pub(crate) fn is_current(id: &str) -> bool {
    current_view(id).is_some()
}
pub(crate) fn session_id() -> Option<String> {
    WINDOW.with_borrow(|current| current.as_ref().map(|(id, _, _)| id.clone()))
}
pub(crate) fn reveal() -> Result<(), String> {
    let id = session_id().ok_or("Native capture window is not open")?;
    action_finished(&id, None);
    super::toolbar::reveal();
    Ok(())
}
pub(crate) fn close_session(id: &str) {
    if is_current(id) {
        close();
    }
}
pub(crate) fn close() {
    let current = WINDOW.with_borrow_mut(Option::take);
    if let Some((id, window, view)) = current {
        view.ivars().updating.set(true);
        super::toolbar::close_session(&id);
        view.ivars().updating.set(true);
        window.setDelegate(None);
        window.orderOut(None);
        window.setContentView(None);
        window.close();
        super::presentation::restore_native_crosshair_cursor();
    }
}
pub(crate) fn suspend(id: &str) {
    if let Some(view) = current_view(id) {
        view.ivars().suspended.set(true);
        super::toolbar::hide();
        if let Some(window) = view.window() {
            window.orderOut(None);
        }
        super::presentation::restore_native_crosshair_cursor();
    }
}
pub(crate) fn action_finished(id: &str, error: Option<String>) {
    let Some(view) = current_view(id) else {
        return;
    };
    view.ivars().updating.set(true);
    if view.ivars().suspended.replace(false) {
        if let Some(window) = view.window() {
            window.makeKeyAndOrderFront(None);
            window.makeFirstResponder(Some(&view));
        }
        super::presentation::push_native_crosshair_cursor();
    }
    view.ivars().busy.set(false);
    view.ivars().opening_editor.set(false);
    if let Some(editor) = view.ivars().editor.borrow().as_ref() {
        editor.status(false, error.clone());
    }
    view.set_hint(
        error
            .as_deref()
            .unwrap_or("选择区域 · Enter 完成 · F5 重新截图 · Esc 取消"),
    );
    view.refresh();
    view.ivars().updating.set(false);
}

pub(crate) fn begin_editor(
    id: &str,
    model: CaptureEditor,
    pixels: image::RgbaImage,
    dark: bool,
    interface_theme: String,
) -> Result<(), String> {
    let view = current_view(id).ok_or("Native capture session is no longer current")?;
    let bounds = view.ivars().selection.borrow().bounds.clone();
    let selection = model.selection.clone();
    let session_id = id.to_owned();
    let editor = super::canvas::EditorView::new(
        id,
        &view,
        model,
        pixels,
        view.ivars().ui_scale,
        dark,
        interface_theme,
        view.ivars().include_cursor.get(),
        view.ivars().preferences.annotation_tool_shortcuts.clone(),
        move |request| {
            if let Some(view) = current_view(&session_id) {
                match request {
                    super::canvas::Request::SelectionInput {
                        point,
                        phase,
                        constrained,
                    } => {
                        view.selection_input(point, phase, constrained);
                        return;
                    }
                    super::canvas::Request::SelectionNudge(dx, dy) => {
                        view.ivars().selection.borrow_mut().nudge_rect(dx, dy);
                        view.refresh_editor_selection();
                        return;
                    }
                    _ => {}
                }
                if matches!(request, super::canvas::Request::Output { .. }) {
                    view.ivars().busy.set(true);
                }
                (view.ivars().dispatch)(Action::Editor(request));
            }
        },
    )?;
    if let Some(previous) = view.ivars().editor.borrow_mut().take() {
        previous.removeFromSuperview();
    }
    editor.setFrame(local_rect(&selection, &bounds));
    view.ivars().selection.borrow_mut().rect = Some(selection);
    *view.ivars().editor.borrow_mut() = Some(editor.clone());
    view.ivars().busy.set(false);
    view.ivars().opening_editor.set(false);
    view.set_hint("");
    view.refresh();
    view.ivars().lens.get().unwrap().setHidden(true);
    editor.activate();
    if let Some(window) = view.window() {
        window.makeFirstResponder(Some(&editor));
    }
    Ok(())
}

fn current_editor(id: &str) -> Option<Retained<super::canvas::EditorView>> {
    current_view(id).and_then(|view| {
        let editor = view.ivars().editor.borrow().clone();
        editor
    })
}
pub(crate) fn editor_control(id: &str, action: CaptureEditorControlAction) {
    if let Some(editor) = current_editor(id) {
        editor.control(action);
    }
}
pub(crate) fn editor_is_busy(id: &str) -> bool {
    current_view(id).is_some_and(|view| {
        view.ivars().busy.get() || view.ivars().updating.get() || view.ivars().suspended.get()
    })
}
pub(crate) fn editor_cancel_selection(id: &str) -> bool {
    let Some(view) = current_view(id) else {
        return false;
    };
    if !view.ivars().selection.borrow().editing() {
        return false;
    }
    view.selection_input(LogicalPoint { x: 0.0, y: 0.0 }, "cancel", false);
    if let Some(editor) = current_editor(id) {
        editor.cancel_selection_tracking();
    }
    true
}
pub(crate) fn editor_rendered(
    id: &str,
    revision: u64,
    excluded: Option<usize>,
    pixels: image::RgbaImage,
) -> Result<(), String> {
    if let Some(editor) = current_editor(id) {
        editor.rendered(revision, excluded, pixels)?;
    }
    Ok(())
}
pub(crate) fn editor_render_failed(id: &str, revision: u64, excluded: Option<usize>, error: &str) {
    if let Some(editor) = current_editor(id) {
        editor.render_failed(revision, excluded, error);
    }
}
pub(crate) fn editor_fonts(id: &str, fonts: Vec<String>) {
    if let Some(editor) = current_editor(id) {
        editor.fonts(fonts);
    }
}
pub(crate) fn editor_presets(id: &str, colors: Vec<[u8; 4]>) {
    if let Some(editor) = current_editor(id) {
        editor.presets(colors);
    }
}
pub(crate) fn report_error(id: &str, error: &str) {
    if let Some(view) = current_view(id) {
        view.set_hint(error);
        if let Some(editor) = view.ivars().editor.borrow().as_ref() {
            editor.report_error(error);
        }
    }
}
pub(crate) fn request_pin(id: &str) -> bool {
    let Some(view) = current_view(id) else {
        return false;
    };
    if let Some(editor) = current_editor(id) {
        editor.control(CaptureEditorControlAction::Command("pin"));
    } else {
        view.complete(Operation::Pin);
    }
    true
}
pub(crate) fn control_result(id: &str, revision: u64, result: Result<Option<LogicalRect>, String>) {
    let Some(view) = current_view(id) else {
        return;
    };
    if view.ivars().busy.get() {
        return;
    }
    match result {
        Ok(rect) => {
            if view
                .ivars()
                .selection
                .borrow_mut()
                .control_result(revision, rect)
            {
                view.refresh();
            }
        }
        Err(error) => {
            if view.ivars().selection.borrow_mut().control_failed(revision) {
                view.set_hint(&format!("界面元素检测不可用：{error}"));
                view.refresh();
            }
        }
    }
}

fn print_view(png: &[u8], mtm: MainThreadMarker) -> Result<Retained<NSImageView>, String> {
    let image = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(png))
        .ok_or("Could not decode capture for printing")?;
    let view = NSImageView::initWithFrame(
        NSImageView::alloc(mtm),
        rect(0.0, 0.0, image.size().width, image.size().height),
    );
    view.setImage(Some(&image));
    view.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
    Ok(view)
}
pub(crate) fn print(id: &str, png: Vec<u8>) -> Result<(), String> {
    if !is_current(id) {
        return Ok(());
    }
    let view = print_view(
        &png,
        MainThreadMarker::new().ok_or("Printing must run on the main thread")?,
    )?;
    suspend(id);
    let info = NSPrintInfo::sharedPrintInfo().copy();
    info.setHorizontalPagination(NSPrintingPaginationMode::Fit);
    info.setVerticalPagination(NSPrintingPaginationMode::Fit);
    info.setHorizontallyCentered(true);
    info.setVerticallyCentered(true);
    let operation = NSPrintOperation::printOperationWithView_printInfo(&view, &info);
    operation.setJobTitle(Some(&NSString::from_str("SnapLingo 截图")));
    operation.runOperation();
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_ready_size(id: &str) -> (u32, u32, bool) {
    let view = current_view(id).unwrap();
    let image = &view.ivars().rasters[0].bitmap;
    (
        image.pixelsWide() as u32,
        image.pixelsHigh() as u32,
        !image.bitmapData().is_null() && view.window().unwrap().isVisible(),
    )
}
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_key(id: &str, event: &NSEvent) {
    let view = current_view(id).unwrap();
    let equivalent: bool = unsafe { msg_send![&view, performKeyEquivalent: event] };
    if !equivalent {
        // Isolated test processes do not always own the desktop keyboard.
        // Deliver the real event to this responder, independently of OS focus.
        unsafe {
            if view.ivars().editor.borrow().is_some() {
                let responder = view.window().unwrap().firstResponder().unwrap();
                let _: () = msg_send![&responder, keyDown: event];
            } else {
                let _: () = msg_send![&view, keyDown: event];
            }
        }
    }
}
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_window_number(id: &str) -> isize {
    current_view(id).unwrap().window().unwrap().windowNumber()
}
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_pointer(id: &str, event: &NSEvent, phase: &str) {
    let view = current_view(id).unwrap();
    if let Some(editor) = current_editor(id) {
        unsafe {
            match phase {
                "down" => {
                    let _: () = msg_send![&editor, mouseDown: event];
                }
                "move" => {
                    let _: () = msg_send![&editor, mouseDragged: event];
                }
                "up" => {
                    let _: () = msg_send![&editor, mouseUp: event];
                }
                "right" => {
                    let _: () = msg_send![&editor, rightMouseDown: event];
                }
                "flags" => {}
                _ => panic!("Unknown pointer phase"),
            }
        }
        return;
    }
    unsafe {
        match phase {
            "down" => {
                let _: () = msg_send![&view, mouseDown: event];
            }
            "move" => view.move_pointer(event),
            "up" => {
                let _: () = msg_send![&view, mouseUp: event];
            }
            "right" => {
                let _: () = msg_send![&view, rightMouseDown: event];
            }
            "flags" => {
                let _: () = msg_send![&view, flagsChanged: event];
            }
            _ => panic!("Unknown pointer phase"),
        }
    }
}

#[cfg(test)]
pub(crate) fn test_editor_state(id: &str) -> Option<super::canvas::TestState> {
    current_editor(id).map(|editor| editor.test_state())
}
#[cfg(test)]
pub(crate) fn test_editor_text(id: &str, text: &str) {
    current_editor(id).unwrap().test_text(text);
}
#[cfg(test)]
pub(crate) fn test_editor_commit_text(id: &str) -> bool {
    current_editor(id).unwrap().test_commit_text()
}
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_frame(id: &str) -> NSRect {
    current_view(id).unwrap().window().unwrap().frame()
}
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_selection(id: &str) -> Option<LogicalRect> {
    current_view(id)
        .unwrap()
        .ivars()
        .selection
        .borrow()
        .active_rect()
        .cloned()
}
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_color(id: &str) -> Option<[u8; 3]> {
    current_view(id).unwrap().sampled_color()
}
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_snapshot(id: &str) -> Vec<u8> {
    crate::infrastructure::system::native_view_snapshot::snapshot(&current_view(id).unwrap())
}
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_decoded_images() -> usize {
    WINDOW.with_borrow(|current| {
        current
            .as_ref()
            .map_or(0, |(_, _, view)| view.ivars().rasters.len())
    })
}
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_print_snapshot(png: &[u8]) -> Vec<u8> {
    crate::infrastructure::system::native_view_snapshot::snapshot(
        &print_view(png, MainThreadMarker::new().unwrap()).unwrap(),
    )
}
