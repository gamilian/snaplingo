use std::cell::RefCell;
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSBezelStyle, NSBezierPath, NSButton, NSColor, NSEvent, NSFont, NSPanel,
    NSScreenSaverWindowLevel, NSTextField, NSView, NSWindowCollectionBehavior, NSWindowSharingType,
    NSWindowStyleMask,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

use crate::domain::capture::{LogicalRect, ScrollingCaptureView};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Undo,
    Preview,
    Copy,
    Save,
    Pin,
    Complete,
    Cancel,
    Resume,
}

struct ControlIvars {
    dispatch: Box<dyn Fn(Action)>,
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
        fn draw_rect(&self, dirty: NSRect) {
            NSColor::windowBackgroundColor().setFill();
            NSBezierPath::bezierPathWithRect(dirty).fill();
        }
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool { true }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            if let Some(window) = self.window() {
                window.performWindowDragWithEvent(event);
            }
        }

        #[unsafe(method(performAction:))]
        fn perform_action(&self, sender: &NSButton) {
            let action = match sender.tag() {
                0 => Action::Undo,
                1 => Action::Preview,
                2 => Action::Copy,
                3 => Action::Save,
                4 => Action::Pin,
                5 => Action::Complete,
                6 => Action::Cancel,
                _ => Action::Resume,
            };
            (self.ivars().dispatch)(action);
        }
    }
);

struct Controls {
    window: Retained<NSPanel>,
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
    window.setBackgroundColor(Some(&NSColor::windowBackgroundColor()));
    window.setHasShadow(true);
    window.setFloatingPanel(true);
    window.setHidesOnDeactivate(false);
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
    });
    let root: Retained<ControlView> = unsafe { msg_send![super(root), initWithFrame: frame] };
    let label = |text: &str, x: f64, y: f64, width: f64, height: f64, font_size: f64| {
        let field = NSTextField::labelWithString(&NSString::from_str(text), mtm);
        field.setFont(Some(&NSFont::systemFontOfSize(font_size * scale)));
        field.setFrame(NSRect::new(
            NSPoint::new(x * scale, y * scale),
            NSSize::new(width * scale, height * scale),
        ));
        root.addSubview(&field);
        field
    };
    label("长截图", 12.0, 10.0, 80.0, 22.0, 14.0);
    let dimensions = label("", 240.0, 12.0, 156.0, 20.0, 12.0);
    dimensions.setTextColor(Some(&NSColor::secondaryLabelColor()));
    let mut buttons = Vec::new();
    let mut x = 8.0;
    for (tag, (title, width)) in [
        ("撤销", 48.0),
        ("预览", 64.0),
        ("复制", 48.0),
        ("另存为", 62.0),
        ("贴图", 48.0),
        ("完成", 58.0),
        ("取消", 48.0),
    ]
    .into_iter()
    .enumerate()
    {
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(title),
                Some(&root),
                Some(sel!(performAction:)),
                mtm,
            )
        };
        button.setTag(tag as isize);
        button.setBezelStyle(NSBezelStyle::Push);
        button.setFont(Some(&NSFont::systemFontOfSize(12.0 * scale)));
        button.setFrame(NSRect::new(
            NSPoint::new(x * scale, 36.0 * scale),
            NSSize::new(width * scale, 32.0 * scale),
        ));
        root.addSubview(&button);
        buttons.push(button);
        x += width + 1.0;
    }
    let status = label("", 12.0, 76.0, 384.0, 44.0, 12.0);
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
    retry.setFrame(NSRect::new(
        NSPoint::new(342.0 * scale, 80.0 * scale),
        NSSize::new(56.0 * scale, 32.0 * scale),
    ));
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
        dimensions,
        status,
        buttons,
        view,
        working: false,
        error: None,
    };
    controls.refresh();
    controls.window.orderFront(None);
    CONTROLS.with(|all| all.borrow_mut().insert(controls.view.id.clone(), controls));
    Ok(())
}

impl Controls {
    fn refresh(&mut self) {
        let text = format!("{} × {}", self.view.width, self.view.height);
        if self.dimensions.stringValue().to_string() != text {
            self.dimensions.setStringValue(&NSString::from_str(&text));
        }
        let hint = if self.working {
            "正在处理…"
        } else {
            self.error
                .as_deref()
                .or(self.view.hint.as_deref())
                .unwrap_or("手动滚动页面，完成后复制并保存 PNG。")
        };
        if self.status.stringValue().to_string() != hint {
            self.status.setStringValue(&NSString::from_str(hint));
        }
        let tooltip = self.status.toolTip().map(|tooltip| tooltip.to_string());
        if tooltip.as_deref() != Some(hint) {
            self.status.setToolTip(Some(&NSString::from_str(hint)));
        }
        let width = if self.error.is_some() { 320.0 } else { 384.0 } * self.view.ui_scale;
        let frame = self.status.frame();
        if frame.size.width != width {
            self.status
                .setFrameSize(NSSize::new(width, frame.size.height));
        }
        let color = if self.error.is_some() {
            NSColor::systemRedColor()
        } else {
            NSColor::secondaryLabelColor()
        };
        if self.status.textColor().as_deref() != Some(&color) {
            self.status.setTextColor(Some(&color));
        }
        for (index, button) in self.buttons.iter().enumerate() {
            let enabled = !self.working
                && match index {
                    0 => self.view.can_undo,
                    1 => self.view.preview_available,
                    _ => true,
                };
            if button.isEnabled() != enabled {
                button.setEnabled(enabled);
            }
        }
        let preview_title = if self.view.preview_expanded {
            "隐藏预览"
        } else {
            "预览"
        };
        if self.buttons[1].title().to_string() != preview_title {
            self.buttons[1].setTitle(&NSString::from_str(preview_title));
        }
        if self.buttons[7].isHidden() != self.error.is_none() {
            self.buttons[7].setHidden(self.error.is_none());
        }
    }
}

pub(super) fn update(mut view: ScrollingCaptureView) {
    view.preview_base64 = String::new();
    CONTROLS.with(|all| {
        if let Some(controls) = all.borrow_mut().get_mut(&view.id) {
            if view.revision < controls.view.revision {
                return;
            }
            controls.error = if view.status == crate::domain::capture::ScrollingCaptureStatus::Error
            {
                view.hint.clone()
            } else {
                None
            };
            controls.view = view;
            controls.refresh();
        }
    });
}

pub(super) fn set_working(id: &str, working: bool, error: Option<String>) {
    CONTROLS.with(|all| {
        if let Some(controls) = all.borrow_mut().get_mut(id) {
            controls.working = working;
            controls.error = error.or_else(|| {
                if !working
                    && controls.view.status == crate::domain::capture::ScrollingCaptureStatus::Error
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
    if !super::scrolling_window::overlaps(&bounds, viewport) {
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
pub(super) fn test_snapshot(id: &str) -> Vec<u8> {
    CONTROLS.with(|all| {
        crate::infrastructure::system::native_view_snapshot::snapshot(
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
