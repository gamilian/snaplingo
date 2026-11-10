use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::{
    define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSBackingStoreType, NSBezierPath, NSButton, NSColor, NSEvent, NSFont, NSFontWeightMedium,
    NSFontWeightSemibold, NSPanel, NSScreenSaverWindowLevel, NSTextAlignment, NSTextField,
    NSTrackingArea, NSTrackingAreaOptions, NSView, NSWindowCollectionBehavior, NSWindowSharingType,
    NSWindowStyleMask,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

use super::native_theme::{color, is_dark, palette, Palette};
use snaplingo_core::capture::model::LogicalRect;
use snaplingo_core::capture::model::ScrollingCaptureView;
use snaplingo_core::settings::model::GeneralSettings;

pub(super) use snaplingo_ui::scrolling::ScrollingAction as Action;
use snaplingo_ui::scrolling::{ScrollingFrame, ScrollingSnapshot, ScrollingUi};
use snaplingo_ui::{KeyStyle, Point, Rect, Snapshot, Theme};

fn model(
    view: &ScrollingCaptureView,
    general: &GeneralSettings,
    working: bool,
    error: Option<String>,
) -> ScrollingSnapshot {
    ScrollingSnapshot {
        shell: Snapshot {
            theme: match general.interface_theme.as_str() {
                "studio" => Theme::Studio,
                "paper" => Theme::Paper,
                _ => Theme::Native,
            },
            dark: is_dark(&general.theme),
            key_style: KeyStyle::Mac,
            ..Snapshot::default()
        },
        width: view.width,
        height: view.height,
        can_undo: view.can_undo,
        preview_available: view.preview_available,
        preview_expanded: view.preview_expanded,
        working,
        hint: view.hint.clone(),
        error,
    }
}
fn native_rect(rect: Rect) -> NSRect {
    NSRect::new(
        NSPoint::new(rect.x as f64, rect.y as f64),
        NSSize::new(rect.width as f64, rect.height as f64),
    )
}
fn action(tag: isize) -> Action {
    match tag {
        0 => Action::Undo,
        1 => Action::Preview,
        2 => Action::Copy,
        3 => Action::Save,
        4 => Action::Pin,
        5 => Action::Complete,
        6 => Action::Cancel,
        _ => Action::Resume,
    }
}

struct ControlIvars {
    dispatch: Box<dyn Fn(Action)>,
    scale: f64,
    general: RefCell<GeneralSettings>,
    colors: Cell<Palette>,
    ui: RefCell<ScrollingUi>,
    error: Cell<bool>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoScrollingControls"]
    #[thread_kind = MainThreadOnly]
    #[ivars = ControlIvars]
    struct ControlView;

    unsafe impl NSObjectProtocol for ControlView {}

    impl ControlView {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let scale = self.ivars().scale;
            let colors = self.ivars().colors.get();
            let mut bounds = self.bounds();
            bounds.origin = NSPoint::new(0.5 * scale, 0.5 * scale);
            bounds.size.width -= scale;
            bounds.size.height -= scale;
            let outline = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(bounds, 13.0 * scale, 13.0 * scale);
            color(colors.surface).setFill(); outline.fill();
            color(colors.line).setStroke(); outline.setLineWidth(scale); outline.stroke();
            for view in self.subviews().iter() {
                let Some(button) = view.downcast_ref::<NSButton>() else { continue; };
                if button.tag() == 7 {
                    if !button.isHidden() {
                        let frame = button.frame();
                        let line = NSBezierPath::bezierPath();
                        line.moveToPoint(NSPoint::new(frame.origin.x + 12.5 * scale, frame.origin.y + 14.0 * scale));
                        line.lineToPoint(NSPoint::new(frame.origin.x + 33.5 * scale, frame.origin.y + 14.0 * scale));
                        color(colors.danger).setStroke(); line.setLineWidth(0.5 * scale); line.stroke();
                    }
                    continue;
                }
                let primary = button.tag() == 5;
                let alpha = if button.isEnabled() { 1.0 } else { 0.4 };
                let hovered = button.isEnabled() && self.ivars().ui.borrow().hovered() == Some(action(button.tag()));
                let path = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(button.frame(), 4.0 * scale, 4.0 * scale);
                let fill = match (primary, hovered) {
                    (true, true) => colors.accent_hover,
                    (true, false) => colors.accent,
                    (false, true) => colors.raised,
                    (false, false) => colors.surface,
                };
                color(fill).colorWithAlphaComponent(alpha).setFill(); path.fill();
                color(if primary { colors.accent } else { colors.line }).colorWithAlphaComponent(alpha).setStroke(); path.setLineWidth(scale); path.stroke();
            }
        }
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool { true }
        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) { self.hover(self.convertPoint_fromView(event.locationInWindow(), None)); }
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, event: &NSEvent) { self.hover(self.convertPoint_fromView(event.locationInWindow(), None)); }
        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            let changed = self.ivars().ui.borrow_mut().hover(None);
            if changed { self.setNeedsDisplay(true); }
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            if point.y > 31.75 * self.ivars().scale { return; }
            if let Some(window) = self.window() {
                window.performWindowDragWithEvent(event);
            }
        }

        #[unsafe(method(performAction:))]
        fn perform_action(&self, sender: &NSButton) {
            if !sender.isEnabled() { return; }
            let action = self.ivars().ui.borrow().activate(action(sender.tag()));
            if let Some(action) = action { (self.ivars().dispatch)(action); }

        }
        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn appearance_changed(&self) { self.apply_palette(); }
    }
);

impl ControlView {
    fn hover(&self, point: NSPoint) {
        let changed = self.ivars().ui.borrow_mut().hover(Some(Point {
            x: point.x as f32,
            y: point.y as f32,
        }));
        if changed {
            self.setNeedsDisplay(true);
        }
    }
    fn apply_palette(&self) {
        let general = self.ivars().general.borrow();
        let colors = palette(&general.interface_theme, is_dark(&general.theme));
        self.ivars().colors.set(colors);
        for view in self.subviews().iter() {
            if let Some(button) = view.downcast_ref::<NSButton>() {
                button.setContentTintColor(Some(&color(if button.tag() == 5 {
                    colors.accent_on
                } else if button.tag() == 7 {
                    colors.danger
                } else {
                    colors.ink
                })));
            } else if let Some(field) = view.downcast_ref::<NSTextField>() {
                field.setTextColor(Some(&color(if field.tag() == 10 {
                    colors.ink
                } else if field.tag() == 12 && self.ivars().error.get() {
                    colors.danger
                } else {
                    colors.muted
                })));
            }
        }
        self.setNeedsDisplay(true);
    }

    fn update_theme(&self, general: GeneralSettings) {
        *self.ivars().general.borrow_mut() = general;
        if let Some(window) = self.window() {
            let appearance = match self.ivars().general.borrow().theme.as_str() {
                "dark" => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }),
                "light" => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameAqua }),
                _ => None,
            };
            window.setAppearance(appearance.as_deref());
        }
        self.apply_palette();
    }
}

struct Controls {
    window: Retained<NSPanel>,
    root: Retained<ControlView>,
    dimensions: Retained<NSTextField>,
    status: Retained<NSTextField>,
    buttons: Vec<Retained<NSButton>>,
    view: ScrollingCaptureView,
    working: bool,
    error: Option<String>,
}

thread_local! {
    static CONTROLS: RefCell<HashMap<String, Controls>> = RefCell::new(HashMap::new());
}

pub(super) fn open(
    mut view: ScrollingCaptureView,
    rect: &LogicalRect,
    general: GeneralSettings,
    dispatch: impl Fn(Action) + 'static,
) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("Scrolling controls must open on the main thread")?;
    let scale = view.ui_scale;
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(rect.width, rect.height));
    let window = NSPanel::initWithContentRect_styleMask_backing_defer(
        NSPanel::alloc(mtm),
        frame,
        NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
        NSBackingStoreType::Buffered,
        false,
    );
    window.setTitle(&NSString::from_str("SnapLingo 长截图"));
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setOpaque(false);
    window.setHasShadow(true);
    window.setFloatingPanel(true);
    window.setHidesOnDeactivate(false);
    window.setAcceptsMouseMovedEvents(true);
    window.setCanHide(false);
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

    let root = ControlView::alloc(mtm).set_ivars(ControlIvars {
        dispatch: Box::new(dispatch),
        scale,
        colors: Cell::new(palette(&general.interface_theme, is_dark(&general.theme))),
        ui: RefCell::new(ScrollingUi::new(model(&view, &general, false, None))),
        error: Cell::new(false),
        general: RefCell::new(general.clone()),
    });
    let root: Retained<ControlView> = unsafe { msg_send![super(root), initWithFrame: frame] };
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
    let ui_frame = root.ivars().ui.borrow_mut().frame(scale as f32);
    let label = |text: &str, rect: Rect, font_size: f64| {
        let field = NSTextField::labelWithString(&NSString::from_str(text), mtm);
        field.setFont(Some(&NSFont::systemFontOfSize(font_size * scale)));
        field.setFrame(native_rect(rect));
        root.addSubview(&field);
        field
    };
    let title = label("长截图", ui_frame.title, 12.5);
    title.setTag(10);
    title.setFont(Some(&NSFont::systemFontOfSize_weight(
        12.5 * scale,
        unsafe { NSFontWeightSemibold },
    )));
    let dimensions = label("", ui_frame.dimensions, 10.5);
    dimensions.setAlignment(NSTextAlignment::Right);
    let mut buttons = Vec::new();
    for (tag, control) in ui_frame.controls.iter().enumerate() {
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(&control.label),
                Some(&root),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        button.setTag(tag as isize);
        button.setBordered(false);
        button.setFont(Some(&NSFont::systemFontOfSize_weight(
            10.5 * scale,
            if tag == 5 {
                unsafe { NSFontWeightSemibold }
            } else {
                unsafe { NSFontWeightMedium }
            },
        )));
        button.setFrame(native_rect(control.rect));
        root.addSubview(&button);
        buttons.push(button);
    }
    let status = label("", ui_frame.status, 10.5);
    status.setTag(12);
    status.setMaximumNumberOfLines(2);
    status.setSelectable(true);
    let retry = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str("重试"),
            Some(&root),
            Some(sel!(performAction:)),
            mtm,
        )
    };
    retry.setTag(7);
    retry.setBordered(false);
    retry.setFont(Some(&NSFont::systemFontOfSize(10.5 * scale)));
    retry.setFrame(native_rect(ui_frame.retry));
    root.addSubview(&retry);
    buttons.push(retry);
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
    view.preview_base64 = String::new();
    let mut controls = Controls {
        window,
        root,
        dimensions,
        status,
        buttons,
        view,
        working: false,
        error: None,
    };
    controls.root.update_theme(general);
    controls.refresh();
    controls.window.orderFront(None);
    CONTROLS.with(|all| all.borrow_mut().insert(controls.view.id.clone(), controls));
    Ok(())
}

impl Controls {
    fn refresh(&mut self) {
        let mut redraw = false;
        self.root.ivars().error.set(self.error.is_some());
        let text = format!("{} × {}", self.view.width, self.view.height);
        if self.dimensions.stringValue().to_string() != text {
            self.dimensions.setStringValue(&NSString::from_str(&text));
        }
        let snapshot = model(
            &self.view,
            &self.root.ivars().general.borrow(),
            self.working,
            self.error.clone(),
        );
        let frame: ScrollingFrame = {
            let mut ui = self.root.ivars().ui.borrow_mut();
            ui.replace(snapshot);
            ui.frame(self.view.ui_scale as f32)
        };
        let hint = self.root.ivars().ui.borrow().status_text().to_owned();
        if self.status.stringValue().to_string() != hint {
            self.status.setStringValue(&NSString::from_str(&hint));
        }
        let tooltip = self.status.toolTip().map(|tooltip| tooltip.to_string());
        if tooltip.as_deref() != Some(hint.as_str()) {
            self.status.setToolTip(Some(&NSString::from_str(&hint)));
        }
        if self.status.frame() != native_rect(frame.status) {
            self.status.setFrame(native_rect(frame.status));
        }
        let colors = self.root.ivars().colors.get();
        let color = color(if self.error.is_some() {
            colors.danger
        } else {
            colors.muted
        });
        if self.status.textColor().as_deref() != Some(&color) {
            self.status.setTextColor(Some(&color));
        }
        for (index, button) in self.buttons.iter().enumerate() {
            let control = frame.controls.get(index);
            let enabled = control.is_some_and(|control| control.enabled);
            let hidden = control.is_none();
            if button.isEnabled() != enabled {
                button.setEnabled(enabled);
                redraw = true;
            }
            if button.isHidden() != hidden {
                button.setHidden(hidden);
                redraw = true;
            }
            if let Some(control) = control {
                if button.title().to_string() != control.label {
                    button.setTitle(&NSString::from_str(&control.label));
                    redraw = true;
                }
                if button.frame() != native_rect(control.rect) {
                    button.setFrame(native_rect(control.rect));
                    redraw = true;
                }
            }
        }
        use snaplingo_core::capture::model::ScrollingShortcutAction;
        let shortcuts = &self.view.shortcut_actions;
        let tooltips = [
            shortcuts
                .contains(&ScrollingShortcutAction::Undo)
                .then_some("Ctrl/Cmd+Z"),
            Some(if self.view.preview_available {
                "显示或隐藏鼠标穿透预览"
            } else {
                "当前屏幕没有足够的预览空间"
            }),
            None,
            Some("另存为 PNG、JPEG 或 WebP"),
            None,
            Some(if shortcuts.contains(&ScrollingShortcutAction::Complete) {
                "Enter · 复制并保存 PNG 到默认目录"
            } else {
                "复制并保存 PNG 到默认目录"
            }),
            shortcuts
                .contains(&ScrollingShortcutAction::Cancel)
                .then_some("Esc"),
        ];
        for (button, tooltip) in self.buttons.iter().zip(tooltips) {
            if button.toolTip().map(|value| value.to_string()).as_deref() != tooltip {
                button.setToolTip(tooltip.map(NSString::from_str).as_deref());
            }
        }
        if redraw {
            self.root.setNeedsDisplay(true);
        }
    }
}

pub(super) fn update_theme(general: GeneralSettings) {
    CONTROLS.with(|all| {
        for controls in all.borrow_mut().values_mut() {
            controls.root.update_theme(general.clone());
            controls.refresh();
        }
    });
}

pub(crate) fn update(mut view: ScrollingCaptureView) {
    view.preview_base64 = String::new();
    CONTROLS.with(|all| {
        if let Some(controls) = all.borrow_mut().get_mut(&view.id) {
            if view.revision < controls.view.revision {
                return;
            }
            controls.error =
                if view.status == snaplingo_core::capture::model::ScrollingCaptureStatus::Error {
                    view.hint.clone()
                } else {
                    None
                };
            controls.view = view;
            controls.refresh();
        }
    });
}

pub(crate) fn set_working(id: &str, working: bool, error: Option<String>) {
    CONTROLS.with(|all| {
        if let Some(controls) = all.borrow_mut().get_mut(id) {
            controls.working = working;
            controls.error = error.or_else(|| {
                if !working
                    && controls.view.status
                        == snaplingo_core::capture::model::ScrollingCaptureStatus::Error
                {
                    controls.view.hint.clone()
                } else {
                    None
                }
            });
            controls.refresh();
        }
    });
}

pub(super) fn rect(id: &str) -> Option<LogicalRect> {
    CONTROLS.with(|all| {
        all.borrow().get(id).map(|controls| {
            let frame = controls.window.frame();
            let main_height = core_graphics::display::CGDisplay::main()
                .bounds()
                .size
                .height;
            LogicalRect {
                x: frame.origin.x,
                y: main_height - frame.origin.y - frame.size.height,
                width: frame.size.width,
                height: frame.size.height,
            }
        })
    })
}

pub(super) fn hide_if_overlapping(id: &str, viewport: &LogicalRect) -> bool {
    let Some(bounds) = rect(id) else {
        return false;
    };
    if !super::scrolling_geometry::overlaps(&bounds, viewport) {
        return false;
    }
    CONTROLS.with(|all| {
        all.borrow().get(id).is_some_and(|controls| {
            let visible = controls.window.isVisible();
            if visible {
                controls.window.orderOut(None);
            }
            visible
        })
    })
}

pub(super) fn show(id: &str) {
    CONTROLS.with(|all| {
        if let Some(controls) = all.borrow().get(id) {
            controls.window.orderFront(None);
        }
    });
}

pub(super) fn close(id: &str) {
    CONTROLS.with(|all| {
        if let Some(controls) = all.borrow_mut().remove(id) {
            controls.window.orderOut(None);
            controls.window.close();
        }
    });
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn test_state(id: &str) -> Option<(String, String, Vec<bool>, bool)> {
    CONTROLS.with(|all| {
        all.borrow().get(id).map(|controls| {
            (
                controls.dimensions.stringValue().to_string(),
                controls.status.stringValue().to_string(),
                controls
                    .buttons
                    .iter()
                    .map(|button| button.isEnabled())
                    .collect(),
                controls.window.isVisible(),
            )
        })
    })
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn test_click(id: &str, action: usize) {
    let button = CONTROLS.with(|all| {
        all.borrow()
            .get(id)
            .map(|controls| controls.buttons[action].clone())
    });
    if let Some(button) = button {
        unsafe { button.performClick(None) };
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn test_retry_visible(id: &str) -> bool {
    CONTROLS.with(|all| !all.borrow().get(id).unwrap().buttons[7].isHidden())
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn test_frames(id: &str) -> Vec<NSRect> {
    CONTROLS.with(|all| {
        all.borrow()
            .get(id)
            .unwrap()
            .buttons
            .iter()
            .map(|button| button.frame())
            .collect()
    })
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn test_snapshot(id: &str) -> Vec<u8> {
    CONTROLS.with(|all| {
        crate::integrations::system::native_view_snapshot::snapshot(
            all.borrow()
                .get(id)
                .unwrap()
                .window
                .contentView()
                .unwrap()
                .as_ref(),
        )
    })
}
