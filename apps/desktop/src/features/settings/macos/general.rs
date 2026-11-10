use std::cell::{Cell, RefCell};
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAlert, NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua,
    NSAppearanceNameDarkAqua, NSBezierPath, NSBorderType, NSButton, NSButtonType,
    NSControlTextEditingDelegate, NSFont, NSFontWeightMedium, NSFontWeightRegular,
    NSFontWeightSemibold, NSMenu, NSMenuItem, NSScrollView, NSScrollerStyle, NSTextAlignment,
    NSTextField, NSTextFieldDelegate, NSView, NSViewBoundsDidChangeNotification,
};
use objc2_foundation::{
    NSEdgeInsets, NSNotification, NSNotificationCenter, NSObjectProtocol, NSPoint, NSRange, NSRect,
    NSSize, NSString,
};

use crate::integrations::system::native_theme;
use crate::integrations::system::native_theme::Palette;
use snaplingo_core::permissions::SystemPermission;
use snaplingo_core::settings::model::GeneralSettings;
use snaplingo_core::settings::settings_ui_text as t;
use snaplingo_core::settings::GeneralField;
use snaplingo_core::settings::GeneralSection;
use snaplingo_core::settings::GeneralValue;

use super::{GeneralAction, GeneralProjection, UpdatePhase};

type Dispatch = Rc<dyn Fn(GeneralAction) -> Result<Option<GeneralProjection>, String>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Style {
    Native,
    Studio,
    Paper,
}
impl Style {
    fn from(general: &GeneralSettings) -> Self {
        match general.interface_theme.as_str() {
            "studio" => Self::Studio,
            "paper" => Self::Paper,
            _ => Self::Native,
        }
    }
    fn row_height(self) -> f64 {
        match self {
            Self::Native => 62.0,
            Self::Studio => 53.0,
            Self::Paper => 67.0,
        }
    }
    fn card_gap(self) -> f64 {
        match self {
            Self::Native => 20.0,
            Self::Studio => 15.0,
            Self::Paper => 25.0,
        }
    }
    fn heading_height(self) -> f64 {
        match self {
            Self::Native => 55.0,
            Self::Studio => 46.0,
            Self::Paper => 65.0,
        }
    }
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(x, y),
        NSSize::new(width.max(0.0), height.max(0.0)),
    )
}
fn fill(frame: NSRect, radius: f64, color: [u8; 3]) {
    native_theme::color(color).setFill();
    NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(frame, radius, radius).fill();
}
fn outline(frame: NSRect, radius: f64, color: [u8; 3], width: f64) {
    native_theme::color(color).setStroke();
    let path = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(frame, radius, radius);
    path.setLineWidth(width);
    path.stroke();
}

// These miniature illustrations use the fixed thumbnail colors in themes.css,
// rather than changing with the selected appearance of the application.
fn draw_theme_preview(preview: NSRect, id: &str) {
    let (canvas, line, sidebar, accent, card) = match id {
        "studio" => (
            [37, 41, 44],
            [64, 71, 75],
            [25, 29, 32],
            [215, 167, 109],
            [43, 48, 52],
        ),
        "paper" => (
            [247, 247, 239],
            [220, 225, 212],
            [240, 242, 232],
            [83, 115, 92],
            [247, 247, 239],
        ),
        _ => (
            [248, 249, 252],
            [220, 226, 235],
            [234, 240, 247],
            [57, 117, 220],
            [255, 255, 255],
        ),
    };
    let blend = |foreground: [u8; 3], background: [u8; 3], alpha: f64| {
        std::array::from_fn(|i| {
            (foreground[i] as f64 * alpha + background[i] as f64 * (1.0 - alpha)).round() as u8
        })
    };
    let x = preview.origin.x;
    let y = preview.origin.y;
    let w = preview.size.width;
    let h = preview.size.height;
    fill(preview, 6.0, canvas);
    outline(preview, 6.0, line, 1.0);
    fill(rect(x, y + h * 0.15, w, 1.0), 0.0, line);
    for i in 0..3 {
        fill(
            rect(x + 5.0 + i as f64 * 6.0, y + 5.0, 3.0, 3.0),
            1.5,
            blend(accent, canvas, 0.45),
        );
    }
    let (left, top, content_width) = if id == "paper" {
        fill(rect(x, y + h * 0.15, w, h * 0.16), 0.0, sidebar);
        for i in 0..4 {
            fill(
                rect(
                    x + w * (0.09 + i as f64 * 0.22),
                    y + h * 0.23,
                    w * 0.15,
                    2.0,
                ),
                0.0,
                blend(accent, sidebar, if i == 1 { 0.85 } else { 0.18 }),
            );
        }
        (x + w * 0.13, y + h * 0.41, w * 0.74)
    } else {
        let nav = if id == "studio" { 0.16 } else { 0.25 };
        fill(rect(x, y + h * 0.15, w * nav, h * 0.85), 0.0, sidebar);
        for i in 0..4 {
            fill(
                rect(
                    x + w * 0.04,
                    y + h * 0.15 + w * if id == "studio" { 0.08 } else { 0.12 } + i as f64 * 9.0,
                    w * (nav - 0.08),
                    3.0,
                ),
                0.0,
                blend(accent, sidebar, if i == 1 { 0.85 } else { 0.18 }),
            );
        }
        (
            x + w * if id == "studio" { 0.24 } else { 0.32 },
            y + h * 0.25,
            w * if id == "studio" { 0.69 } else { 0.61 },
        )
    };
    fill(
        rect(left, top, content_width * 0.29, h * 0.04),
        0.0,
        blend(accent, canvas, 0.55),
    );
    let card_top = top + h * 0.13;
    let card_height = ((y + h * 0.93 - card_top) - 12.0) / 3.0;
    for i in 0..3 {
        let frame = rect(
            left,
            card_top + i as f64 * (card_height + 6.0),
            content_width,
            card_height,
        );
        if id == "paper" {
            fill(
                rect(
                    frame.origin.x,
                    frame.origin.y + frame.size.height - 1.0,
                    frame.size.width,
                    1.0,
                ),
                0.0,
                line,
            );
        } else {
            fill(frame, if id == "studio" { 2.0 } else { 3.0 }, card);
            outline(frame, if id == "studio" { 2.0 } else { 3.0 }, line, 1.0);
        }
    }
}

struct SurfaceIvars {
    palette: Cell<Palette>,
    rounded: Cell<f64>,
    border: Cell<bool>,
    lines: RefCell<Vec<(f64, f64)>>,
    status_dot: Cell<Option<NSPoint>>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoSettingsSurface"]
    #[thread_kind = MainThreadOnly]
    #[ivars = SurfaceIvars]
    struct Surface;
    unsafe impl NSObjectProtocol for Surface {}
    impl Surface {
        #[unsafe(method(isFlipped))] fn is_flipped(&self) -> bool { true }
        #[unsafe(method(drawRect:))]
        fn draw(&self, _: NSRect) {
            let p = self.ivars().palette.get();
            fill(self.bounds(), self.ivars().rounded.get(), p.surface);
            if self.ivars().border.get() { outline(self.bounds(), self.ivars().rounded.get(), p.line, 1.0); }
            for &(y, inset) in self.ivars().lines.borrow().iter() { fill(rect(inset, y, self.bounds().size.width - inset * 2.0, 1.0), 0.0, p.line); }
            if let Some(dot)=self.ivars().status_dot.get() {fill(rect(dot.x-3.0,dot.y-3.0,12.0,12.0),6.0,p.good_soft);fill(rect(dot.x,dot.y,6.0,6.0),3.0,p.good);}
        }
    }
);

#[derive(Clone)]
enum ButtonKind {
    Plain,
    Choice,
    Toggle,
    Tab,
    Danger,
    Primary,
    Disclosure(bool),
    Theme(&'static str),
    Stepper(bool),
}
struct ButtonIvars {
    palette: Cell<Palette>,
    kind: ButtonKind,
    selected: Cell<bool>,
    underline: Cell<bool>,
    label: RefCell<Option<Retained<NSTextField>>>,
}
define_class!(
    #[unsafe(super(NSButton))]
    #[name = "SnapLingoSettingsButton"]
    #[thread_kind = MainThreadOnly]
    #[ivars = ButtonIvars]
    struct Button;
    unsafe impl NSObjectProtocol for Button {}
    impl Button {
        #[unsafe(method(isFlipped))] fn is_flipped(&self) -> bool { true }
        #[unsafe(method(acceptsFirstResponder))] fn accepts_first_responder(&self) -> bool { true }
        #[unsafe(method(wantsDefaultClipping))] fn wants_default_clipping(&self) -> bool { !matches!(self.ivars().kind, ButtonKind::Theme(_)) }
        #[unsafe(method(drawRect:))]
        fn draw(&self, _: NSRect) {
            let state = self.ivars(); let p = state.palette.get(); let b = self.bounds();
            match state.kind {
                ButtonKind::Toggle => {
                    let on = self.state() != 0;
                    fill(b, 10.0, if on { p.accent } else { p.line_strong });
                    fill(rect(if on { 18.0 } else { 2.0 }, 2.0, 16.0, 16.0), 8.0, p.surface);
                }
                ButtonKind::Tab => {
                    if state.selected.get() {
                        if state.underline.get() { fill(rect(0.0, b.size.height - 2.0, b.size.width, 2.0), 0.0, p.accent); }
                        else { fill(b, 5.0, p.accent_soft); }
                    }
                }
                ButtonKind::Theme(id) => {
                    if state.selected.get() { outline(rect(-1.0, -1.0, b.size.width + 2.0, b.size.height + 2.0), 11.0, p.accent_line, 2.0); }
                    fill(b, 10.0, p.surface);
                    outline(rect(0.5, 0.5, b.size.width - 1.0, b.size.height - 1.0), 10.0, if state.selected.get() { p.accent } else { p.line }, 1.0);
                    let preview_width = if b.size.height < 100.0 {105.0} else {b.size.width - 18.0};
                    let preview = rect(9.0, 9.0, preview_width, preview_width / 1.6);
                    draw_theme_preview(preview, id);
                    if state.selected.get() {
                        native_theme::color(p.accent).setStroke(); let check = NSBezierPath::bezierPath();
                        check.moveToPoint(NSPoint::new(b.size.width - 22.0, b.size.height - 39.0)); check.lineToPoint(NSPoint::new(b.size.width - 19.0, b.size.height - 36.0)); check.lineToPoint(NSPoint::new(b.size.width - 14.0, b.size.height - 43.0)); check.setLineWidth(1.5); check.stroke();
                    }
                }
                ButtonKind::Disclosure(open) => {
                    native_theme::color(p.muted).setStroke(); let path = NSBezierPath::bezierPath();
                    path.moveToPoint(NSPoint::new(1.0, 9.0)); path.lineToPoint(NSPoint::new(if open {5.0} else {5.0}, if open {13.0} else {9.0})); path.lineToPoint(NSPoint::new(9.0, if open {9.0} else {5.0})); path.setLineWidth(1.0); path.stroke();
                }
                ButtonKind::Danger | ButtonKind::Primary => {
                    let primary = matches!(state.kind, ButtonKind::Primary);
                    fill(b, 8.0, if primary {p.accent} else {[254,242,242]});
                    if !primary {outline(rect(0.5,0.5,b.size.width-1.0,b.size.height-1.0),8.0,[254,202,202],1.0);}
                }
                ButtonKind::Stepper(up) => {
                    native_theme::color(p.muted).setStroke();
                    let path = NSBezierPath::bezierPath();
                    path.moveToPoint(NSPoint::new(7.0, if up { 12.0 } else { 7.0 }));
                    path.lineToPoint(NSPoint::new(12.0, if up { 7.0 } else { 12.0 }));
                    path.lineToPoint(NSPoint::new(17.0, if up { 12.0 } else { 7.0 }));
                    path.setLineWidth(1.5); path.stroke();
                }
                _ => {
                    fill(b, 8.0, if self.isEnabled(){p.surface}else{p.sunken});
                    outline(rect(0.5, 0.5, b.size.width - 1.0, b.size.height - 1.0), 8.0, p.line, 1.0);
                    if matches!(state.kind, ButtonKind::Choice) {
                        native_theme::color(p.muted).setStroke();
                        let path = NSBezierPath::bezierPath(); let x = b.size.width - 20.0; let y = b.size.height * 0.5;
                        path.moveToPoint(NSPoint::new(x - 6.0, y - 3.0)); path.lineToPoint(NSPoint::new(x, y + 3.0)); path.lineToPoint(NSPoint::new(x + 6.0, y - 3.0)); path.setLineWidth(1.7); path.stroke();
                    }
                }
            }
        }
    }
);

struct Row {
    label: Retained<NSTextField>,
    description: Option<Retained<NSTextField>>,
    control: Control,
    status: Option<Retained<NSTextField>>,
}
#[derive(Clone)]
enum Control {
    Choice(
        GeneralField,
        Retained<Button>,
        &'static [(&'static str, &'static str)],
    ),
    Toggle(GeneralField, Retained<Button>),
    Text(GeneralField, Retained<NSTextField>, Retained<Surface>),
    Number(
        GeneralField,
        Retained<NSTextField>,
        Retained<Surface>,
        Retained<Button>,
        Retained<Button>,
        u32,
        u32,
        u32,
        &'static str,
    ),
    Value(Retained<NSTextField>),
    Actions(Vec<(GeneralAction, Retained<Button>)>),
    Paragraph(Retained<NSTextField>),
    Disclosure(Retained<Button>),
    Disabled(Retained<Button>),
    Logs(
        Retained<NSScrollView>,
        Retained<Surface>,
        Vec<Retained<NSTextField>>,
    ),
}
struct Card {
    section: GeneralSection,
    surface: Retained<Surface>,
    heading: Retained<NSTextField>,
    rows: Vec<Row>,
    themes: Vec<Retained<Button>>,
    extra: Vec<Retained<NSTextField>>,
    number: Retained<NSTextField>,
}
struct Contents {
    scroll: Retained<NSScrollView>,
    document: Retained<Surface>,
    header: Retained<Surface>,
    cards: Vec<Card>,
    tabs: Vec<Retained<Button>>,
    tab_numbers: Vec<Retained<NSTextField>>,
    title: Retained<NSTextField>,
    status: Retained<NSTextField>,
    directory: Retained<Surface>,
    directory_title: Retained<NSTextField>,
    directory_caption: Retained<NSTextField>,
    directory_status: Retained<NSTextField>,
}
pub(crate) struct GeneralIvars {
    projection: RefCell<GeneralProjection>,
    dispatch: Dispatch,
    contents: RefCell<Option<Contents>>,
    updating: Cell<bool>,
    layouting: Cell<bool>,
    shell_width: Cell<Option<f64>>,
    viewport_width: Cell<Option<f64>>,
    pending_rebuild: Cell<bool>,
    navigation_offset: Cell<Option<f64>>,
    recovery_open: Cell<bool>,
    release_notes_open: Cell<bool>,
    active: Cell<GeneralSection>,
    section_offsets: RefCell<Vec<f64>>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoGeneralSettingsView"]
    #[thread_kind = MainThreadOnly]
    #[ivars = GeneralIvars]
    pub(crate) struct GeneralView;
    unsafe impl NSObjectProtocol for GeneralView {}
    unsafe impl NSControlTextEditingDelegate for GeneralView {
        #[unsafe(method(controlTextDidChange:))]
        fn field_changed(&self, notification: &NSNotification) {
            if self.ivars().updating.get() { return; }
            if let Some(field) = notification.object().and_then(|object| object.downcast::<NSTextField>().ok()) {
                let mut number = false;
                self.control(field.tag(), |control| number = matches!(control, Control::Number(..)));
                if number { self.commit_field(&field); }
            }
        }
        #[unsafe(method(controlTextDidEndEditing:))]
        fn field_ended(&self, notification: &NSNotification) {
            if self.ivars().updating.get() { return; }
            if let Some(field) = notification.object().and_then(|object| object.downcast::<NSTextField>().ok()) { self.commit_field(&field); }
            // The shared field editor detaches after this notification returns.
            // Deliver the structural refresh on the next main run-loop turn.
            unsafe { let _: () = msg_send![self, performSelector: sel!(finishPendingRefresh), withObject: std::ptr::null::<objc2::runtime::AnyObject>(), afterDelay: 0.0f64]; }
        }
    }
    unsafe impl NSTextFieldDelegate for GeneralView {}
    impl GeneralView {
        #[unsafe(method(isFlipped))] fn is_flipped(&self) -> bool { true }
        #[unsafe(method(drawRect:))]
        fn draw(&self, _: NSRect) {
            let p = self.palette(); fill(self.bounds(), 0.0, p.canvas);
            if let Some(c) = self.ivars().contents.borrow().as_ref() {
                let frame = c.status.frame(); fill(rect(frame.origin.x - 15.0, frame.origin.y + 4.0, 12.0, 12.0), 6.0, p.good_soft); fill(rect(frame.origin.x - 12.0, frame.origin.y + 7.0, 6.0, 6.0), 3.0, p.good);
                if !c.directory.isHidden() {
                    let x = c.directory_status.frame().origin.x - 12.0;
                    fill(rect(x, c.directory_status.frame().origin.y + 5.0, 5.0, 5.0), 2.5, p.good);
                    if Style::from(&self.ivars().projection.borrow().general) == Style::Studio { fill(rect(c.directory.frame().size.width - 1.0, 0.0, 1.0, self.bounds().size.height), 0.0, p.line); }
                }
            }
        }
        #[unsafe(method(setFrameSize:))]
        fn resized(&self, size: NSSize) { unsafe { let _: () = msg_send![super(self), setFrameSize: size]; } self.reflow(); }
        #[unsafe(method(performAction:))]
        fn button_action(&self, sender: &Button) { self.perform(sender.tag()); }
        #[unsafe(method(selectChoice:))]
        fn select_choice(&self, item: &NSMenuItem) { self.select(item.tag()); }
        #[unsafe(method(finishPendingRefresh))]
        fn finish_pending_refresh(&self) {
            if !self.has_editor() && self.ivars().pending_rebuild.replace(false) { self.rebuild(); }
        }
        #[unsafe(method(scrollChanged:))]
        fn scroll_changed(&self, _: &NSNotification) { self.did_scroll(); }
    }
);

const APPEARANCE: &[(&str, &str)] = &[("system", "跟随系统"), ("light", "浅色"), ("dark", "深色")];
const LANGUAGE: &[(&str, &str)] = &[("zh-CN", "中文简体")];
const PROXY: &[(&str, &str)] = &[
    ("system", "跟随系统"),
    ("none", "不使用代理"),
    ("manual", "手动代理"),
];
const LOG_LEVEL: &[(&str, &str)] = &[
    ("debug", "Debug"),
    ("info", "Info"),
    ("warn", "Warn"),
    ("error", "Error"),
];
const THEMES: &[(&str, &str, &str, &str)] = &[
    ("native", "澄明", "Native", "清透蓝调 · 经典侧栏"),
    ("studio", "夜航", "Studio", "琥珀强调 · 紧凑工作台"),
    ("paper", "松间", "Paper", "暖纸松绿 · 舒展阅读"),
];

fn surface(parent: &NSView, p: Palette, radius: f64, border: bool) -> Retained<Surface> {
    let view = Surface::alloc(parent.mtm()).set_ivars(SurfaceIvars {
        palette: Cell::new(p),
        rounded: Cell::new(radius),
        border: Cell::new(border),
        lines: RefCell::new(Vec::new()),
        status_dot: Cell::new(None),
    });
    let view: Retained<Surface> =
        unsafe { msg_send![super(view), initWithFrame: rect(0.0, 0.0, 1.0, 1.0)] };
    parent.addSubview(&view);
    view
}
fn label(parent: &NSView, title: &str, size: f64, p: Palette) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&NSString::from_str(title), parent.mtm());
    field.setFont(Some(&NSFont::systemFontOfSize_weight(size, unsafe {
        NSFontWeightRegular
    })));
    field.setTextColor(Some(&native_theme::color(p.ink)));
    field.setSelectable(false);
    parent.addSubview(&field);
    field
}
fn button(
    parent: &NSView,
    root: &GeneralView,
    title: &str,
    kind: ButtonKind,
    tag: isize,
) -> Retained<Button> {
    let p = root.palette();
    let button = Button::alloc(parent.mtm()).set_ivars(ButtonIvars {
        palette: Cell::new(p),
        kind: kind.clone(),
        selected: Cell::new(false),
        underline: Cell::new(false),
        label: RefCell::new(None),
    });
    let button: Retained<Button> =
        unsafe { msg_send![super(button), initWithFrame: rect(0.0, 0.0, 1.0, 1.0)] };
    button.setTitle(&NSString::from_str(title));
    button.setBordered(false);
    button.setTag(tag);
    if matches!(kind, ButtonKind::Toggle) {
        button.setButtonType(NSButtonType::Switch);
    }
    unsafe {
        button.setTarget(Some(root));
        button.setAction(Some(sel!(performAction:)));
    }
    if !matches!(
        kind,
        ButtonKind::Toggle | ButtonKind::Theme(_) | ButtonKind::Stepper(_)
    ) {
        let title = label(
            &button,
            title,
            if matches!(kind, ButtonKind::Choice) {
                12.5
            } else {
                10.5
            },
            p,
        );
        title.setFont(Some(&NSFont::systemFontOfSize_weight(
            if matches!(kind, ButtonKind::Choice) {
                12.5
            } else {
                10.5
            },
            unsafe {
                if matches!(kind, ButtonKind::Choice) {
                    NSFontWeightRegular
                } else {
                    NSFontWeightMedium
                }
            },
        )));
        title.setTextColor(Some(&native_theme::color(match kind {
            ButtonKind::Primary => p.accent_on,
            ButtonKind::Danger => [220, 38, 38],
            ButtonKind::Plain => p.muted,
            _ => p.ink,
        })));
        title.setAlignment(
            if matches!(kind, ButtonKind::Choice | ButtonKind::Disclosure(_)) {
                NSTextAlignment::Left
            } else {
                NSTextAlignment::Center
            },
        );
        *button.ivars().label.borrow_mut() = Some(title);
    }
    parent.addSubview(&button);
    button
}
fn text_field(parent: &NSView, root: &GeneralView, tag: isize) -> Retained<NSTextField> {
    let field = NSTextField::initWithFrame(parent.mtm().alloc(), rect(0.0, 0.0, 1.0, 1.0));
    field.setBezeled(false);
    field.setBordered(false);
    field.setDrawsBackground(false);
    field.setFont(Some(&NSFont::systemFontOfSize(12.5)));
    field.setTextColor(Some(&native_theme::color(root.palette().ink)));
    field.setTag(tag);
    unsafe {
        field.setDelegate(Some(ProtocolObject::from_ref(root)));
    }
    parent.addSubview(&field);
    field
}
fn set_button_frame(button: &Button, frame: NSRect) {
    button.setFrame(frame);
    if let Some(label) = button.ivars().label.borrow().as_ref() {
        if !button.isEnabled() {
            label.setTextColor(Some(&native_theme::color(
                button.ivars().palette.get().faint,
            )));
        }
        label.setFrame(rect(
            12.0,
            (frame.size.height - 18.0) * 0.5,
            frame.size.width - 24.0,
            18.0,
        ));
    }
}

impl GeneralView {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        projection: GeneralProjection,
        dispatch: Dispatch,
    ) -> Retained<Self> {
        let view = Self::alloc(mtm).set_ivars(GeneralIvars {
            projection: RefCell::new(projection),
            dispatch,
            contents: RefCell::new(None),
            updating: Cell::new(false),
            layouting: Cell::new(false),
            shell_width: Cell::new(None),
            viewport_width: Cell::new(None),
            pending_rebuild: Cell::new(false),
            navigation_offset: Cell::new(None),
            recovery_open: Cell::new(false),
            release_notes_open: Cell::new(false),
            active: Cell::new(GeneralSection::Interface),
            section_offsets: RefCell::new(Vec::new()),
        });
        let view: Retained<Self> =
            unsafe { msg_send![super(view), initWithFrame: rect(0.0, 0.0, 698.0, 650.0)] };
        view.rebuild();
        view
    }
    fn palette(&self) -> Palette {
        let data = self.ivars().projection.borrow();
        native_theme::palette(
            &data.general.interface_theme,
            native_theme::is_dark(&data.general.theme),
        )
    }
    pub(crate) fn set_layout_widths(&self, shell: f64, viewport: f64) {
        self.ivars().shell_width.set(Some(shell));
        self.ivars().viewport_width.set(Some(viewport));
        self.reflow();
    }
    fn shell_width(&self) -> f64 {
        self.ivars().shell_width.get().unwrap_or_else(|| {
            self.bounds().size.width
                + match Style::from(&self.ivars().projection.borrow().general) {
                    Style::Native => 202.0,
                    Style::Studio => 68.0,
                    Style::Paper => 0.0,
                }
        })
    }
    pub(crate) fn refresh(&self, next: GeneralProjection) {
        let old = self.ivars().projection.borrow();
        let rebuild = old.general.interface_theme != next.general.interface_theme
            || old.general.theme != next.general.theme
            || old.general.proxy_mode != next.general.proxy_mode
            || serde_json::to_value(&old.logs).unwrap()
                != serde_json::to_value(&next.logs).unwrap()
            || old.permissions != next.permissions
            || old.update_phase != next.update_phase
            || serde_json::to_value(&old.update).unwrap()
                != serde_json::to_value(&next.update).unwrap()
            || serde_json::to_value(&old.permission_context).unwrap()
                != serde_json::to_value(&next.permission_context).unwrap()
            || old.downloaded_path != next.downloaded_path;
        drop(old);
        *self.ivars().projection.borrow_mut() = next;
        self.ivars()
            .pending_rebuild
            .set(self.ivars().pending_rebuild.get() || rebuild);
        if self.has_editor() {
            // Keep the actual editor, its draft, selection and marked text alive
            // while background log/update/permission snapshots arrive.
            self.sync();
        } else if self.ivars().pending_rebuild.replace(false) {
            self.rebuild();
        } else {
            self.sync();
            self.reflow();
        }
    }
    fn has_editor(&self) -> bool {
        self.ivars().contents.borrow().as_ref().is_some_and(|c| {
            c.cards
                .iter()
                .flat_map(|card| &card.rows)
                .any(|row| match &row.control {
                    Control::Text(_, field, _) | Control::Number(_, field, ..) => {
                        field.currentEditor().is_some()
                    }
                    _ => false,
                })
        })
    }
    fn dispatch(&self, action: GeneralAction) {
        let result = (self.ivars().dispatch)(action);
        match result {
            Ok(Some(projection)) => self.refresh(projection),
            Ok(None) => {}
            Err(error) => {
                self.ivars().projection.borrow_mut().error = Some(error);
                self.sync();
            }
        }
    }
    fn rebuild(&self) {
        self.ivars().updating.set(true);
        unsafe {
            NSNotificationCenter::defaultCenter().removeObserver_name_object(
                self,
                Some(NSViewBoundsDidChangeNotification),
                None,
            );
        }
        let top = self
            .ivars()
            .contents
            .borrow()
            .as_ref()
            .map(|c| c.scroll.contentView().bounds().origin.y)
            .unwrap_or(0.0);
        for child in self.subviews() {
            child.removeFromSuperview();
        }
        let p = self.palette();
        let dark = native_theme::is_dark(&self.ivars().projection.borrow().general.theme);
        self.setAppearance(
            NSAppearance::appearanceNamed(unsafe {
                if dark {
                    NSAppearanceNameDarkAqua
                } else {
                    NSAppearanceNameAqua
                }
            })
            .as_deref(),
        );
        let scroll = NSScrollView::initWithFrame(self.mtm().alloc(), self.bounds());
        scroll.setHasVerticalScroller(true);
        scroll.setAutomaticallyAdjustsContentInsets(false);
        scroll.setContentInsets(NSEdgeInsets {
            top: 0.0,
            left: 0.0,
            bottom: 0.0,
            right: 0.0,
        });
        scroll.setScrollerStyle(NSScrollerStyle::Overlay);
        scroll.setBorderType(NSBorderType::NoBorder);
        scroll.setDrawsBackground(false);
        self.addSubview(&scroll);
        scroll
            .contentView()
            .setPostsBoundsChangedNotifications(true);
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                self,
                sel!(scrollChanged:),
                Some(NSViewBoundsDidChangeNotification),
                Some(&scroll.contentView()),
            );
        }
        let mut dp = p;
        dp.surface = p.canvas;
        let document = surface(self, dp, 0.0, false);
        document.removeFromSuperview();
        scroll.setDocumentView(Some(&document));
        let header = surface(self, dp, 0.0, false);
        let directory = surface(self, dp, 0.0, false);
        let directory_title = label(&directory, t("general"), 15.0, p);
        let directory_caption = label(&directory, "CONTENTS", 9.0, p);
        directory_caption.setTextColor(Some(&native_theme::color(p.faint)));
        let directory_status = label(&directory, t("autoSave"), 10.0, p);
        directory_status.setTextColor(Some(&native_theme::color(p.muted)));
        let title = label(self, t("general"), 28.0, p);
        let status = label(self, t("autoSave"), 10.0, p);
        status.setTextColor(Some(&native_theme::color(p.muted)));
        let mut tabs = Vec::new();
        for (index, section) in GeneralSection::ALL.into_iter().enumerate() {
            tabs.push(button(
                self,
                self,
                section.label(),
                ButtonKind::Tab,
                10_000 + index as isize,
            ));
        }
        let tab_numbers = tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let n = label(tab, &format!("{:02}", i + 1), 10.0, p);
                n.setFont(Some(
                    &NSFont::fontWithName_size(&NSString::from_str("Menlo"), 10.0)
                        .unwrap_or_else(|| NSFont::systemFontOfSize(10.0)),
                ));
                n.setTextColor(Some(&native_theme::color(p.faint)));
                n
            })
            .collect();
        let mut cards = Vec::new();
        for section in GeneralSection::ALL {
            let card = surface(&document, p, 12.0, true);
            let heading = label(&card, section.label(), 13.0, p);
            let mut contents = Card {
                section,
                surface: card.clone(),
                heading,
                rows: Vec::new(),
                themes: Vec::new(),
                extra: Vec::new(),
                number: {
                    let n = label(&card, &format!("{:02}", section as usize + 1), 10.0, p);
                    n.setFont(Some(
                        &NSFont::fontWithName_size(&NSString::from_str("Menlo"), 10.0)
                            .unwrap_or_else(|| NSFont::systemFontOfSize(10.0)),
                    ));
                    n.setTextColor(Some(&native_theme::color(p.faint)));
                    n
                },
            };
            self.populate(&mut contents);
            cards.push(contents);
        }
        *self.ivars().contents.borrow_mut() = Some(Contents {
            scroll,
            document,
            header,
            cards,
            tabs,
            tab_numbers,
            title,
            status,
            directory,
            directory_title,
            directory_caption,
            directory_status,
        });
        self.sync();
        self.reflow();
        if let Some(contents) = self.ivars().contents.borrow().as_ref() {
            contents
                .scroll
                .contentView()
                .scrollToPoint(NSPoint::new(0.0, top));
        }
        self.ivars().updating.set(false);
        self.did_scroll();
        self.setNeedsDisplay(true);
    }
    fn row(
        &self,
        card: &mut Card,
        title: &str,
        description: Option<&str>,
        control: impl FnOnce(&NSView, isize) -> Control,
    ) {
        let p = self.palette();
        let tag = card.section as isize * 100 + card.rows.len() as isize;
        let row = Row {
            label: label(&card.surface, title, 12.0, p),
            description: description.map(|s| {
                let f = label(&card.surface, s, 10.0, p);
                f.setTextColor(Some(&native_theme::color(p.muted)));
                f
            }),
            control: control(&card.surface, tag),
            status: None,
        };
        card.rows.push(row);
    }
    fn choice(
        &self,
        card: &mut Card,
        key: &str,
        field: GeneralField,
        options: &'static [(&'static str, &'static str)],
    ) {
        self.row(card, t(key), None, |parent, tag| {
            Control::Choice(
                field,
                button(parent, self, "", ButtonKind::Choice, tag),
                options,
            )
        });
    }
    fn toggle(&self, card: &mut Card, key: &str, field: GeneralField) {
        self.row(card, t(key), None, |parent, tag| {
            Control::Toggle(field, button(parent, self, t(key), ButtonKind::Toggle, tag))
        });
    }
    fn number(
        &self,
        card: &mut Card,
        key: &str,
        field: GeneralField,
        min: u32,
        max: u32,
        step: u32,
        suffix: &'static str,
    ) {
        self.row(card, t(key), None, |parent, tag| {
            let background = surface(parent, self.palette(), 8.0, true);
            let suffix_label = label(&background, suffix, 14.0, self.palette());
            suffix_label.setTextColor(Some(&native_theme::color(self.palette().muted)));
            Control::Number(
                field,
                text_field(parent, self, tag),
                background,
                button(
                    parent,
                    self,
                    "增加",
                    ButtonKind::Stepper(true),
                    20_000 + tag * 2,
                ),
                button(
                    parent,
                    self,
                    "减少",
                    ButtonKind::Stepper(false),
                    20_001 + tag * 2,
                ),
                min,
                max,
                step,
                suffix,
            )
        });
    }
    fn actions(
        &self,
        card: &mut Card,
        title: &str,
        description: Option<&str>,
        actions: Vec<(GeneralAction, &'static str)>,
    ) {
        self.row(card, title, description, |parent, tag| {
            Control::Actions(
                actions
                    .into_iter()
                    .enumerate()
                    .map(|(index, (action, title))| {
                        (
                            action.clone(),
                            button(
                                parent,
                                self,
                                title,
                                match action {
                                    GeneralAction::ClearLogs => ButtonKind::Danger,
                                    GeneralAction::DownloadUpdate
                                    | GeneralAction::RequestPermission(
                                        SystemPermission::ScreenRecording,
                                    ) => ButtonKind::Primary,
                                    _ => ButtonKind::Plain,
                                },
                                30_000 + tag * 10 + index as isize,
                            ),
                        )
                    })
                    .collect(),
            )
        });
    }
    fn paragraph(&self, card: &mut Card, text: &str, danger: bool) {
        self.row(card, "", None, |parent, _| {
            let field = NSTextField::wrappingLabelWithString(&NSString::from_str(text), self.mtm());
            field.setFont(Some(&NSFont::systemFontOfSize_weight(10.5, unsafe {
                NSFontWeightRegular
            })));
            field.setTextColor(Some(&native_theme::color(if danger {
                self.palette().danger
            } else {
                self.palette().muted
            })));
            field.setSelectable(true);
            parent.addSubview(&field);
            Control::Paragraph(field)
        });
    }
    fn disclosure(&self, card: &mut Card, title: &str, open: bool, tag: isize) {
        self.row(card, "", None, |parent, _| {
            Control::Disclosure(button(
                parent,
                self,
                title,
                ButtonKind::Disclosure(open),
                tag,
            ))
        });
    }
    fn populate(&self, card: &mut Card) {
        let data = self.ivars().projection.borrow();
        let p = self.palette();
        match card.section {
            GeneralSection::Interface => {
                for (i, &(id, name, english, description)) in THEMES.iter().enumerate() {
                    let theme = button(
                        &card.surface,
                        self,
                        name,
                        ButtonKind::Theme(id),
                        40_000 + i as isize,
                    );
                    let name = label(&theme, name, 13.0, p);
                    name.setTag(1);
                    let en = label(&theme, english, 10.0, p);
                    en.setTag(2);
                    en.setTextColor(Some(&native_theme::color(p.faint)));
                    let desc = label(&theme, description, 10.0, p);
                    desc.setTag(3);
                    desc.setTextColor(Some(&native_theme::color(p.muted)));
                    card.themes.push(theme);
                }
                self.choice(card, "interfaceLanguage", GeneralField::Language, LANGUAGE);
                self.choice(card, "appearanceMode", GeneralField::Appearance, APPEARANCE);
                self.toggle(card, "startOnBoot", GeneralField::StartOnBoot);
            }
            GeneralSection::Permissions => {
                if data
                    .permission_context
                    .as_ref()
                    .is_some_and(|c| c.needs_installation)
                {
                    self.paragraph(card,"当前运行的是临时副本。请将 SnapLingo 拖入“应用程序”后打开，再检测是否需要授权。",false);
                }
                for (permission, title, description, granted) in [
                    (
                        SystemPermission::ScreenRecording,
                        "屏幕录制",
                        "截图与屏幕识别",
                        data.permissions.map(|s| s.screen_recording),
                    ),
                    (
                        SystemPermission::Accessibility,
                        "辅助功能",
                        "选中文本与界面元素检测（可选）",
                        data.permissions.map(|s| s.accessibility),
                    ),
                ] {
                    if granted == Some(false) {
                        self.actions(
                            card,
                            title,
                            Some(description),
                            vec![(GeneralAction::RequestPermission(permission), "去授权")],
                        );
                        let row = card.rows.last_mut().unwrap();
                        if let Some(detail) = &row.description {
                            detail.setStringValue(&NSString::from_str(&format!(
                                "{description}\n当前运行的 SnapLingo 尚未获得{title}权限。"
                            )));
                            detail.setUsesSingleLineMode(false);
                            detail.setMaximumNumberOfLines(0);
                        }
                        let status = label(&card.surface, "待授权", 10.5, p);
                        status.setTextColor(Some(&native_theme::color(p.muted)));
                        row.status = Some(status);
                    } else {
                        self.row(card, title, Some(description), |parent, _| {
                            Control::Value(label(
                                parent,
                                if granted == Some(true) {
                                    "已授权"
                                } else {
                                    "检测中…"
                                },
                                10.5,
                                p,
                            ))
                        });
                    }
                }
                if data
                    .permissions
                    .is_some_and(|p| !p.screen_recording || !p.accessibility)
                {
                    self.paragraph(card,"如果你此前已授权，重装或更新时的应用签名变化可能使旧版本的授权未对当前应用生效。仅凭系统检测无法判断是尚未授权，还是旧授权未生效。若系统中已开启，请先重新检测；仅在 macOS 要求时重启应用，仍不可用再重新授权。",false);
                }
                self.disclosure(
                    card,
                    "授权遇到问题？",
                    self.ivars().recovery_open.get(),
                    50_000,
                );
                if self.ivars().recovery_open.get() {
                    self.paragraph(
                        card,
                        "这里检测的是当前运行应用的权限。系统设置有变化时，可先重新检测。",
                        false,
                    );
                    if let Some(path) = data
                        .permission_context
                        .as_ref()
                        .and_then(|c| c.app_path.as_ref())
                    {
                        self.paragraph(card, &format!("当前应用：{path}"), false);
                    }
                    let mut recovery = vec![(GeneralAction::RefreshPermissions, "重新检测")];
                    if data
                        .permission_context
                        .as_ref()
                        .is_some_and(|c| c.can_reset)
                    {
                        if data.permissions.is_some_and(|p| !p.screen_recording) {
                            recovery.push((
                                GeneralAction::ResetPermission(SystemPermission::ScreenRecording),
                                "重新授权屏幕录制",
                            ));
                        }
                        if data.permissions.is_some_and(|p| !p.accessibility) {
                            recovery.push((
                                GeneralAction::ResetPermission(SystemPermission::Accessibility),
                                "重新授权辅助功能",
                            ));
                        }
                    }
                    self.actions(card, "", None, recovery);
                    self.paragraph(card, "如果 macOS 要求，请重启一次。", false);
                    self.actions(
                        card,
                        "",
                        None,
                        vec![(GeneralAction::Restart, "重启 SnapLingo")],
                    );
                }
            }
            GeneralSection::Network => {
                self.choice(card, "proxy", GeneralField::ProxyMode, PROXY);
                if data.general.proxy_mode == "manual" {
                    self.row(card, t("proxyAddress"), None, |parent, tag| {
                        Control::Text(
                            GeneralField::ProxyUrl,
                            text_field(parent, self, tag),
                            surface(parent, p, 8.0, true),
                        )
                    });
                }
                self.number(
                    card,
                    "requestTimeout",
                    GeneralField::RequestTimeout,
                    1_000,
                    120_000,
                    1_000,
                    "ms",
                );
                self.number(card, "retryCount", GeneralField::RetryCount, 0, 5, 1, "");
            }
            GeneralSection::Maintenance => {
                self.choice(card, "logLevel", GeneralField::LogLevel, LOG_LEVEL);
                self.number(
                    card,
                    "logRetentionDays",
                    GeneralField::LogRetention,
                    1,
                    365,
                    1,
                    t("days"),
                );
                self.row(card, t("logStorage"), None, |parent, _| {
                    Control::Value(label(
                        parent,
                        &t("recentLogs").replace("{count}", &data.logs.len().to_string()),
                        12.0,
                        p,
                    ))
                });
                self.actions(
                    card,
                    t("logActions"),
                    None,
                    vec![
                        (GeneralAction::RefreshLogs, t("refreshLogs")),
                        (GeneralAction::ClearLogs, t("clearLogs")),
                    ],
                );
                if !data.logs.is_empty() {
                    self.row(card, "", None, |parent, _| {
                        let scroll = NSScrollView::initWithFrame(
                            self.mtm().alloc(),
                            rect(0.0, 0.0, 1.0, 1.0),
                        );
                        scroll.setHasVerticalScroller(true);
                        scroll.setBorderType(NSBorderType::NoBorder);
                        parent.addSubview(&scroll);
                        let mut lp = p;
                        lp.surface = p.sunken;
                        let document = surface(parent, lp, 8.0, true);
                        document.removeFromSuperview();
                        scroll.setDocumentView(Some(&document));
                        let labels = data
                            .logs
                            .iter()
                            .map(|entry| {
                                let field = label(
                                    &document,
                                    &format!("[{}] {}", entry.level, entry.message),
                                    10.0,
                                    p,
                                );
                                field.setTextColor(Some(&native_theme::color(p.muted)));
                                field.setFont(Some(
                                    &NSFont::fontWithName_size(&NSString::from_str("Menlo"), 10.0)
                                        .unwrap_or_else(|| NSFont::systemFontOfSize(10.0)),
                                ));
                                field
                            })
                            .collect();
                        Control::Logs(scroll, document, labels)
                    });
                }
                self.actions(
                    card,
                    t("historyRecords"),
                    None,
                    vec![(GeneralAction::ClearHistory, t("clearAllHistory"))],
                );
                self.row(card, t("appSettings"), None, |parent, _| {
                    let button = button(parent, self, t("unavailable"), ButtonKind::Plain, -1);
                    button.setEnabled(false);
                    Control::Disabled(button)
                });
            }
            GeneralSection::Experimental => {
                self.toggle(card, "gpuComposition", GeneralField::GpuComposition);
                self.toggle(
                    card,
                    "performanceMonitoring",
                    GeneralField::PerformanceMonitoring,
                );
            }
            GeneralSection::About => {
                for (title, value) in [
                    (t("currentVersion"), data.version.as_str()),
                    (t("openSourceLicense"), "MIT License"),
                ] {
                    self.row(card, title, None, |parent, _| {
                        Control::Value(label(parent, value, 10.5, p))
                    });
                }
                self.actions(
                    card,
                    "项目主页",
                    None,
                    vec![(GeneralAction::OpenProject, "GitHub")],
                );
                self.actions(
                    card,
                    "第三方组件",
                    None,
                    vec![(GeneralAction::OpenNotices, "查看许可声明")],
                );
                let description = match data.update_phase {
                    UpdatePhase::Checking => "正在检查更新…".to_owned(),
                    UpdatePhase::Downloading => "正在下载并校验安装包…".to_owned(),
                    _ if data.downloaded_path.is_some() => "安装包已下载并打开".to_owned(),
                    _ => match data.update.as_ref() {
                        Some(update) if update.available => format!(
                            "发现新版本 {}",
                            update.latest_version.as_deref().unwrap_or("")
                        ),
                        Some(update) if update.latest_version.is_some() => {
                            "当前已是最新版本".to_owned()
                        }
                        Some(_) => "暂时没有已发布的正式版本".to_owned(),
                        None => "打开设置时自动查找最新正式版".to_owned(),
                    },
                };
                self.actions(
                    card,
                    "软件更新",
                    Some(&description),
                    vec![
                        (GeneralAction::CheckUpdates, "检查更新"),
                        (GeneralAction::OpenReleases, "发布页面"),
                    ],
                );
                if let Some(error) = &data.error {
                    self.paragraph(card, error, true);
                }
                if let Some(update) = data.update.as_ref().filter(|u| u.available) {
                    if let Some(size) = update.download_size {
                        let title = format!(
                            "{}（{} MB）",
                            if data.update_phase == UpdatePhase::Downloading {
                                "正在下载并校验…"
                            } else if data.downloaded_path.is_some() {
                                "再次下载并打开安装包"
                            } else {
                                "下载并打开安装包"
                            },
                            size.div_ceil(1_000_000)
                        );
                        self.row(card, "", None, |parent, tag| {
                            Control::Actions(vec![(
                                GeneralAction::DownloadUpdate,
                                button(
                                    parent,
                                    self,
                                    &title,
                                    ButtonKind::Primary,
                                    30_000 + tag * 10,
                                ),
                            )])
                        });
                        self.paragraph(card,if update.platform=="macos" {"安装包打开后，退出 SnapLingo，将新版拖入“应用程序”替换旧版，再打开。设置和历史记录会保留。"} else {"下载完成后将打开系统安装器，按提示覆盖安装。设置和历史记录会保留。"},false);
                    } else {
                        self.paragraph(
                            card,
                            "此版本暂未提供可验证的适配安装包，请前往发布页面查看。",
                            false,
                        );
                    }
                    if let Some(path) = &data.downloaded_path {
                        self.paragraph(card, &format!("安装包位置：{path}"), false);
                    }
                    if !update.notes.is_empty() {
                        self.disclosure(
                            card,
                            &format!(
                                "{} 更新说明",
                                update.latest_version.as_deref().unwrap_or("")
                            ),
                            self.ivars().release_notes_open.get(),
                            50_001,
                        );
                        if self.ivars().release_notes_open.get() {
                            self.paragraph(card, &update.notes, false);
                        }
                    }
                }
            }
        }
    }
    fn sync(&self) {
        let data = self.ivars().projection.borrow();
        let contents = self.ivars().contents.borrow();
        let Some(c) = contents.as_ref() else {
            return;
        };
        c.status.setStringValue(&NSString::from_str(
            data.error.as_deref().unwrap_or(t("autoSave")),
        ));
        c.status
            .setTextColor(Some(&native_theme::color(if data.error.is_some() {
                self.palette().danger
            } else {
                self.palette().muted
            })));
        for card in &c.cards {
            for (i, theme) in card.themes.iter().enumerate() {
                theme
                    .ivars()
                    .selected
                    .set(data.general.interface_theme == THEMES[i].0);
                NSView::setNeedsDisplay(theme, true);
            }
            for row in &card.rows {
                match &row.control {
                    Control::Choice(field, button, options) => {
                        if let GeneralValue::Text(value) = field.value(&data.general) {
                            let title = options
                                .iter()
                                .find(|(id, _)| *id == value)
                                .map(|(_, label)| *label)
                                .unwrap_or("");
                            if let Some(label) = button.ivars().label.borrow().as_ref() {
                                label.setStringValue(&NSString::from_str(title));
                            }
                        }
                    }
                    Control::Toggle(field, button) => {
                        if let GeneralValue::Toggle(value) = field.value(&data.general) {
                            button.setState(isize::from(value));
                            NSView::setNeedsDisplay(button, true);
                        }
                    }
                    Control::Text(field, text, _) | Control::Number(field, text, ..) => {
                        if text.currentEditor().is_none() {
                            let value = match field.value(&data.general) {
                                GeneralValue::Text(s) => s,
                                GeneralValue::Number(n) => n.to_string(),
                                _ => unreachable!(),
                            };
                            text.setStringValue(&NSString::from_str(&value));
                        }
                    }
                    Control::Actions(actions) => {
                        for (action, button) in actions {
                            let enabled = match action {
                                GeneralAction::CheckUpdates
                                | GeneralAction::OpenReleases
                                | GeneralAction::DownloadUpdate => {
                                    data.update_phase == UpdatePhase::Idle
                                }
                                GeneralAction::RequestPermission(_)
                                | GeneralAction::ResetPermission(_)
                                | GeneralAction::Restart => {
                                    !data.permission_busy
                                        && data.permissions.is_some()
                                        && data
                                            .permission_context
                                            .as_ref()
                                            .is_some_and(|c| !c.needs_installation)
                                }
                                GeneralAction::RefreshPermissions => !data.permission_busy,
                                _ => true,
                            };
                            button.setEnabled(enabled);
                        }
                    }
                    _ => {}
                }
                if let Control::Number(field, _, _, up, down, min, max, ..) = &row.control {
                    if let GeneralValue::Number(value) = field.value(&data.general) {
                        up.setEnabled(value < *max);
                        down.setEnabled(value > *min);
                    }
                }
            }
        }
    }
    fn reflow(&self) {
        if self.ivars().layouting.replace(true) {
            return;
        }
        let contents = self.ivars().contents.borrow();
        let Some(c) = contents.as_ref() else {
            self.ivars().layouting.set(false);
            return;
        };
        let style = Style::from(&self.ivars().projection.borrow().general);
        let bounds = self.bounds();
        // The React breakpoints apply to the complete Settings shell, including
        // its main navigation, rather than to this content pane alone.
        let shell_width = self.shell_width();
        let viewport_width = self.ivars().viewport_width.get().unwrap_or(shell_width);
        let narrow = shell_width <= 700.0 || viewport_width <= 700.0;
        let compact = shell_width <= 900.0 || viewport_width <= 900.0;
        let context_width = if style == Style::Native || narrow {
            0.0
        } else if viewport_width <= 900.0 {
            139.0
        } else {
            164.0
        };
        c.directory.setHidden(context_width == 0.0);
        c.directory
            .setFrame(rect(0.0, 0.0, context_width, bounds.size.height));
        let margin = if compact {
            22.0
        } else {
            match style {
                Style::Studio => 28.0,
                Style::Paper => 38.0,
                _ => 32.0,
            }
        };
        let pane_width = bounds.size.width - context_width;
        let page_width = (pane_width - 8.0).min(1080.0);
        let page_left = ((pane_width - 8.0) - page_width) * 0.5;
        let x = context_width + page_left + margin;
        let width = (page_width - margin * 2.0).max(200.0);
        let top = match style {
            Style::Native => 30.0,
            Style::Studio => 29.0,
            Style::Paper => 33.0,
        };
        let header_height = if context_width == 0.0 { 107.0 } else { 82.0 };
        let scroll_top = c.scroll.contentView().bounds().origin.y;
        let current_top = (top - scroll_top).max(0.0);
        let first_card = top + header_height + 24.0;
        c.header
            .setFrame(rect(x, 0.0, width, current_top + header_height));
        *c.header.ivars().lines.borrow_mut() = vec![(current_top + header_height - 1.0, 0.0)];
        c.header.setNeedsDisplay(true);
        let title_font = match style {
            Style::Paper => NSFont::fontWithName_size(&NSString::from_str("Songti SC"), 37.0)
                .unwrap_or_else(|| NSFont::systemFontOfSize(37.0)),
            Style::Studio => NSFont::systemFontOfSize_weight(25.0, unsafe { NSFontWeightMedium }),
            Style::Native => NSFont::systemFontOfSize_weight(28.0, unsafe { NSFontWeightSemibold }),
        };
        c.title.setFont(Some(&title_font));
        c.title.setFrame(rect(
            x - 2.0,
            current_top + if style == Style::Studio { 16.0 } else { 1.0 },
            200.0,
            42.0,
        ));
        let status_x = x + c.title.intrinsicContentSize().width + 22.0;
        c.header.ivars().status_dot.set(Some(NSPoint::new(
            status_x - x - 12.0,
            current_top + if style == Style::Studio { 33.0 } else { 17.0 },
        )));
        c.directory
            .ivars()
            .status_dot
            .set(Some(NSPoint::new(18.0, bounds.size.height - 32.0)));
        c.status.setFrame(rect(
            status_x,
            current_top + if style == Style::Studio { 26.0 } else { 10.0 },
            (x + width - status_x).max(0.0),
            22.0,
        ));
        c.directory_title.setFrame(rect(
            18.0,
            if style == Style::Paper { 35.0 } else { 28.0 },
            context_width - 31.0,
            25.0,
        ));
        let directory_font = if style == Style::Paper {
            NSFont::fontWithName_size(&NSString::from_str("Songti SC"), 22.0)
                .unwrap_or_else(|| NSFont::systemFontOfSize(22.0))
        } else {
            NSFont::systemFontOfSize_weight(15.0, unsafe { NSFontWeightSemibold })
        };
        c.directory_title.setFont(Some(&directory_font));
        c.directory_caption.setFrame(rect(
            18.0,
            if style == Style::Paper { 70.0 } else { 59.0 },
            context_width - 31.0,
            14.0,
        ));
        c.directory_status.setFrame(rect(
            30.0,
            bounds.size.height - 37.0,
            context_width - 40.0,
            18.0,
        ));
        let mut tab_x = x;
        for (i, tab) in c.tabs.iter().enumerate() {
            let label = tab.ivars().label.borrow();
            let label = label.as_ref().unwrap();
            label.setFont(Some(&NSFont::systemFontOfSize_weight(
                if context_width == 0.0 { 12.0 } else { 11.0 },
                unsafe { NSFontWeightMedium },
            )));
            label.setStringValue(&NSString::from_str(GeneralSection::ALL[i].label()));
            let tab_width = if context_width == 0.0 {
                label.intrinsicContentSize().width + 20.0
            } else {
                context_width - 18.0
            };
            let tab_frame = if context_width == 0.0 {
                rect(tab_x, current_top + 52.0, tab_width, 34.0)
            } else {
                rect(
                    9.0,
                    if style == Style::Paper { 110.0 } else { 92.0 } + i as f64 * 39.0,
                    tab_width,
                    34.0,
                )
            };
            set_button_frame(tab, tab_frame);
            c.tab_numbers[i].setHidden(context_width == 0.0);
            c.tab_numbers[i].setFrame(rect(9.0, 8.0, 20.0, 18.0));
            if context_width != 0.0 {
                label.setFrame(rect(32.0, 7.0, tab_width - 39.0, 20.0));
            }
            tab_x += tab_width + 2.0;
            tab.ivars()
                .selected
                .set(self.ivars().active.get() == GeneralSection::ALL[i]);
            tab.ivars()
                .underline
                .set(context_width == 0.0 && style == Style::Native);
            label.setAlignment(if context_width == 0.0 {
                NSTextAlignment::Center
            } else {
                NSTextAlignment::Left
            });
            label.setTextColor(Some(&native_theme::color(if tab.ivars().selected.get() {
                self.palette().accent
            } else {
                self.palette().muted
            })));
            NSView::setNeedsDisplay(tab, true);
        }
        c.scroll
            .setFrame(rect(context_width, 0.0, pane_width, bounds.size.height));
        let mut y = first_card;
        let mut offsets = Vec::new();
        for card in &c.cards {
            offsets.push(y);
            let inner = if style == Style::Paper { 0.0 } else { 23.0 };
            let content_width = width - inner * 2.0;
            card.surface
                .ivars()
                .rounded
                .set(if style == Style::Studio { 7.0 } else { 12.0 });
            card.surface.ivars().border.set(style != Style::Paper);
            let mut cp = self.palette();
            if style == Style::Paper {
                cp.surface = cp.canvas;
                card.surface.ivars().rounded.set(0.0);
            }
            card.surface.ivars().palette.set(cp);
            let heading_font = match style {
                Style::Paper => NSFont::fontWithName_size(&NSString::from_str("Songti SC"), 23.0)
                    .unwrap_or_else(|| NSFont::systemFontOfSize(23.0)),
                Style::Studio => {
                    NSFont::systemFontOfSize_weight(12.0, unsafe { NSFontWeightMedium })
                }
                Style::Native => {
                    NSFont::systemFontOfSize_weight(13.0, unsafe { NSFontWeightSemibold })
                }
            };
            card.heading.setFont(Some(&heading_font));
            card.heading
                .setStringValue(&NSString::from_str(card.section.label()));
            card.number.setHidden(style == Style::Native);
            card.number.setFrame(rect(
                inner,
                if style == Style::Paper { 25.0 } else { 16.0 },
                20.0,
                18.0,
            ));
            let heading_height = style.heading_height();
            card.heading.setFrame(rect(
                inner + if style == Style::Native { 0.0 } else { 24.0 },
                if style == Style::Studio { 13.0 } else { 17.0 },
                content_width,
                if style == Style::Paper { 32.0 } else { 22.0 },
            ));
            let mut lines = vec![(heading_height, 0.0)];
            let mut row_y = heading_height;
            if !card.themes.is_empty() {
                let gap = if compact { 9.0 } else { 15.0 };
                let theme_width = if narrow {
                    content_width
                } else {
                    (content_width - gap * 2.0) / 3.0
                };
                let theme_height = if narrow {
                    87.0
                } else {
                    (theme_width - 18.0) / 1.6 + 71.0
                };
                for (i, theme) in card.themes.iter().enumerate() {
                    set_button_frame(
                        theme,
                        rect(
                            inner
                                + if narrow {
                                    0.0
                                } else {
                                    i as f64 * (theme_width + gap)
                                },
                            row_y
                                + 24.0
                                + if narrow {
                                    i as f64 * (theme_height + gap)
                                } else {
                                    0.0
                                },
                            theme_width,
                            theme_height,
                        ),
                    );
                    for child in theme.subviews() {
                        let tag = child
                            .downcast_ref::<NSTextField>()
                            .map(|f| f.tag())
                            .unwrap_or(0);
                        let frame = match tag {
                            1 => rect(
                                if narrow { 126.0 } else { 12.0 },
                                theme_height - 49.0,
                                theme_width - if narrow { 138.0 } else { 24.0 },
                                22.0,
                            ),
                            2 => rect(49.0, theme_height - 47.0, 65.0, 18.0),
                            _ => rect(
                                if narrow { 126.0 } else { 12.0 },
                                theme_height - 25.0,
                                theme_width - if narrow { 138.0 } else { 24.0 },
                                16.0,
                            ),
                        };
                        child.setHidden(tag == 2 && compact);
                        child.setFrame(frame);
                        if let Some(field) = child.downcast_ref::<NSTextField>() {
                            let font = if tag == 1 {
                                NSFont::systemFontOfSize_weight(13.0, unsafe {
                                    NSFontWeightSemibold
                                })
                            } else {
                                NSFont::systemFontOfSize_weight(
                                    if tag == 3 && compact { 9.0 } else { 10.0 },
                                    unsafe { NSFontWeightRegular },
                                )
                            };
                            field.setFont(Some(&font));
                        }
                    }
                }
                row_y += 48.0
                    + theme_height * if narrow { 3.0 } else { 1.0 }
                    + if narrow { gap * 2.0 } else { 0.0 };
            }
            for row in &card.rows {
                match &row.control {
                    Control::Paragraph(field) => {
                        row.label.setHidden(true);
                        field.setPreferredMaxLayoutWidth(content_width);
                        let height = field.intrinsicContentSize().height.max(15.0);
                        let gap = if field.stringValue().to_string().starts_with("安装包位置：")
                            || field.stringValue().to_string().starts_with("当前应用：")
                        {
                            8.0
                        } else {
                            12.0
                        };
                        field.setFrame(rect(inner, row_y + gap, content_width, height));
                        row_y += height + gap;
                        continue;
                    }
                    Control::Disclosure(button) => {
                        row.label.setHidden(true);
                        set_button_frame(button, rect(inner, row_y + 8.0, content_width, 28.0));
                        row_y += 44.0;
                        continue;
                    }
                    Control::Logs(scroll, document, labels) => {
                        row.label.setHidden(true);
                        let height = (labels.len() as f64 * 20.0 + 16.0).min(160.0);
                        scroll.setFrame(rect(inner, row_y, content_width, height));
                        document.setFrame(rect(
                            0.0,
                            0.0,
                            content_width,
                            labels.len() as f64 * 20.0 + 16.0,
                        ));
                        for (i, label) in labels.iter().enumerate() {
                            label.setFrame(rect(
                                12.0,
                                8.0 + i as f64 * 20.0,
                                content_width - 24.0,
                                20.0,
                            ));
                        }
                        row_y += height;
                        continue;
                    }
                    _ => {}
                }
                if row_y > heading_height {
                    lines.push((row_y, if card.section==GeneralSection::Maintenance && matches!(&row.control,Control::Actions(actions) if actions.iter().any(|(a,_)|matches!(a,GeneralAction::ClearHistory))) {0.0} else {inner}));
                }
                let padding: f64 = match style {
                    Style::Native => 13.0,
                    Style::Studio => 9.0,
                    Style::Paper => 16.0,
                };
                let control_height: f64 = match row.control {
                    Control::Choice(..) => 42.0,
                    Control::Number(..) => 40.0,
                    Control::Text(..) => 36.0,
                    Control::Actions(..) => 36.0,
                    _ => 20.0,
                };
                let height = if card.section == GeneralSection::Permissions && row.status.is_some()
                {
                    85.0
                } else if card.section == GeneralSection::Permissions && row.description.is_some() {
                    64.0
                } else if matches!(&row.control, Control::Actions(_))
                    && row.label.stringValue().is_empty()
                {
                    if card.section == GeneralSection::About {
                        47.0
                    } else {
                        44.0
                    }
                } else if narrow {
                    20.0 + 10.0 + control_height + padding * 2.0
                } else {
                    (control_height + padding * 2.0 + 1.0).max(style.row_height())
                };
                let control_x = if narrow {
                    inner
                } else {
                    inner + content_width - 230.0
                };
                row.label
                    .setFont(Some(&NSFont::systemFontOfSize_weight(12.5, unsafe {
                        NSFontWeightSemibold
                    })));
                row.label.setFrame(rect(
                    inner,
                    row_y
                        + if narrow {
                            padding
                        } else if row.description.is_some() {
                            10.0
                        } else {
                            (height - 20.0) * 0.5
                        },
                    if narrow {
                        content_width
                    } else {
                        (content_width - 250.0).max(40.0)
                    },
                    20.0,
                ));
                if let Some(description) = &row.description {
                    description.setFrame(rect(
                        inner,
                        row_y
                            + if card.section == GeneralSection::Permissions {
                                32.0
                            } else {
                                30.0
                            },
                        if card.section == GeneralSection::Permissions {
                            content_width - 150.0
                        } else {
                            (content_width - 250.0).max(50.0)
                        },
                        if row.status.is_some() { 42.0 } else { 20.0 },
                    ));
                }
                if let Some(status) = &row.status {
                    status.setFrame(rect(
                        inner + content_width - 100.0,
                        (row_y + height * 0.5) - 9.0,
                        40.0,
                        18.0,
                    ));
                }
                let cy = row_y
                    + if narrow {
                        padding + 30.0
                    } else {
                        (height - control_height) * 0.5
                    };
                match &row.control {
                    Control::Choice(_, button, _) => {
                        set_button_frame(button, rect(control_x, cy, 230.0, 42.0))
                    }
                    Control::Toggle(_, button) => set_button_frame(
                        button,
                        rect(
                            if narrow {
                                inner
                            } else {
                                inner + content_width - 36.0
                            },
                            cy,
                            36.0,
                            20.0,
                        ),
                    ),
                    Control::Text(_, field, background) => {
                        field.setFont(Some(&NSFont::systemFontOfSize(10.5)));
                        background.setFrame(rect(control_x, cy, 230.0, 36.0));
                        field.setFrame(rect(control_x + 12.0, cy + 8.0, 206.0, 22.0));
                        background.removeFromSuperview();
                        card.surface.addSubview_positioned_relativeTo(
                            background,
                            objc2_app_kit::NSWindowOrderingMode::Below,
                            Some(field),
                        );
                    }
                    Control::Number(_, field, background, up, down, ..) => {
                        background.setFrame(rect(control_x, cy, 230.0, 40.0));
                        field.setFrame(rect(control_x + 16.0, cy + 9.0, 158.0, 24.0));
                        background.removeFromSuperview();
                        card.surface.addSubview_positioned_relativeTo(
                            background,
                            objc2_app_kit::NSWindowOrderingMode::Below,
                            Some(field),
                        );
                        set_button_frame(up, rect(control_x + 198.0, cy, 24.0, 20.0));
                        set_button_frame(down, rect(control_x + 198.0, cy + 20.0, 24.0, 20.0));
                        if let Some(suffix) = background.subviews().firstObject() {
                            suffix.setFrame(rect(145.0, 10.0, 35.0, 22.0));
                        }
                    }
                    Control::Value(value) => {
                        value.setTextColor(Some(&native_theme::color(
                            if card.section == GeneralSection::Permissions {
                                self.palette().good
                            } else if card.section == GeneralSection::Maintenance {
                                self.palette().muted
                            } else {
                                self.palette().ink
                            },
                        )));
                        value.setAlignment(NSTextAlignment::Right);
                        value.setFrame(rect(control_x, cy, 230.0, 20.0));
                    }
                    Control::Disabled(button) => set_button_frame(
                        button,
                        rect(inner + content_width - 132.0, cy, 132.0, 36.0),
                    ),
                    Control::Paragraph(_) | Control::Disclosure(_) | Control::Logs(..) => {
                        unreachable!()
                    }
                    Control::Actions(actions) => {
                        let widths = actions
                            .iter()
                            .map(|(_, b)| {
                                let label = b.ivars().label.borrow();
                                let measured = label
                                    .as_ref()
                                    .map(|l| l.intrinsicContentSize().width)
                                    .unwrap_or(60.0)
                                    + 24.0;
                                measured
                                    .max(if matches!(card.section, GeneralSection::Maintenance) || (card.section==GeneralSection::About && matches!(row.control,Control::Actions(ref a) if a.iter().any(|(action,_)|matches!(action,GeneralAction::OpenProject|GeneralAction::OpenNotices)))) {
                                        132.0
                                    } else if card.section==GeneralSection::Permissions {56.0} else {68.0})
                                    .min(content_width)
                            })
                            .collect::<Vec<_>>();
                        let mut ax = if row.label.stringValue().is_empty() {
                            inner
                        } else {
                            inner + content_width
                                - widths.iter().sum::<f64>()
                                - actions.len().saturating_sub(1) as f64 * 8.0
                        };
                        for ((_, button), w) in actions.iter().zip(widths) {
                            set_button_frame(button, rect(ax, cy, w, 36.0));
                            ax += w + 8.0;
                        }
                    }
                }
                row_y += height;
            }
            for log in &card.extra {
                log.setFrame(rect(inner + 12.0, row_y, content_width - 24.0, 20.0));
                row_y += 20.0;
            }
            *card.surface.ivars().lines.borrow_mut() = lines;
            card.surface.setFrame(rect(
                page_left + margin,
                y,
                width,
                row_y + if style == Style::Paper { 0.0 } else { 1.0 },
            ));
            card.surface.setNeedsDisplay(true);
            y += card.surface.frame().size.height + style.card_gap();
        }
        c.document.setFrame(rect(0.0, 0.0, pane_width, y + 56.0));
        *self.ivars().section_offsets.borrow_mut() = offsets;
        // AppKit may adjust the clip origin when resize/theme changes the
        // document frame. Preserve the requested position after all layout.
        c.scroll
            .contentView()
            .scrollToPoint(NSPoint::new(0.0, scroll_top));
        c.scroll.reflectScrolledClipView(&c.scroll.contentView());
        self.ivars().layouting.set(false);
        self.setNeedsDisplay(true);
    }
    fn did_scroll(&self) {
        if self.ivars().updating.get() || self.ivars().layouting.get() {
            return;
        }
        let offset = self
            .ivars()
            .contents
            .borrow()
            .as_ref()
            .map(|c| c.scroll.contentView().bounds().origin.y)
            .unwrap_or(0.0);
        if self
            .ivars()
            .navigation_offset
            .get()
            .is_some_and(|target| (target - offset).abs() < 0.5)
        {
            self.reflow();
            return;
        }
        self.ivars().navigation_offset.set(None);
        let style = Style::from(&self.ivars().projection.borrow().general);
        let shell_width = self.shell_width();
        let header = if style == Style::Native || shell_width <= 700.0 {
            107.0
        } else {
            82.0
        };
        let mut active = GeneralSection::Interface;
        for (index, y) in self.ivars().section_offsets.borrow().iter().enumerate() {
            if *y <= offset + header + 24.0 {
                active = GeneralSection::ALL[index];
            }
        }
        self.ivars().active.set(active);
        self.reflow();
    }
    fn control(&self, tag: isize, read: impl FnOnce(&Control)) {
        let control = self
            .ivars()
            .contents
            .borrow()
            .as_ref()
            .and_then(|c| c.cards.get((tag / 100) as usize))
            .and_then(|card| card.rows.get((tag % 100) as usize))
            .map(|row| row.control.clone());
        if let Some(control) = control {
            read(&control);
        }
    }
    fn perform(&self, tag: isize) {
        if tag >= 50_000 {
            if tag == 50_000 {
                self.ivars()
                    .recovery_open
                    .set(!self.ivars().recovery_open.get());
            } else {
                self.ivars()
                    .release_notes_open
                    .set(!self.ivars().release_notes_open.get());
            }
            self.rebuild();
            return;
        }
        if tag >= 40_000 {
            self.dispatch(GeneralAction::Edit(
                GeneralField::InterfaceTheme,
                GeneralValue::Text(THEMES[(tag - 40_000) as usize].0.into()),
            ));
            return;
        }
        if tag >= 30_000 {
            let mut action = None;
            self.control((tag - 30_000) / 10, |control| {
                if let Control::Actions(actions) = control {
                    action = actions
                        .get(((tag - 30_000) % 10) as usize)
                        .map(|(a, _)| a.clone());
                }
            });
            if let Some(action) = action {
                let prompt = match action {
                    GeneralAction::ClearLogs => Some(t("clearLogsConfirm")),
                    GeneralAction::ClearHistory => Some(t("clearHistoryConfirm")),
                    GeneralAction::ResetPermission(SystemPermission::ScreenRecording) => {
                        Some("将重置 SnapLingo 的“屏幕录制”权限，之后需要重新允许。")
                    }
                    GeneralAction::ResetPermission(SystemPermission::Accessibility) => {
                        Some("将重置 SnapLingo 的“辅助功能”权限，之后需要重新允许。")
                    }
                    _ => None,
                };
                if let Some(prompt) = prompt {
                    let alert = NSAlert::new(self.mtm());
                    alert.setMessageText(&NSString::from_str(prompt));
                    alert.addButtonWithTitle(&NSString::from_str("确认"));
                    alert.addButtonWithTitle(&NSString::from_str(t("cancel")));
                    if alert.runModal() != 1000 {
                        return;
                    }
                }
                self.dispatch(action);
            }
            return;
        }
        if tag >= 20_000 {
            let mut edit = None;
            self.control((tag - 20_000) / 2, |control| {
                if let Control::Number(field, _, _, _, _, min, max, step, _) = control {
                    if let GeneralValue::Number(value) =
                        field.value(&self.ivars().projection.borrow().general)
                    {
                        let next = if tag % 2 == 0 {
                            value.saturating_add(*step).min(*max)
                        } else {
                            value.saturating_sub(*step).max(*min)
                        };
                        edit = Some((*field, GeneralValue::Number(next)));
                    }
                }
            });
            if let Some((field, value)) = edit {
                self.dispatch(GeneralAction::Edit(field, value));
            }
            return;
        }
        if tag >= 10_000 {
            let section = GeneralSection::ALL[(tag - 10_000) as usize];
            self.jump(section);
            return;
        }
        let mut edit = None;
        self.control(tag, |control| match control {
            Control::Toggle(field, button) => {
                edit = Some((*field, GeneralValue::Toggle(button.state() != 0)))
            }
            Control::Choice(_, button, options) => {
                let menu = NSMenu::new(self.mtm());
                for (index, (_, title)) in options.iter().enumerate() {
                    let item = unsafe {
                        NSMenuItem::initWithTitle_action_keyEquivalent(
                            self.mtm().alloc(),
                            &NSString::from_str(title),
                            Some(sel!(selectChoice:)),
                            &NSString::new(),
                        )
                    };
                    unsafe {
                        item.setTarget(Some(self));
                    }
                    item.setTag(tag * 100 + index as isize);
                    menu.addItem(&item);
                }
                menu.popUpMenuPositioningItem_atLocation_inView(
                    None,
                    NSPoint::new(0.0, button.bounds().size.height),
                    Some(button),
                );
            }
            _ => {}
        });
        if let Some((field, value)) = edit {
            self.dispatch(GeneralAction::Edit(field, value));
        }
    }
    fn select(&self, tag: isize) {
        let mut edit = None;
        self.control(tag / 100, |control| {
            if let Control::Choice(field, _, options) = control {
                edit = options
                    .get((tag % 100) as usize)
                    .map(|(id, _)| (*field, GeneralValue::Text((*id).into())));
            }
        });
        if let Some((field, value)) = edit {
            self.dispatch(GeneralAction::Edit(field, value));
        }
    }
    fn commit_field(&self, field: &NSTextField) {
        let mut edit = None;
        self.control(field.tag(), |control| match control {
            Control::Text(key, ..) => {
                edit = Some((*key, GeneralValue::Text(field.stringValue().to_string())))
            }
            Control::Number(key, _, _, _, _, min, max, _, _) => {
                if let Ok(value) = field.stringValue().to_string().parse::<u32>() {
                    if (*min..=*max).contains(&value) {
                        edit = Some((*key, GeneralValue::Number(value)));
                    }
                }
            }
            _ => {}
        });
        if let Some((key, value)) = edit {
            self.dispatch(GeneralAction::Edit(key, value));
        } else {
            self.sync();
        }
    }
    pub(crate) fn jump(&self, section: GeneralSection) {
        self.reflow();
        let offset = self
            .ivars()
            .section_offsets
            .borrow()
            .get(section as usize)
            .copied()
            .unwrap_or(0.0);
        let style = Style::from(&self.ivars().projection.borrow().general);
        let shell_width = self.shell_width();
        let header = if style == Style::Native || shell_width <= 700.0 {
            107.0
        } else {
            82.0
        };
        self.ivars().updating.set(true);
        if let Some(c) = self.ivars().contents.borrow().as_ref() {
            let max = (c.document.bounds().size.height
                - c.scroll.contentView().bounds().size.height)
                .max(0.0);
            let target = if section == GeneralSection::Interface {
                0.0
            } else {
                (offset - header - 16.0).max(0.0).min(max)
            };
            c.scroll
                .contentView()
                .scrollToPoint(NSPoint::new(0.0, target));
            c.scroll.reflectScrolledClipView(&c.scroll.contentView());
            self.ivars()
                .navigation_offset
                .set(Some(c.scroll.contentView().bounds().origin.y));
        }
        self.ivars().updating.set(false);
        self.ivars().active.set(section);
        self.reflow();
    }
    #[cfg(test)]
    pub(crate) fn test_projection(&self) -> GeneralProjection {
        self.ivars().projection.borrow().clone()
    }
    #[cfg(test)]
    pub(crate) fn test_action(&self, action: GeneralAction) {
        self.dispatch(action);
    }
    #[cfg(test)]
    pub(crate) fn test_geometry(&self) -> serde_json::Value {
        let contents = self.ivars().contents.borrow();
        let c = contents.as_ref().unwrap();
        let frame = |r: NSRect| serde_json::json!({"x":r.origin.x,"y":r.origin.y,"width":r.size.width,"height":r.size.height});
        let card = &c.cards[0];
        let local = |r: NSRect| frame(self.convertRect_fromView(r, Some(&card.surface)));
        serde_json::json!({"card":frame(self.convertRect_fromView(card.surface.bounds(), Some(&card.surface))),"themes":card.themes.iter().map(|theme| local(theme.frame())).collect::<Vec<_>>(),"scroll":frame(c.scroll.frame()),"clip":frame(c.scroll.contentView().bounds()),"document":frame(c.document.frame()),"viewBounds":frame(self.bounds()),"contentInsets":format!("{:?}",c.scroll.contentInsets())})
    }
    #[cfg(test)]
    pub(crate) fn test_toggle(&self, field: GeneralField) {
        let button = self
            .ivars()
            .contents
            .borrow()
            .as_ref()
            .unwrap()
            .cards
            .iter()
            .flat_map(|c| &c.rows)
            .find_map(|row| match &row.control {
                Control::Toggle(key, button) if *key == field => Some(button.clone()),
                _ => None,
            })
            .unwrap();
        unsafe {
            button.performClick(None);
        }
    }
    #[cfg(test)]
    pub(crate) fn test_theme(&self, id: &str) {
        let index = THEMES.iter().position(|theme| theme.0 == id).unwrap();
        let button =
            self.ivars().contents.borrow().as_ref().unwrap().cards[0].themes[index].clone();
        unsafe {
            button.performClick(None);
        }
    }
    #[cfg(test)]
    pub(crate) fn test_choose(&self, field: GeneralField, id: &str) {
        let (tag, index) = self
            .ivars()
            .contents
            .borrow()
            .as_ref()
            .unwrap()
            .cards
            .iter()
            .flat_map(|c| &c.rows)
            .find_map(|row| match &row.control {
                Control::Choice(key, button, options) if *key == field => Some((
                    button.tag(),
                    options.iter().position(|(value, _)| *value == id).unwrap(),
                )),
                _ => None,
            })
            .unwrap();
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                self.mtm().alloc(),
                &NSString::from_str(id),
                Some(sel!(selectChoice:)),
                &NSString::new(),
            )
        };
        item.setTag(tag * 100 + index as isize);
        unsafe {
            assert!(objc2_app_kit::NSApplication::sharedApplication(self.mtm())
                .sendAction_to_from(sel!(selectChoice:), Some(self), Some(&item)));
        }
    }
    #[cfg(test)]
    pub(crate) fn test_input(&self, field: GeneralField, value: &str) {
        let input = self
            .ivars()
            .contents
            .borrow()
            .as_ref()
            .unwrap()
            .cards
            .iter()
            .flat_map(|c| &c.rows)
            .find_map(|row| match &row.control {
                Control::Text(key, input, _) | Control::Number(key, input, ..) if *key == field => {
                    Some(input.clone())
                }
                _ => None,
            })
            .unwrap();
        input.setStringValue(&NSString::from_str(value));
        let notification = unsafe {
            NSNotification::notificationWithName_object(
                &NSString::from_str("NSControlTextDidEndEditingNotification"),
                Some(&input),
            )
        };
        unsafe {
            assert!(objc2_app_kit::NSApplication::sharedApplication(self.mtm())
                .sendAction_to_from(
                    sel!(controlTextDidEndEditing:),
                    Some(self),
                    Some(&notification)
                ));
        }
    }
    #[cfg(test)]
    pub(crate) fn test_begin_edit(
        &self,
        field: GeneralField,
        draft: &str,
    ) -> Retained<objc2_app_kit::NSText> {
        let input = self
            .ivars()
            .contents
            .borrow()
            .as_ref()
            .unwrap()
            .cards
            .iter()
            .flat_map(|c| &c.rows)
            .find_map(|row| match &row.control {
                Control::Text(key, input, _) | Control::Number(key, input, ..) if *key == field => {
                    Some(input.clone())
                }
                _ => None,
            })
            .unwrap();
        self.window().unwrap().makeFirstResponder(Some(&input));
        unsafe {
            let _: () =
                msg_send![&input, selectText: std::ptr::null::<objc2::runtime::AnyObject>()];
        }
        let editor = input
            .currentEditor()
            .expect("AppKit did not create a real field editor");
        editor.setString(&NSString::from_str(draft));
        editor.setSelectedRange(NSRange::new(3, 5));
        editor
    }
    #[cfg(test)]
    pub(crate) fn test_has_editor(&self) -> bool {
        self.has_editor()
    }
    #[cfg(test)]
    pub(crate) fn test_scroll_to(&self, offset: f64) {
        {
            let contents = self.ivars().contents.borrow();
            let c = contents.as_ref().unwrap();
            c.scroll
                .contentView()
                .scrollToPoint(NSPoint::new(0.0, offset));
            c.scroll.reflectScrolledClipView(&c.scroll.contentView());
        }
        self.did_scroll();
    }
    #[cfg(test)]
    pub(crate) fn test_section_offset(&self, section: GeneralSection) -> f64 {
        self.ivars().section_offsets.borrow()[section as usize]
    }
    #[cfg(test)]
    pub(crate) fn test_active_section(&self) -> GeneralSection {
        self.ivars().active.get()
    }
    #[cfg(test)]
    pub(crate) fn test_disclosure(&self, release_notes: bool) {
        self.perform(if release_notes { 50_001 } else { 50_000 });
    }
    #[cfg(test)]
    pub(crate) fn test_control_count(&self) -> usize {
        self.ivars()
            .contents
            .borrow()
            .as_ref()
            .unwrap()
            .cards
            .iter()
            .map(|card| card.rows.len())
            .sum()
    }
}

impl Drop for GeneralView {
    fn drop(&mut self) {
        unsafe {
            NSNotificationCenter::defaultCenter().removeObserver(self);
        }
    }
}
