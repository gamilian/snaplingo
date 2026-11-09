#![cfg_attr(test, allow(dead_code))]

use std::cell::RefCell;
use std::rc::Rc;

use crate::application::capture::{
    CaptureEditorControlAction, CaptureEditorStylePresentation, CaptureEditorToolbarPresentation,
};
use objc2::rc::Retained;
use objc2::{
    define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSApplication, NSBackingStoreType, NSBezelStyle,
    NSBezierPath, NSButton, NSCellImagePosition, NSColor, NSEvent, NSEventModifierFlags, NSFont,
    NSImage, NSMenu, NSMenuItem, NSPanel, NSView, NSWindow, NSWindowAnimationBehavior,
    NSWindowOrderingMode, NSWindowSharingType, NSWindowStyleMask,
};
use objc2_foundation::{NSData, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

pub(crate) type Dispatch = Rc<dyn Fn(CaptureEditorControlAction)>;

fn toolbar_icon(action: &str) -> Option<Retained<NSImage>> {
    // Rasterized at 4x from the existing frontend SVG paths. AppKit tints the alpha template.
    let bytes: &[u8] = match action {
        "select" => include_bytes!("../../../../assets/toolbar/select.png"),
        "shapes" => include_bytes!("../../../../assets/toolbar/shapes.png"),
        "rectangle" => include_bytes!("../../../../assets/toolbar/rectangle.png"),
        "ellipse" => include_bytes!("../../../../assets/toolbar/ellipse.png"),
        "arrow" => include_bytes!("../../../../assets/toolbar/arrow.png"),
        "line" => include_bytes!("../../../../assets/toolbar/line.png"),
        "pen" => include_bytes!("../../../../assets/toolbar/pen.png"),
        "text" => include_bytes!("../../../../assets/toolbar/text.png"),
        "mosaic" => include_bytes!("../../../../assets/toolbar/mosaic.png"),
        "eraser" => include_bytes!("../../../../assets/toolbar/eraser.png"),
        "highlight" => include_bytes!("../../../../assets/toolbar/highlight.png"),
        "region-highlight" => include_bytes!("../../../../assets/toolbar/region-highlight.png"),
        "blur" => include_bytes!("../../../../assets/toolbar/blur.png"),
        "spotlight" => include_bytes!("../../../../assets/toolbar/spotlight.png"),
        "save" => include_bytes!("../../../../assets/toolbar/save.png"),
        "copy" => include_bytes!("../../../../assets/toolbar/copy.png"),
        "undo" => include_bytes!("../../../../assets/toolbar/undo.png"),
        "redo" => include_bytes!("../../../../assets/toolbar/redo.png"),
        "pin" => include_bytes!("../../../../assets/toolbar/pin.png"),
        "favorite" => include_bytes!("../../../../assets/toolbar/favorite.png"),
        _ => return None,
    };
    let image = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(bytes))?;
    image.setSize(NSSize::new(17.0, 17.0));
    image.setTemplate(true);
    Some(image)
}

pub(crate) fn preferred_size(
    state: &CaptureEditorStylePresentation,
    available_width: f64,
    scale: f64,
) -> NSSize {
    let width = (700.0 * scale).min(available_width);
    let main = if width / scale < 460.0 { 100.0 } else { 68.0 };
    let rows = usize::from(state.text_mode)
        + usize::from(state.round_enabled)
        + usize::from(state.error.is_some());
    NSSize::new(
        width,
        (main
            + if rows > 0 {
                8.0 + rows as f64 * 36.0
            } else {
                0.0
            })
            * scale,
    )
}

fn color(channels: [u8; 3]) -> Retained<NSColor> {
    let [r, g, b] = channels.map(|value| value as f64 / 255.0);
    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0)
}

fn palette(presentation: &CaptureEditorToolbarPresentation) -> [[u8; 3]; 6] {
    let colors =
        super::super::native_theme::palette(&presentation.interface_theme, presentation.dark);
    [
        colors.surface,
        colors.line,
        colors.muted,
        colors.accent,
        colors.accent_soft,
        colors.accent_on,
    ]
}

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
    dispatch: Dispatch,
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
            let state = self.ivars().presentation.borrow();
            let scale = state.ui_scale;
            let colors = palette(&state);
            let main = if self.bounds().size.width / scale < 460.0 { 100.0 } else { 68.0 } * scale;
            for frame in [rect(0.5, 0.5, self.bounds().size.width - 1.0, main - 1.0), rect(0.5, main + 8.0 * scale, self.bounds().size.width - 1.0, self.bounds().size.height - main - 8.0 * scale - 0.5)] {
                if frame.size.height <= 0.0 { continue; }
                let path = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(frame, 11.0 * scale, 11.0 * scale);
                color(colors[0]).setFill(); path.fill();
                color(colors[1]).setStroke(); path.setLineWidth(0.5); path.stroke();
            }
            for view in self.subviews().iter() {
                let Some(button) = view.downcast_ref::<NSButton>() else { continue; };
                if button.isHidden() || button.tag() > 22 || (button.state() == 0 && button.tag() != 22) { continue; }
                let frame = button.frame();
                let path = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(frame, 4.0 * scale, 4.0 * scale);
                if button.tag() == 22 { color(colors[3]).setFill(); }
                else { color(colors[4]).setFill(); }
                path.fill();
                if button.tag() != 22 { color(colors[3]).colorWithAlphaComponent(0.4).setStroke(); path.setLineWidth(0.5); path.stroke(); }
            }
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
            let action = if *action == "save" && shift { "quick-save" }
                else if state.active_tool.as_deref() == Some(action) { "select" } else { action };
            drop(state);
            (self.ivars().dispatch)(CaptureEditorControlAction::Command(action));
        }
        #[unsafe(method(openToolGroup:))]
        fn open_group(&self, sender: &NSButton) {
            let start = if sender.tag() == 30 { 1 } else { 3 };
            let menu = NSMenu::new(self.mtm());
            let active = self.ivars().presentation.borrow().active_tool.clone();
            for index in start..start+2 {
                let (action, title, _symbol) = TOOLS[index];
                let item = unsafe { NSMenuItem::initWithTitle_action_keyEquivalent(NSMenuItem::alloc(self.mtm()), &NSString::from_str(title), Some(sel!(chooseTool:)), &NSString::new()) };
                item.setTag(index as isize);
                item.setState(isize::from(active.as_deref() == Some(action)));
                unsafe { item.setTarget(Some(self)); }
                item.setImage(toolbar_icon(action).as_deref());
                menu.addItem(&item);
            }
            let frame = sender.frame();
            menu.popUpMenuPositioningItem_atLocation_inView(None, NSPoint::new(frame.origin.x, frame.origin.y + frame.size.height), Some(self));
        }
        #[unsafe(method(chooseTool:))]
        fn choose_tool(&self, sender: &NSMenuItem) {
            if self.ivars().presentation.borrow().busy { return; }
            if let Some((action, _, _)) = TOOLS.get(sender.tag() as usize) {
                (self.ivars().dispatch)(CaptureEditorControlAction::Command(action));
            }
        }
    }
);

struct Toolbar {
    parent: Retained<NSWindow>,
    window: Retained<ToolbarPanel>,
    view: Retained<ToolbarView>,
    buttons: Vec<Retained<NSButton>>,
    groups: Vec<Retained<NSButton>>,
    styles: super::editor_style_macos::StyleControls,
}

thread_local! {
    static TOOLBAR: RefCell<Option<Toolbar>> = const { RefCell::new(None) };
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}

fn create(
    id: &str,
    parent: Retained<NSWindow>,
    presentation: &CaptureEditorToolbarPresentation,
    dispatch: Dispatch,
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
    window.setHasShadow(true);
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
        dispatch: dispatch.clone(),
        session_id: id.into(),
        presentation: RefCell::new(presentation.clone()),
    });
    let view: Retained<ToolbarView> = unsafe { msg_send![super(view), initWithFrame: frame] };
    let buttons = TOOLS
        .iter()
        .chain(COMMANDS)
        .enumerate()
        .map(|(index, (action, title, _symbol))| {
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
            if let Some(image) = toolbar_icon(action) {
                button.setImage(Some(&image));
                button.setImagePosition(NSCellImagePosition::ImageOnly);
                button.setBordered(false);
            }
            view.addSubview(&button);
            button
        })
        .collect();
    let styles =
        super::editor_style_macos::StyleControls::new(&view, &presentation.styling, dispatch);
    let groups = [30, 31]
        .into_iter()
        .map(|tag| {
            let button = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str("⌄"),
                    Some(&view),
                    Some(sel!(openToolGroup:)),
                    mtm,
                )
            };
            button.setTag(tag);
            button.setBordered(false);
            button.setToolTip(Some(&NSString::from_str(if tag == 30 {
                "形状"
            } else {
                "箭头和直线"
            })));
            view.addSubview(&button);
            button
        })
        .collect();
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
        groups,
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
        let compact = frame.width / scale >= 460.0;
        let main_height = if compact { 68.0 } else { 100.0 };
        let shape = if presentation.active_tool.as_deref() == Some("ellipse") {
            2
        } else {
            1
        };
        let arrow = if presentation.active_tool.as_deref() == Some("line") {
            4
        } else {
            3
        };
        let colors = palette(presentation);
        self.buttons[shape].setImage(toolbar_icon("shapes").as_deref());
        self.buttons[arrow].setImage(toolbar_icon("arrow").as_deref());
        for (index, (action, _, _)) in TOOLS.iter().chain(COMMANDS).enumerate() {
            let button = &self.buttons[index];
            button.setHidden(index == 1 + 2 - shape || index == 3 + 4 - arrow);
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
                index < TOOLS.len() && presentation.active_tool.as_deref() == Some(action)
            };
            button.setState(isize::from(active));
            let tint = if *action == "copy" {
                color(colors[5])
            } else if active {
                color(colors[3])
            } else {
                color(colors[2])
            };
            button.setContentTintColor(Some(&tint));
        }
        let mut x = 6.0;
        for index in [0, shape, arrow, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14] {
            let slot = if index == shape || index == arrow {
                42.0
            } else {
                30.0
            };
            self.buttons[index].setFrame(rect(x * scale, 4.0 * scale, 28.0 * scale, 28.0 * scale));
            if index == shape || index == arrow {
                let group = &self.groups[usize::from(index == arrow)];
                group.setFrame(rect(
                    (x + 28.0) * scale,
                    4.0 * scale,
                    14.0 * scale,
                    28.0 * scale,
                ));
                group.setEnabled(!presentation.busy);
                group.setFont(Some(&NSFont::systemFontOfSize(10.0 * scale)));
            }
            x += slot;
        }
        let labels = [
            (16, "ESC", 38.0),
            (17, "OCR", 38.0),
            (18, "长截图", 54.0),
            (15, "清空", 40.0),
            (19, "", 30.0),
            (20, "", 30.0),
            (21, "", 30.0),
            (22, "", 30.0),
        ];
        let mut command_x = if compact { 194.0 } else { 6.0 };
        let command_y = if compact { 36.0 } else { 68.0 };
        for (index, title, width) in labels {
            let button = &self.buttons[index];
            if !title.is_empty() {
                button.setTitle(&NSString::from_str(title));
                button.setImage(None);
                button.setBordered(false);
            }
            button.setFrame(rect(
                command_x * scale,
                command_y * scale,
                (width - 2.0) * scale,
                28.0 * scale,
            ));
            command_x += width;
        }
        self.styles
            .root()
            .setFrame(rect(0.0, 0.0, frame.width, frame.height));
        self.styles.update(
            &presentation.styling,
            presentation.busy,
            scale,
            main_height,
            palette(presentation),
        );
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
        self.styles.close_palette();
        self.window.orderOut(None);
        self.parent.removeChildWindow(&self.window);
        self.window.setContentView(None);
        self.window.close();
    }
}

pub(crate) fn update(
    id: &str,
    parent: Retained<NSWindow>,
    presentation: Option<&CaptureEditorToolbarPresentation>,
    dispatch: Dispatch,
) -> Result<bool, String> {
    MainThreadMarker::new().ok_or("Editor toolbar must update on the main thread")?;
    let Some(presentation) = presentation else {
        close_session(id);
        return Ok(true);
    };
    TOOLBAR.with(|toolbar| {
        let mut toolbar = toolbar.borrow_mut();
        if toolbar.as_ref().is_some_and(|current| {
            current.view.ivars().session_id != id || !std::ptr::eq(&*current.parent, &*parent)
        }) {
            toolbar.take().unwrap().close();
        }
        let current = toolbar.get_or_insert_with(|| create(id, parent, presentation, dispatch));
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

pub(crate) fn dismiss_palette(id: &str) -> bool {
    let styles = TOOLBAR.with(|toolbar| {
        toolbar
            .borrow()
            .as_ref()
            .filter(|toolbar| toolbar.view.ivars().session_id == id)
            .map(|toolbar| toolbar.styles.clone())
    });
    styles.is_some_and(|styles| styles.dismiss_palette())
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

#[cfg(test)]
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
    let button = TOOLBAR.with(|toolbar| {
        let toolbar = toolbar.borrow();
        let toolbar = toolbar.as_ref().expect("Native editor toolbar is not open");
        let index = TOOLS
            .iter()
            .chain(COMMANDS)
            .position(|(id, _, _)| *id == action)
            .expect("Unknown toolbar button");
        toolbar.buttons[index].clone()
    });
    unsafe {
        button.performClick(None);
    }
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
pub(crate) fn test_geometry() -> serde_json::Value {
    TOOLBAR.with_borrow(|toolbar| {
        let toolbar=toolbar.as_ref().unwrap();
        let frame=|view:&NSView|{let r=view.frame();serde_json::json!([r.origin.x,r.origin.y,r.size.width,r.size.height])};
        let mut controls=toolbar.buttons.iter().enumerate().filter(|(_,button)|!button.isHidden()).map(|(index,button)|serde_json::json!({"target":TOOLS.iter().chain(COMMANDS).nth(index).unwrap().0,"frame":frame(button),"enabled":button.isEnabled(),"selected":button.state()!=0})).collect::<Vec<_>>();
        controls.extend(toolbar.groups.iter().zip(["shape-menu","arrow-menu"]).map(|(button,target)|serde_json::json!({"target":target,"frame":frame(button),"enabled":button.isEnabled(),"selected":false})));
        let state=toolbar.view.ivars().presentation.borrow();
        serde_json::json!({"size":[toolbar.view.bounds().size.width,toolbar.view.bounds().size.height],"controls":controls,"style":toolbar.styles.test_geometry(),"tool":state.active_tool,"scale":state.ui_scale,"theme":state.interface_theme,"dark":state.dark})
    })
}

#[cfg(test)]
pub(crate) fn test_styles<T>(
    action: impl FnOnce(&super::editor_style_macos::StyleControls) -> T,
) -> T {
    let styles = TOOLBAR.with(|toolbar| toolbar.borrow().as_ref().unwrap().styles.clone());
    action(&styles)
}
