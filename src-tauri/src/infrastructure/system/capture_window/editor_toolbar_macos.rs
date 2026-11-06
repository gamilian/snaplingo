#![cfg_attr(test, allow(dead_code))]

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSApplication, NSBackingStoreType, NSBezelStyle,
    NSBezierPath, NSButton, NSCellImagePosition, NSColor, NSEvent, NSEventModifierFlags, NSFont,
    NSImage, NSPanel, NSView, NSWindow, NSWindowAnimationBehavior, NSWindowOrderingMode,
    NSWindowSharingType, NSWindowStyleMask,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::application::capture::CaptureEditorToolbarPresentation;

const TOOLS: &[(&str, &str, &str)] = &[
    ("select", "选择并移动", "cursorarrow"),
    ("rectangle", "矩形", "rectangle"),
    ("ellipse", "椭圆 / 圆", "circle"),
    ("arrow", "箭头", "arrow.up.right"),
    ("line", "直线", "line.diagonal"),
    ("pen", "画笔", "pencil.tip"),
    ("text", "文字", "textformat"),
    ("mosaic", "马赛克", "checkerboard.rectangle"),
    ("eraser", "橡皮擦", "eraser"),
    ("highlight", "荧光笔", "highlighter"),
    ("region-highlight", "区域高亮", "square.fill"),
    ("blur", "模糊", "drop.halffull"),
    ("spotlight", "聚光", "flashlight.on.fill"),
];
const COMMANDS: &[(&str, &str, &str)] = &[
    ("undo", "撤销 · Cmd+Z", "arrow.uturn.backward"),
    ("redo", "重做 · Cmd+Y", "arrow.uturn.forward"),
    ("clear", "清空标注 · Cmd+Alt+Backspace", "trash"),
    ("cancel", "取消 · Esc", "xmark"),
    ("ocr", "识别文字", "text.viewfinder"),
    ("scrolling", "长截图", "arrow.up.and.down.square"),
    ("favorite", "收藏截图", "star"),
    ("pin", "贴图 · F3 / Cmd+T", "pin"),
    (
        "save",
        "保存 · Cmd+S · Shift 点击快速保存",
        "square.and.arrow.down",
    ),
    ("copy", "复制 · Cmd+C", "doc.on.doc"),
];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolbarAction<'a> {
    session_id: &'a str,
    action: &'a str,
}

define_class!(
    #[unsafe(super(NSPanel))]
    #[name = "SnapLingoEditorToolbarPanel"]
    #[thread_kind = MainThreadOnly]
    struct ToolbarPanel;
    unsafe impl NSObjectProtocol for ToolbarPanel {}
    impl ToolbarPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool { false }
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool { false }
    }
);

struct ToolbarIvars {
    app: AppHandle,
    session_id: String,
    presentation: RefCell<CaptureEditorToolbarPresentation>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoEditorToolbarView"]
    #[thread_kind = MainThreadOnly]
    #[ivars = ToolbarIvars]
    struct ToolbarView;
    unsafe impl NSObjectProtocol for ToolbarView {}
    impl ToolbarView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool { true }
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            NSColor::windowBackgroundColor().setFill();
            NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(self.bounds(), 4.0, 4.0).fill();
        }
        #[unsafe(method(performAction:))]
        fn perform_action(&self, sender: &NSButton) {
            if !sender.isEnabled() { return; }
            let Some((action, _, _)) = TOOLS.iter().chain(COMMANDS).nth(sender.tag() as usize) else { return; };
            let state = self.ivars().presentation.borrow();
            if state.busy { return; }
            // Grouped shape buttons activate their tool; repeated clicks do not toggle it off.
            if matches!(*action, "rectangle" | "ellipse" | "arrow" | "line")
                && state.active_tool.as_deref() == Some(action) { return; }
            let shift = NSApplication::sharedApplication(self.mtm())
                .currentEvent()
                .is_some_and(|event| event.modifierFlags().contains(NSEventModifierFlags::Shift));
            let action = if *action == "save" && shift { "quick-save" } else { action };
            let _ = self.ivars().app.emit_to(
                super::backend::CAPTURE_WINDOW_LABEL,
                "capture-editor-toolbar-action",
                ToolbarAction { session_id: &self.ivars().session_id, action },
            );
        }
    }
);

struct Toolbar {
    parent: Retained<NSWindow>,
    window: Retained<ToolbarPanel>,
    view: Retained<ToolbarView>,
    buttons: Vec<Retained<NSButton>>,
    styles: super::editor_style_macos::StyleControls,
}

thread_local! {
    static TOOLBAR: RefCell<Option<Toolbar>> = const { RefCell::new(None) };
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}

fn create(
    app: &AppHandle,
    id: &str,
    parent: Retained<NSWindow>,
    presentation: &CaptureEditorToolbarPresentation,
) -> Toolbar {
    let mtm = parent.mtm();
    let frame = rect(
        0.0,
        0.0,
        presentation.frame.width,
        presentation.frame.height,
    );
    let window = ToolbarPanel::alloc(mtm).set_ivars(());
    let window: Retained<ToolbarPanel> = unsafe {
        msg_send![super(window), initWithContentRect: frame,
            styleMask: NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
            backing: NSBackingStoreType::Buffered, defer: false]
    };
    window.setTitle(&NSString::from_str("SnapLingo 标注工具条"));
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setHasShadow(false);
    window.setHidesOnDeactivate(false);
    window.setCanHide(false);
    window.setAnimationBehavior(NSWindowAnimationBehavior::None);
    window.setSharingType(NSWindowSharingType::None);
    window.setCollectionBehavior(parent.collectionBehavior());
    window.setExcludedFromWindowsMenu(true);
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    let view = ToolbarView::alloc(mtm).set_ivars(ToolbarIvars {
        app: app.clone(),
        session_id: id.into(),
        presentation: RefCell::new(presentation.clone()),
    });
    let view: Retained<ToolbarView> = unsafe { msg_send![super(view), initWithFrame: frame] };
    let buttons = TOOLS
        .iter()
        .chain(COMMANDS)
        .enumerate()
        .map(|(index, (_, title, symbol))| {
            let button = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str(title),
                    Some(&view),
                    Some(sel!(performAction:)),
                    mtm,
                )
            };
            button.setTag(index as isize);
            button.setBezelStyle(NSBezelStyle::Push);
            button.setToolTip(Some(&NSString::from_str(title)));
            if let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                &NSString::from_str(symbol),
                Some(&NSString::from_str(title)),
            ) {
                button.setImage(Some(&image));
                button.setImagePosition(NSCellImagePosition::ImageOnly);
                button.setBordered(false);
            }
            view.addSubview(&button);
            button
        })
        .collect();
    let styles =
        super::editor_style_macos::StyleControls::new(app, id, &view, &presentation.styling);
    window.setContentView(Some(&view));
    window.orderOut(None);
    // Both windows are retained by Toolbar until the child is explicitly detached.
    unsafe {
        parent.addChildWindow_ordered(&window, NSWindowOrderingMode::Above);
    }
    Toolbar {
        parent,
        window,
        view,
        buttons,
        styles,
    }
}

impl Toolbar {
    fn update(&self, presentation: &CaptureEditorToolbarPresentation) {
        let frame = &presentation.frame;
        let parent = self.parent.frame();
        let native_frame = rect(
            parent.origin.x + frame.x,
            parent.origin.y + parent.size.height - frame.y - frame.height,
            frame.width,
            frame.height,
        );
        if self.window.frame() != native_frame {
            self.window.setFrame_display(native_frame, false);
        }
        self.view
            .setFrame(rect(0.0, 0.0, frame.width, frame.height));
        let appearance = if presentation.dark {
            "NSAppearanceNameDarkAqua"
        } else {
            "NSAppearanceNameAqua"
        };
        self.window.setAppearance(
            NSAppearance::appearanceNamed(&NSString::from_str(appearance)).as_deref(),
        );
        let scale = presentation.ui_scale;
        let row_height = 32.0 * scale;
        for (row, items) in [TOOLS, COMMANDS].into_iter().enumerate() {
            let cell = (frame.width / items.len() as f64).min(44.0 * scale);
            let offset = (frame.width - cell * items.len() as f64) / 2.0;
            for (column, (action, _, _)) in items.iter().enumerate() {
                let index = if row == 0 {
                    column
                } else {
                    TOOLS.len() + column
                };
                let button = &self.buttons[index];
                button.setFrame(rect(
                    offset + column as f64 * cell + 2.0 * scale,
                    row as f64 * row_height + 2.0 * scale,
                    (cell - 4.0 * scale).max(1.0),
                    (row_height - 4.0 * scale).max(1.0),
                ));
                button.setFont(Some(&NSFont::systemFontOfSize(11.0 * scale)));
                button.setEnabled(
                    !presentation.busy
                        && match *action {
                            "undo" | "clear" => presentation.can_undo,
                            "redo" => presentation.can_redo,
                            _ => true,
                        },
                );
                let active = if *action == "select" {
                    presentation.active_tool.is_none()
                } else {
                    row == 0 && presentation.active_tool.as_deref() == Some(action)
                };
                button.setState(if active { 1 } else { 0 });
                let tint = if active {
                    NSColor::controlAccentColor()
                } else {
                    NSColor::labelColor()
                };
                button.setContentTintColor(Some(&tint));
            }
        }
        self.styles.root().setFrame(rect(
            0.0,
            64.0 * scale,
            frame.width,
            frame.height - 64.0 * scale,
        ));
        self.styles
            .update(&presentation.styling, presentation.busy, scale);
        *self.view.ivars().presentation.borrow_mut() = presentation.clone();
        self.view.setNeedsDisplay(true);
        self.reveal();
    }
    fn reveal(&self) {
        self.window.setLevel(self.parent.level());
        if self.parent.isVisible() && self.view.ivars().presentation.borrow().visible {
            self.window.orderFront(None);
        } else {
            self.window.orderOut(None);
        }
    }
    fn close(self) {
        self.styles.close_color_panel();
        self.window.orderOut(None);
        self.parent.removeChildWindow(&self.window);
        self.window.setContentView(None);
        self.window.close();
    }
}

pub(crate) fn update(
    app: &AppHandle,
    id: &str,
    presentation: Option<&CaptureEditorToolbarPresentation>,
) -> Result<bool, String> {
    MainThreadMarker::new().ok_or("Editor toolbar must update on the main thread")?;
    let Some(presentation) = presentation else {
        close_session(id);
        return Ok(true);
    };
    let Some(window) = app.get_webview_window(super::backend::CAPTURE_WINDOW_LABEL) else {
        return Ok(false);
    };
    let pointer = window.ns_window().map_err(|error| error.to_string())?;
    let parent = unsafe { Retained::<NSWindow>::retain(pointer.cast()) }
        .ok_or("Capture editor window is unavailable")?;
    TOOLBAR.with(|toolbar| {
        let mut toolbar = toolbar.borrow_mut();
        if toolbar.as_ref().is_some_and(|current| {
            current.view.ivars().session_id != id || !std::ptr::eq(&*current.parent, &*parent)
        }) {
            toolbar.take().unwrap().close();
        }
        let current = toolbar.get_or_insert_with(|| create(app, id, parent, presentation));
        current.update(presentation);
    });
    Ok(true)
}

pub(crate) fn owns_focus(id: &str) -> bool {
    TOOLBAR.with(|toolbar| {
        toolbar.borrow().as_ref().is_some_and(|toolbar| {
            toolbar.view.ivars().session_id == id
                && toolbar.window.isVisible()
                && NSApplication::sharedApplication(toolbar.view.mtm()).isActive()
                && (toolbar.parent.isKeyWindow() || toolbar.styles.color_panel_owns_focus())
        })
    })
}

pub(crate) fn reveal() {
    TOOLBAR.with(|toolbar| {
        if let Some(toolbar) = toolbar.borrow().as_ref() {
            toolbar.reveal();
        }
    });
}

pub(crate) fn hide() {
    TOOLBAR.with(|toolbar| {
        if let Some(toolbar) = toolbar.borrow().as_ref() {
            toolbar.styles.close_color_panel();
            toolbar.window.orderOut(None);
        }
    });
}

pub(crate) fn close_session(id: &str) {
    TOOLBAR.with(|toolbar| {
        let mut toolbar = toolbar.borrow_mut();
        if toolbar
            .as_ref()
            .is_some_and(|current| current.view.ivars().session_id == id)
        {
            toolbar.take().unwrap().close();
        }
    });
}

pub(crate) fn close() {
    TOOLBAR.with(|toolbar| {
        if let Some(toolbar) = toolbar.borrow_mut().take() {
            toolbar.close();
        }
    });
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct TestState {
    pub session_id: String,
    pub active_tool: Option<String>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub visible: bool,
    pub requested_visible: bool,
    pub can_become_key: bool,
}

#[cfg(test)]
pub(crate) fn test_state() -> Option<TestState> {
    TOOLBAR.with(|toolbar| {
        toolbar.borrow().as_ref().map(|toolbar| {
            let state = toolbar.view.ivars().presentation.borrow();
            TestState {
                session_id: toolbar.view.ivars().session_id.clone(),
                active_tool: state.active_tool.clone(),
                can_undo: state.can_undo,
                can_redo: state.can_redo,
                visible: toolbar.window.isVisible(),
                requested_visible: state.visible,
                can_become_key: toolbar.window.canBecomeKeyWindow(),
            }
        })
    })
}

#[cfg(test)]
pub(crate) fn test_click(action: &str) {
    TOOLBAR.with(|toolbar| {
        let toolbar = toolbar.borrow();
        let toolbar = toolbar.as_ref().expect("Native editor toolbar is not open");
        let index = TOOLS
            .iter()
            .chain(COMMANDS)
            .position(|(id, _, _)| *id == action)
            .expect("Unknown toolbar button");
        // The main-thread button and its target remain retained throughout this action.
        unsafe {
            toolbar.buttons[index].performClick(None);
        }
    });
}

#[cfg(test)]
pub(crate) fn test_snapshot() -> Vec<u8> {
    TOOLBAR.with(|toolbar| {
        crate::infrastructure::system::native_view_snapshot::snapshot(
            &toolbar.borrow().as_ref().unwrap().view,
        )
    })
}

#[cfg(test)]
pub(crate) fn test_styles<T>(
    action: impl FnOnce(&super::editor_style_macos::StyleControls) -> T,
) -> T {
    TOOLBAR.with(|toolbar| action(&toolbar.borrow().as_ref().unwrap().styles))
}
