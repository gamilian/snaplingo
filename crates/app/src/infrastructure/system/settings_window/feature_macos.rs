//! Native OCR and Translation preferences share the existing Settings page layout.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{
    define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSBezierPath, NSBorderType, NSButton, NSButtonType, NSFont, NSFontAttributeName,
    NSFontWeightMedium, NSFontWeightRegular, NSFontWeightSemibold, NSForegroundColorAttributeName,
    NSKernAttributeName, NSMenu, NSMenuItem, NSScrollView, NSScrollerStyle, NSSlider,
    NSTextAlignment, NSTextField, NSView, NSViewBoundsDidChangeNotification,
};
use objc2_foundation::{
    NSAttributedString, NSDictionary, NSEdgeInsets, NSNotification, NSNotificationCenter, NSNumber,
    NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use super::super::native_theme::{self, Palette};
use super::HotkeyView;
use crate::application::settings::{FeaturePreferences, GeneralValue, OcrField, TranslationField};
use crate::domain::GeneralSettings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FeatureKind {
    Ocr,
    Translation,
}
impl FeatureKind {
    fn title(self) -> &'static str {
        match self {
            Self::Ocr => "OCR",
            Self::Translation => "翻译",
        }
    }
    pub(crate) fn sections(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Ocr => &[
                ("hotkeys", "快捷键"),
                ("recognition", "识别与文本"),
                ("window", "窗口与提示"),
            ],
            Self::Translation => &[
                ("hotkeys", "快捷键"),
                ("language", "语言与取词"),
                ("window", "窗口与输入框"),
                ("behavior", "翻译行为"),
            ],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FeatureField {
    Ocr(OcrField),
    Translation(TranslationField),
}
impl FeatureField {
    fn label(self) -> &'static str {
        match self {
            Self::Ocr(f) => f.label(),
            Self::Translation(f) => f.label(),
        }
    }
    fn choices(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Ocr(f) => f.choices(),
            Self::Translation(f) => f.choices(),
        }
    }
    fn value(self, p: &FeaturePreferences) -> GeneralValue {
        match self {
            Self::Ocr(f) => f.value(&p.ocr),
            Self::Translation(f) => f.value(&p.translation),
        }
    }
    fn range(self) -> Option<(u32, u32, u32, &'static str)> {
        match self {
            Self::Translation(TranslationField::MaxWindowHeightRatio) => Some((30, 90, 1, "%")),
            Self::Translation(TranslationField::WindowWidth) => Some((300, 1000, 10, "pt")),
            _ => None,
        }
    }
}
#[derive(Clone)]
pub(crate) struct FeatureProjection {
    pub general: GeneralSettings,
    pub preferences: FeaturePreferences,
    pub error: Option<String>,
}
type Dispatch = Rc<dyn Fn(FeatureField, GeneralValue) -> Result<FeatureProjection, String>>;

fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w.max(0.0), h.max(0.0)))
}
fn fill(r: NSRect, radius: f64, color: [u8; 3]) {
    native_theme::color(color).setFill();
    NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(r, radius, radius).fill();
}
fn stroke(r: NSRect, radius: f64, color: [u8; 3]) {
    native_theme::color(color).setStroke();
    let path = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(r, radius, radius);
    path.setLineWidth(1.0);
    path.stroke();
}
fn label(
    parent: &NSView,
    text: &str,
    size: f64,
    weight: f64,
    color: [u8; 3],
) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&NSString::from_str(text), parent.mtm());
    field.setFont(Some(&NSFont::systemFontOfSize_weight(size, weight)));
    field.setTextColor(Some(&native_theme::color(color)));
    field.setSelectable(false);
    parent.addSubview(&field);
    field
}
struct SurfaceState {
    palette: Cell<Palette>,
    card: Cell<bool>,
    paper: Cell<bool>,
    studio: Cell<bool>,
    lines: RefCell<Vec<(f64, f64)>>,
    dot: Cell<Option<NSPoint>>,
    horizontal_tabs: Cell<Option<NSRect>>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name="SnapLingoFeatureSettingsSurface"]
    #[thread_kind=MainThreadOnly]
    #[ivars=SurfaceState]
    struct Surface;
    unsafe impl NSObjectProtocol for Surface {}
    impl Surface {
        #[unsafe(method(isFlipped))] fn is_flipped(&self)->bool{true}
        #[unsafe(method(drawRect:))] fn draw(&self,_:NSRect){
            let s=self.ivars();let p=s.palette.get();let card=s.card.get();let paper=s.paper.get();let radius=if !card||paper {0.0}else if s.studio.get(){7.0}else{12.0};
            fill(self.bounds(),radius,if card&&!paper{p.surface}else{p.canvas});if card&&!paper{stroke(self.bounds(),radius,p.line);}
            for &(y,inset) in s.lines.borrow().iter(){fill(rect(inset,y,self.bounds().size.width-inset*2.0,1.0),0.0,p.line);}
            if let Some(dot)=s.dot.get(){fill(rect(dot.x-3.0,dot.y-3.0,12.0,12.0),6.0,p.good_soft);fill(rect(dot.x,dot.y,6.0,6.0),3.0,p.good);}
            if let Some(tabs)=s.horizontal_tabs.get(){fill(tabs,7.0,p.raised);}
        }
    }
);
fn surface(parent: &NSView, p: Palette, card: bool) -> Retained<Surface> {
    let view = Surface::alloc(parent.mtm()).set_ivars(SurfaceState {
        palette: Cell::new(p),
        card: Cell::new(card),
        paper: Cell::new(false),
        studio: Cell::new(false),
        lines: RefCell::new(Vec::new()),
        dot: Cell::new(None),
        horizontal_tabs: Cell::new(None),
    });
    let view: Retained<Surface> =
        unsafe { msg_send![super(view),initWithFrame:rect(0.0,0.0,1.0,1.0)] };
    parent.addSubview(&view);
    view
}
#[derive(Clone, Copy)]
enum ButtonKind {
    Choice,
    Toggle,
    Tab,
}
struct ButtonState {
    palette: Cell<Palette>,
    kind: ButtonKind,
    selected: Cell<bool>,
    underline: Cell<bool>,
    directory: Cell<bool>,
    title: Retained<NSTextField>,
}
define_class!(
    #[unsafe(super(NSButton))]
    #[name="SnapLingoFeatureSettingsButton"]
    #[thread_kind=MainThreadOnly]
    #[ivars=ButtonState]
    struct Button;
    unsafe impl NSObjectProtocol for Button{}
    impl Button {
        #[unsafe(method(isFlipped))]fn is_flipped(&self)->bool{true}
        #[unsafe(method(acceptsFirstResponder))]fn accepts_first_responder(&self)->bool{true}
        #[unsafe(method(hitTest:))]fn hit_test(&self, point: NSPoint) -> *mut NSView {
            let hit: *mut NSView = unsafe { msg_send![super(self), hitTest: point] };
            if hit.is_null() { hit } else { (self as *const Self).cast_mut().cast() }
        }
        #[unsafe(method(drawRect:))]fn draw(&self,_:NSRect){
            let s=self.ivars();let p=s.palette.get();let b=self.bounds();
            match s.kind{
                ButtonKind::Toggle=>{fill(b,10.0,if self.state()!=0{p.accent}else{p.line_strong});fill(rect(if self.state()!=0{18.0}else{2.0},2.0,16.0,16.0),8.0,p.surface);}
                ButtonKind::Choice=>{fill(b,8.0,p.surface);stroke(rect(0.5,0.5,b.size.width-1.0,b.size.height-1.0),8.0,p.line);native_theme::color(p.faint).setStroke();let path=NSBezierPath::bezierPath();let x=b.size.width-26.0;let y=b.size.height*0.5;path.moveToPoint(NSPoint::new(x-6.0,y-3.0));path.lineToPoint(NSPoint::new(x,y+3.0));path.lineToPoint(NSPoint::new(x+6.0,y-3.0));path.setLineWidth(1.7);path.stroke();}
                ButtonKind::Tab=>if s.selected.get(){if s.underline.get(){fill(rect(0.0,b.size.height-2.0,b.size.width,2.0),0.0,p.accent);}else{fill(b,5.0,if s.directory.get(){p.accent_soft}else{p.surface});}},
            }
        }
    }
);
fn button(
    parent: &NSView,
    root: &FeatureView,
    title: &str,
    kind: ButtonKind,
    tag: isize,
) -> Retained<Button> {
    let p = root.palette();
    let text = label(parent, title, 12.5, unsafe { NSFontWeightRegular }, p.ink);
    text.removeFromSuperview();
    let control = Button::alloc(parent.mtm()).set_ivars(ButtonState {
        palette: Cell::new(p),
        kind,
        selected: Cell::new(false),
        underline: Cell::new(false),
        directory: Cell::new(false),
        title: text,
    });
    let control: Retained<Button> =
        unsafe { msg_send![super(control),initWithFrame:rect(0.0,0.0,1.0,1.0)] };
    control.setTitle(&NSString::from_str(title));
    control.setBordered(false);
    control.setTag(tag);
    if matches!(kind, ButtonKind::Toggle) {
        control.setButtonType(NSButtonType::Switch);
        control.ivars().title.setHidden(true);
    }
    control.addSubview(&control.ivars().title);
    unsafe {
        control.setTarget(Some(root));
        control.setAction(Some(sel!(performFeatureAction:)));
    }
    parent.addSubview(&control);
    control
}
fn button_frame(button: &Button, r: NSRect) {
    button.setFrame(r);
    button.ivars().title.setFrame(rect(
        16.0,
        (r.size.height - 20.0) * 0.5,
        r.size.width - 32.0,
        20.0,
    ));
}
struct RangeState {
    palette: Cell<Palette>,
}
define_class!(
    #[unsafe(super(NSSlider))]
    #[name="SnapLingoFeatureSettingsRange"]
    #[thread_kind=MainThreadOnly]
    #[ivars=RangeState]
    struct Range;
    unsafe impl NSObjectProtocol for Range{}
    impl Range{
        #[unsafe(method(drawRect:))]fn draw(&self,_:NSRect){let p=self.ivars().palette.get();let b=self.bounds();let fraction=((self.doubleValue()-self.minValue())/(self.maxValue()-self.minValue())).clamp(0.0,1.0);let width=(b.size.width-18.0).max(0.0);let x=9.0+width*fraction;fill(rect(0.0,(b.size.height-6.0)*0.5,b.size.width,6.0),3.0,p.line);fill(rect(0.0,(b.size.height-6.0)*0.5,x,6.0),3.0,p.accent);fill(rect(x-9.0,(b.size.height-18.0)*0.5,18.0,18.0),9.0,[255;3]);native_theme::color(p.accent).setStroke();let path=NSBezierPath::bezierPathWithOvalInRect(rect(x-8.0,(b.size.height-16.0)*0.5,16.0,16.0));path.setLineWidth(2.0);path.stroke();}
    }
);
#[derive(Clone)]
enum Control {
    Toggle(Retained<Button>),
    Choice(Retained<Button>),
    Range(Retained<Range>, Retained<NSTextField>),
}
struct Row {
    field: FeatureField,
    label: Retained<NSTextField>,
    description: Option<Retained<NSTextField>>,
    control: Control,
    group_start: bool,
}
struct Card {
    surface: Retained<Surface>,
    heading: Retained<NSTextField>,
    number: Retained<NSTextField>,
    rows: Vec<Row>,
}
struct Contents {
    scroll: Retained<NSScrollView>,
    document: Retained<Surface>,
    header: Retained<Surface>,
    directory: Retained<Surface>,
    title: Retained<NSTextField>,
    status: Retained<NSTextField>,
    directory_title: Retained<NSTextField>,
    caption: Retained<NSTextField>,
    directory_status: Retained<NSTextField>,
    tabs: Vec<Retained<Button>>,
    tab_numbers: Vec<Retained<NSTextField>>,
    cards: Vec<Card>,
}
pub(crate) struct FeatureState {
    kind: FeatureKind,
    projection: RefCell<FeatureProjection>,
    dispatch: Dispatch,
    hotkeys: Retained<HotkeyView>,
    contents: RefCell<Option<Contents>>,
    layouting: Cell<bool>,
    disposed: Cell<bool>,
    shell_width: Cell<Option<f64>>,
    viewport_width: Cell<Option<f64>>,
    active: Cell<usize>,
    offsets: RefCell<Vec<f64>>,
    navigation_offset: Cell<Option<f64>>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name="SnapLingoFeatureSettingsView"]
    #[thread_kind=MainThreadOnly]
    #[ivars=FeatureState]
    pub(crate)struct FeatureView;
    unsafe impl NSObjectProtocol for FeatureView{}
    impl FeatureView{
        #[unsafe(method(isFlipped))]fn is_flipped(&self)->bool{true}
        #[unsafe(method(drawRect:))]fn draw(&self,_:NSRect){fill(self.bounds(),0.0,self.palette().canvas);}
        #[unsafe(method(setFrameSize:))]fn resized(&self,size:NSSize){unsafe{let _:()=msg_send![super(self),setFrameSize:size];}self.reflow();}
        #[unsafe(method(performFeatureAction:))]fn action(&self,button:&Button){self.perform(button.tag());}
        #[unsafe(method(selectFeatureChoice:))]fn choice(&self,item:&NSMenuItem){let index=item.tag()/100;let choice=(item.tag()%100)as usize;if let Some(field)=self.field(index){if let Some((value,_))=field.choices().get(choice){self.edit(field,GeneralValue::Text((*value).into()));}}}
        #[unsafe(method(changeFeatureRange:))]fn range_changed(&self,range:&Range){self.range_value(range.tag(),range.doubleValue());}
        #[unsafe(method(featureScrollChanged:))]fn scrolled(&self,_:&NSNotification){self.did_scroll();}
    }
);
impl FeatureView {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        kind: FeatureKind,
        projection: FeatureProjection,
        hotkeys: Retained<HotkeyView>,
        dispatch: Dispatch,
    ) -> Retained<Self> {
        let view = Self::alloc(mtm).set_ivars(FeatureState {
            kind,
            projection: RefCell::new(projection),
            dispatch,
            hotkeys,
            contents: RefCell::new(None),
            layouting: Cell::new(false),
            disposed: Cell::new(false),
            shell_width: Cell::new(None),
            viewport_width: Cell::new(None),
            active: Cell::new(0),
            offsets: RefCell::new(Vec::new()),
            navigation_offset: Cell::new(None),
        });
        let view: Retained<Self> =
            unsafe { msg_send![super(view),initWithFrame:rect(0.0,0.0,698.0,650.0)] };
        view.build();
        view.refresh_theme();
        view.sync();
        view.reflow();
        view
    }
    pub(crate) fn view(&self) -> &NSView {
        self
    }
    pub(crate) fn set_layout_widths(&self, shell: f64, viewport: f64) {
        self.ivars().shell_width.set(Some(shell));
        self.ivars().viewport_width.set(Some(viewport));
        self.reflow();
    }
    fn shell_width(&self) -> f64 {
        self.ivars().shell_width.get().unwrap_or_else(|| {
            let general = &self.ivars().projection.borrow().general;
            self.bounds().size.width
                + match general.interface_theme.as_str() {
                    "studio" => 68.0,
                    "paper" => 0.0,
                    _ => 202.0,
                }
        })
    }
    pub(crate) fn hotkeys(&self) -> &HotkeyView {
        &self.ivars().hotkeys
    }
    fn palette(&self) -> Palette {
        let p = self.ivars().projection.borrow();
        native_theme::palette(
            &p.general.interface_theme,
            native_theme::is_dark(&p.general.theme),
        )
    }
    pub(crate) fn refresh(&self, projection: FeatureProjection) {
        if self.ivars().disposed.get() {
            return;
        }
        *self.ivars().projection.borrow_mut() = projection;
        self.refresh_theme();
        self.sync();
        self.reflow();
    }
    fn refresh_theme(&self) {
        let general = &self.ivars().projection.borrow().general;
        let dark = native_theme::is_dark(&general.theme);
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
        self.ivars().hotkeys.refresh(general.clone());
    }
    fn fields(&self) -> Vec<Vec<(FeatureField, bool)>> {
        use FeatureField::{Ocr as O, Translation as T};
        use OcrField as Of;
        use TranslationField as Tf;
        match self.ivars().kind {
            FeatureKind::Ocr => vec![
                vec![],
                vec![
                    (O(Of::GpuAcceleration), false),
                    (O(Of::RecognitionLanguage), false),
                    (O(Of::PreserveFormatting), false),
                    (O(Of::RemoveChineseSpaces), false),
                    (O(Of::ShowConfidence), false),
                ],
                vec![
                    (O(Of::WindowPosition), false),
                    (O(Of::HideSilentStatus), true),
                ],
            ],
            FeatureKind::Translation => vec![
                vec![],
                vec![
                    (T(Tf::DefaultSourceLang), false),
                    (T(Tf::DefaultTargetLang), false),
                    (T(Tf::SelectionTextMode), true),
                ],
                vec![
                    (T(Tf::SelectionWindowPosition), false),
                    (T(Tf::InputWindowPosition), false),
                    (T(Tf::MaxWindowHeightRatio), true),
                    (T(Tf::WindowWidth), false),
                    (T(Tf::SelectionInputState), true),
                    (T(Tf::ScreenshotInputState), false),
                ],
                vec![
                    (T(Tf::AutoTranslate), false),
                    (T(Tf::IncrementalTranslation), false),
                    (T(Tf::AutoCopy), false),
                    (T(Tf::PreserveLineBreaks), false),
                    (T(Tf::WindowAlwaysOnTop), true),
                    (T(Tf::HideOnBlur), false),
                ],
            ],
        }
    }
    fn build(&self) {
        let p = self.palette();
        let scroll = NSScrollView::initWithFrame(self.mtm().alloc(), self.bounds());
        scroll.setHasVerticalScroller(true);
        scroll.setScrollerStyle(NSScrollerStyle::Overlay);
        scroll.setBorderType(NSBorderType::NoBorder);
        scroll.setDrawsBackground(false);
        scroll.setAutomaticallyAdjustsContentInsets(false);
        scroll.setContentInsets(NSEdgeInsets {
            top: 0.0,
            left: 0.0,
            bottom: 0.0,
            right: 0.0,
        });
        self.addSubview(&scroll);
        let document = surface(self, p, false);
        document.removeFromSuperview();
        scroll.setDocumentView(Some(&document));
        let header = surface(self, p, false);
        let directory = surface(self, p, false);
        let title = label(
            self,
            self.ivars().kind.title(),
            28.0,
            unsafe { NSFontWeightSemibold },
            p.ink,
        );
        let status = label(
            self,
            "自动保存",
            10.5,
            unsafe { NSFontWeightMedium },
            p.muted,
        );
        let directory_title = label(
            &directory,
            self.ivars().kind.title(),
            15.0,
            unsafe { NSFontWeightSemibold },
            p.ink,
        );
        let caption = label(
            &directory,
            "CONTENTS",
            9.0,
            unsafe { NSFontWeightRegular },
            p.faint,
        );
        let directory_status = label(
            &directory,
            "自动保存",
            10.0,
            unsafe { NSFontWeightRegular },
            p.muted,
        );
        let mut tabs = Vec::new();
        let mut tab_numbers = Vec::new();
        let mut cards = Vec::new();
        let fields = self.fields();
        for (index, (_, name)) in self.ivars().kind.sections().iter().enumerate() {
            let tab = button(self, self, name, ButtonKind::Tab, 10_000 + index as isize);
            let n = label(
                &tab,
                &format!("{:02}", index + 1),
                10.0,
                unsafe { NSFontWeightRegular },
                p.faint,
            );
            tabs.push(tab);
            tab_numbers.push(n);
            let card = surface(&document, p, true);
            let heading = label(&card, name, 13.0, unsafe { NSFontWeightSemibold }, p.ink);
            let number = label(
                &card,
                &format!("{:02}", index + 1),
                10.0,
                unsafe { NSFontWeightRegular },
                p.faint,
            );
            let mut rows = Vec::new();
            for (row_index, &(field, group_start)) in fields[index].iter().enumerate() {
                let tag = (index * 100 + row_index) as isize;
                let text = label(
                    &card,
                    field.label(),
                    12.5,
                    unsafe { NSFontWeightSemibold },
                    p.ink,
                );
                let description = if field == FeatureField::Ocr(OcrField::GpuAcceleration) {
                    Some(label(
                        &card,
                        "用于 PaddleOCR；设备不支持时自动使用 CPU",
                        10.5,
                        unsafe { NSFontWeightRegular },
                        p.muted,
                    ))
                } else {
                    None
                };
                let control = if let Some((min, max, _, _)) = field.range() {
                    let range = Range::alloc(self.mtm()).set_ivars(RangeState {
                        palette: Cell::new(p),
                    });
                    let range: Retained<Range> =
                        unsafe { msg_send![super(range),initWithFrame:rect(0.0,0.0,210.0,18.0)] };
                    range.setMinValue(min as f64);
                    range.setMaxValue(max as f64);
                    range.setContinuous(true);
                    range.setTag(tag);
                    unsafe {
                        range.setTarget(Some(self));
                        range.setAction(Some(sel!(changeFeatureRange:)));
                    }
                    card.addSubview(&range);
                    Control::Range(
                        range,
                        label(&card, "", 10.5, unsafe { NSFontWeightSemibold }, p.accent),
                    )
                } else if field.choices().is_empty() {
                    Control::Toggle(button(&card, self, field.label(), ButtonKind::Toggle, tag))
                } else {
                    Control::Choice(button(&card, self, "", ButtonKind::Choice, tag))
                };
                rows.push(Row {
                    field,
                    label: text,
                    description,
                    control,
                    group_start,
                });
            }
            cards.push(Card {
                surface: card,
                heading,
                number,
                rows,
            });
        }
        cards[0].surface.addSubview(&self.ivars().hotkeys);
        cards[0]
            .surface
            .addSubview(self.ivars().hotkeys.toolbar_view());
        *self.ivars().contents.borrow_mut() = Some(Contents {
            scroll,
            document,
            header,
            directory,
            title,
            status,
            directory_title,
            caption,
            directory_status,
            tabs,
            tab_numbers,
            cards,
        });
        let c = self.ivars().contents.borrow();
        let c = c.as_ref().unwrap();
        c.scroll
            .contentView()
            .setPostsBoundsChangedNotifications(true);
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                self,
                sel!(featureScrollChanged:),
                Some(NSViewBoundsDidChangeNotification),
                Some(&c.scroll.contentView()),
            );
        }
    }
    fn sync(&self) {
        let p = self.palette();
        let projection = self.ivars().projection.borrow();
        let contents = self.ivars().contents.borrow();
        let Some(c) = contents.as_ref() else {
            return;
        };
        c.status.setStringValue(&NSString::from_str(
            projection.error.as_deref().unwrap_or("自动保存"),
        ));
        c.status
            .setTextColor(Some(&native_theme::color(if projection.error.is_some() {
                p.danger
            } else {
                p.muted
            })));
        for card in &c.cards {
            for row in &card.rows {
                match (&row.control, row.field.value(&projection.preferences)) {
                    (Control::Toggle(button), GeneralValue::Toggle(value)) => {
                        button.setState(isize::from(value));
                        button.ivars().palette.set(p);
                        NSView::setNeedsDisplay(button, true);
                    }
                    (Control::Choice(button), GeneralValue::Text(value)) => {
                        let title = row
                            .field
                            .choices()
                            .iter()
                            .find(|(id, _)| *id == value)
                            .map(|(_, label)| *label)
                            .unwrap_or(&value);
                        button
                            .ivars()
                            .title
                            .setStringValue(&NSString::from_str(title));
                        button
                            .ivars()
                            .title
                            .setTextColor(Some(&native_theme::color(p.ink)));
                        button.ivars().palette.set(p);
                        NSView::setNeedsDisplay(button, true);
                    }
                    (Control::Range(range, label), GeneralValue::Number(value)) => {
                        range.setDoubleValue(value as f64);
                        range.ivars().palette.set(p);
                        NSView::setNeedsDisplay(range, true);
                        label.setStringValue(&NSString::from_str(&format!(
                            "{value}{}",
                            row.field.range().unwrap().3
                        )));
                        label.setTextColor(Some(&native_theme::color(p.accent)));
                    }
                    _ => {}
                }
            }
        }
    }
    fn field(&self, tag: isize) -> Option<FeatureField> {
        self.ivars()
            .contents
            .borrow()
            .as_ref()?
            .cards
            .get((tag / 100) as usize)?
            .rows
            .get((tag % 100) as usize)
            .map(|row| row.field)
    }
    fn edit(&self, field: FeatureField, value: GeneralValue) {
        if self.ivars().disposed.get() {
            return;
        }
        match (self.ivars().dispatch)(field, value) {
            Ok(next) => self.refresh(next),
            Err(error) => {
                self.ivars().projection.borrow_mut().error = Some(error);
                self.sync();
            }
        }
    }
    fn perform(&self, tag: isize) {
        if self.ivars().disposed.get() {
            return;
        }
        if tag >= 10_000 {
            if let Some((id, _)) = self.ivars().kind.sections().get((tag - 10_000) as usize) {
                self.jump(id);
            }
            return;
        }
        let Some(field) = self.field(tag) else {
            return;
        };
        if field.choices().is_empty() {
            let current = field.value(&self.ivars().projection.borrow().preferences);
            if let GeneralValue::Toggle(value) = current {
                self.edit(field, GeneralValue::Toggle(!value));
            }
            return;
        }
        let control = self
            .ivars()
            .contents
            .borrow()
            .as_ref()
            .and_then(|c| c.cards.get((tag / 100) as usize))
            .and_then(|c| c.rows.get((tag % 100) as usize))
            .map(|row| row.control.clone());
        if let Some(Control::Choice(button)) = control {
            let menu = NSMenu::new(self.mtm());
            for (index, (id, title)) in field.choices().iter().enumerate() {
                let item = unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        self.mtm().alloc(),
                        &NSString::from_str(title),
                        Some(sel!(selectFeatureChoice:)),
                        &NSString::new(),
                    )
                };
                item.setTag(tag * 100 + index as isize);
                if let GeneralValue::Text(value) =
                    field.value(&self.ivars().projection.borrow().preferences)
                {
                    item.setState(isize::from(value == *id));
                }
                unsafe {
                    item.setTarget(Some(self));
                }
                menu.addItem(&item);
            }
            menu.popUpMenuPositioningItem_atLocation_inView(
                None,
                NSPoint::new(0.0, button.bounds().size.height),
                Some(&button),
            );
        }
    }
    fn range_value(&self, tag: isize, value: f64) {
        if let Some(field) = self.field(tag) {
            if let Some((min, max, step, _)) = field.range() {
                let value = (min as f64
                    + ((value - min as f64) / step as f64).round() * step as f64)
                    .clamp(min as f64, max as f64) as u32;
                self.edit(field, GeneralValue::Number(value));
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
        let p = self.palette();
        let projection = self.ivars().projection.borrow();
        let general = &projection.general;
        let studio = general.interface_theme == "studio";
        let paper = general.interface_theme == "paper";
        let bounds = self.bounds();
        let shell_width = self.shell_width();
        let viewport_width = self.ivars().viewport_width.get().unwrap_or(shell_width);
        let narrow = shell_width <= 700.0 || viewport_width <= 700.0;
        let context = if (!studio && !paper) || narrow {
            0.0
        } else if viewport_width <= 900.0 {
            139.0
        } else {
            164.0
        };
        let margin = if shell_width <= 900.0 || viewport_width <= 900.0 {
            22.0
        } else if paper {
            38.0
        } else if studio {
            28.0
        } else {
            32.0
        };
        let pane = bounds.size.width - context;
        let page_width = (pane - 8.0).min(1080.0);
        let page_left = ((pane - 8.0) - page_width) * 0.5;
        let width = page_width - margin * 2.0;
        let x = context + page_left + margin;
        let top = if paper {
            33.0
        } else if studio {
            29.0
        } else {
            30.0
        };
        let header_height = self.header_height();
        let clip_top = c.scroll.contentView().bounds().origin.y;
        let current_top = (top - clip_top).max(0.0);
        for surface in [&c.document, &c.header, &c.directory] {
            surface.ivars().palette.set(p);
            surface.setNeedsDisplay(true);
        }
        c.directory.setHidden(context == 0.0);
        c.directory
            .setFrame(rect(0.0, 0.0, context, bounds.size.height));
        c.header
            .setFrame(rect(x, 0.0, width, current_top + header_height));
        *c.header.ivars().lines.borrow_mut() = vec![(current_top + header_height - 1.0, 0.0)];
        let title_font = if paper {
            NSFont::fontWithName_size(&NSString::from_str("Songti SC"), 37.0)
                .unwrap_or_else(|| NSFont::systemFontOfSize(37.0))
        } else {
            NSFont::systemFontOfSize_weight(if studio { 25.0 } else { 28.0 }, unsafe {
                NSFontWeightSemibold
            })
        };
        c.title.setFont(Some(&title_font));
        c.title.setTextColor(Some(&native_theme::color(p.ink)));
        c.title.setFrame(rect(
            x - 2.0,
            current_top + if studio { 16.0 } else { 1.0 },
            200.0,
            42.0,
        ));
        let status_x = x + c.title.intrinsicContentSize().width + 22.0;
        c.status.setFrame(rect(
            status_x,
            current_top + if studio { 26.0 } else { 10.0 },
            width - (status_x - x),
            22.0,
        ));
        c.header.ivars().dot.set(Some(NSPoint::new(
            status_x - x - 12.0,
            current_top + if studio { 33.0 } else { 17.0 },
        )));
        let directory_font = if paper {
            NSFont::fontWithName_size(&NSString::from_str("Songti SC"), 22.0)
                .unwrap_or_else(|| NSFont::systemFontOfSize(22.0))
        } else {
            NSFont::systemFontOfSize_weight(15.0, unsafe { NSFontWeightSemibold })
        };
        c.directory_title.setFont(Some(&directory_font));
        c.directory_title.setFrame(rect(
            18.0,
            if paper { 35.0 } else { 28.0 },
            context - 31.0,
            if paper { 33.0 } else { 25.0 },
        ));
        c.directory_title
            .setTextColor(Some(&native_theme::color(p.ink)));
        c.caption.setFrame(rect(
            18.0,
            if paper { 77.0 } else { 59.0 },
            context - 31.0,
            14.0,
        ));
        let mono = NSFont::monospacedSystemFontOfSize_weight(9.0, unsafe { NSFontWeightRegular });
        let caption_color = native_theme::color(p.faint);
        let kern = NSNumber::new_f64(1.5);
        let attributes = NSDictionary::from_slices(
            &[
                unsafe { NSFontAttributeName },
                unsafe { NSForegroundColorAttributeName },
                unsafe { NSKernAttributeName },
            ],
            &[
                &*mono as &AnyObject,
                &*caption_color as &AnyObject,
                &*kern as &AnyObject,
            ],
        );
        let caption = unsafe {
            NSAttributedString::initWithString_attributes(
                NSAttributedString::alloc(),
                &NSString::from_str("CONTENTS"),
                Some(&attributes),
            )
        };
        c.caption.setAttributedStringValue(&caption);
        c.directory_status
            .setFrame(rect(30.0, bounds.size.height - 37.0, context - 40.0, 18.0));
        c.directory_status
            .setTextColor(Some(&native_theme::color(p.muted)));
        c.directory
            .ivars()
            .dot
            .set(Some(NSPoint::new(18.0, bounds.size.height - 32.0)));
        let horizontal_native = !studio && !paper;
        let horizontal_y = current_top
            + if horizontal_native {
                if narrow {
                    49.0
                } else {
                    52.0
                }
            } else {
                18.0
            };
        let tab_total = c
            .tabs
            .iter()
            .map(|tab| {
                tab.ivars()
                    .title
                    .setFont(Some(&NSFont::systemFontOfSize_weight(12.5, unsafe {
                        NSFontWeightMedium
                    })));
                tab.ivars().title.intrinsicContentSize().width + 16.0
            })
            .sum::<f64>()
            + (c.tabs.len() - 1) as f64 * 2.0;
        let mut tab_x = if context == 0.0 && !horizontal_native {
            x + width - tab_total - 3.0
        } else {
            x
        };
        c.header.ivars().horizontal_tabs.set(
            (context == 0.0 && !horizontal_native)
                .then(|| rect(tab_x - x - 3.0, horizontal_y - 3.0, tab_total + 6.0, 31.0)),
        );
        for (index, tab) in c.tabs.iter().enumerate() {
            let title = &tab.ivars().title;
            title.setFont(Some(&NSFont::systemFontOfSize_weight(
                if context == 0.0 { 12.5 } else { 11.0 },
                unsafe { NSFontWeightMedium },
            )));
            let tab_width = if context == 0.0 {
                title.intrinsicContentSize().width + 16.0
            } else {
                context - 18.0
            };
            button_frame(
                tab,
                if context == 0.0 {
                    rect(
                        tab_x,
                        horizontal_y,
                        tab_width,
                        if horizontal_native { 34.0 } else { 25.0 },
                    )
                } else {
                    rect(
                        9.0,
                        if paper { 110.0 } else { 92.0 } + index as f64 * 39.0,
                        tab_width,
                        34.0,
                    )
                },
            );
            tab_x += tab_width + 2.0;
            c.tab_numbers[index].setHidden(context == 0.0);
            c.tab_numbers[index].setFrame(rect(9.0, 8.0, 20.0, 18.0));
            c.tab_numbers[index].setTextColor(Some(&native_theme::color(p.faint)));
            if context != 0.0 {
                title.setFrame(rect(32.0, 7.0, tab_width - 39.0, 20.0));
            } else {
                title.setFrame(rect(
                    10.0,
                    if horizontal_native { 7.0 } else { 2.5 },
                    tab_width - 20.0,
                    20.0,
                ));
            }
            title.setAlignment(if context == 0.0 {
                NSTextAlignment::Center
            } else {
                NSTextAlignment::Left
            });
            title.setTextColor(Some(&native_theme::color(
                if self.ivars().active.get() == index {
                    if context != 0.0 || horizontal_native {
                        p.accent
                    } else {
                        p.ink
                    }
                } else {
                    p.muted
                },
            )));
            tab.ivars().palette.set(p);
            tab.ivars().directory.set(context != 0.0);
            tab.ivars().selected.set(self.ivars().active.get() == index);
            tab.ivars()
                .underline
                .set(context == 0.0 && !studio && !paper);
            NSView::setNeedsDisplay(tab, true);
        }
        c.scroll
            .setFrame(rect(context, 0.0, pane, bounds.size.height));
        let mut y = top + header_height + 24.0;
        let mut offsets = Vec::new();
        for (index, card) in c.cards.iter().enumerate() {
            // The shortcut toolbar is taller than a plain card heading.
            let heading_height = if paper {
                65.0
            } else if index == 0 {
                if studio {
                    60.0
                } else {
                    68.0
                }
            } else if studio {
                45.0
            } else {
                54.0
            };
            offsets.push(y);
            let inner = if paper { 0.0 } else { 23.0 };
            let content_width = width - inner * 2.0;
            card.surface.ivars().palette.set(p);
            card.surface.ivars().paper.set(paper);
            card.surface.ivars().studio.set(studio);
            let heading_font = if paper {
                NSFont::fontWithName_size(&NSString::from_str("Songti SC"), 23.0)
                    .unwrap_or_else(|| NSFont::systemFontOfSize(23.0))
            } else {
                NSFont::systemFontOfSize_weight(if studio { 12.0 } else { 13.0 }, unsafe {
                    NSFontWeightSemibold
                })
            };
            card.heading.setFont(Some(&heading_font));
            card.heading.setTextColor(Some(&native_theme::color(p.ink)));
            card.heading.setFrame(rect(
                inner + if !paper && !studio { 0.0 } else { 24.0 },
                if index == 0 && !paper {
                    (heading_height - 20.0) * 0.5
                } else if studio {
                    13.0
                } else {
                    17.0
                },
                content_width,
                if paper { 32.0 } else { 22.0 },
            ));
            card.number.setHidden(!paper && !studio);
            card.number
                .setFont(Some(&NSFont::monospacedSystemFontOfSize_weight(
                    10.0,
                    unsafe { NSFontWeightRegular },
                )));
            card.number
                .setTextColor(Some(&native_theme::color(p.faint)));
            card.number
                .setFrame(rect(inner, if paper { 25.0 } else { 16.0 }, 20.0, 18.0));
            let mut lines = vec![(heading_height - 1.0, 0.0)];
            let mut row_y = heading_height;
            if index == 0 {
                let toolbar = self.ivars().hotkeys.toolbar_view();
                toolbar.setFrame(rect(
                    (width - inner - 156.0).max(inner + 80.0),
                    (heading_height - 32.0) * 0.5,
                    156.0,
                    32.0,
                ));
                self.ivars().hotkeys.setFrame(rect(
                    0.0,
                    heading_height,
                    width,
                    self.ivars().hotkeys.preferred_height(),
                ));
                row_y += self.ivars().hotkeys.preferred_height();
            }
            for (row_index, row) in card.rows.iter().enumerate() {
                let new_group = row_index == 0 || row.group_start;
                if new_group && !paper {
                    lines.push((row_y, 0.0));
                    row_y += 1.0;
                }
                let divider = if new_group { 0.0 } else { 1.0 };
                let padding: f64 = if paper {
                    16.0
                } else if studio {
                    9.0
                } else {
                    13.0
                };
                let row_min: f64 = if paper {
                    67.0
                } else if studio {
                    53.0
                } else {
                    62.0
                };
                let choice = matches!(row.control, Control::Choice(_));
                let control_height = if choice { 42.0 } else { 20.0 };
                let label_height = if row.description.is_some() {
                    37.0
                } else {
                    18.0
                };
                let inline_height = match row.control {
                    Control::Choice(_) => 42.0,
                    Control::Range(..) => 24.0,
                    Control::Toggle(_) => 28.0,
                };
                let height = if narrow {
                    padding * 2.0 + label_height + 10.0 + inline_height + divider
                } else {
                    (padding * 2.0
                        + if row.description.is_some() {
                            37.0
                        } else {
                            control_height
                        }
                        + divider)
                        .max(row_min)
                };
                if !new_group {
                    lines.push((row_y, inner));
                }
                row.label.setTextColor(Some(&native_theme::color(p.ink)));
                row.label.setFrame(rect(
                    inner,
                    row_y
                        + if narrow || row.description.is_some() {
                            padding + divider
                        } else {
                            (height - 20.0) * 0.5
                        },
                    if narrow {
                        content_width
                    } else {
                        (content_width - 254.0).max(40.0)
                    },
                    20.0,
                ));
                if let Some(description) = &row.description {
                    description.setTextColor(Some(&native_theme::color(p.muted)));
                    description.setFrame(rect(
                        inner,
                        row_y + padding + divider + 21.0,
                        (content_width - 60.0).max(0.0),
                        18.0,
                    ));
                }
                let cy = row_y
                    + if narrow {
                        padding + divider + label_height + 10.0
                    } else {
                        divider + (height - divider - control_height) * 0.5
                    };
                match &row.control {
                    Control::Toggle(button) => button_frame(
                        button,
                        rect(
                            if narrow {
                                inner
                            } else {
                                inner + content_width - 36.0
                            },
                            cy + if narrow { 8.0 } else { 4.0 },
                            36.0,
                            20.0,
                        ),
                    ),
                    Control::Choice(button) => button_frame(
                        button,
                        rect(
                            if narrow {
                                inner
                            } else {
                                inner + content_width - 230.0
                            },
                            cy,
                            230.0,
                            42.0,
                        ),
                    ),
                    Control::Range(range, value) => {
                        let rx = if narrow {
                            inner
                        } else {
                            inner + content_width - 270.0
                        };
                        range.setFrame(rect(rx, cy + if narrow { 5.0 } else { 3.0 }, 210.0, 20.0));
                        value.setAlignment(NSTextAlignment::Right);
                        value.setFrame(rect(rx + 222.0, cy, 48.0, 20.0));
                    }
                }
                row_y += height;
            }
            *card.surface.ivars().lines.borrow_mut() = lines;
            card.surface.setFrame(rect(
                page_left + margin,
                y,
                width,
                row_y + if paper { 0.0 } else { 1.0 },
            ));
            card.surface.setNeedsDisplay(true);
            y += card.surface.frame().size.height
                + if paper {
                    25.0
                } else if studio {
                    15.0
                } else {
                    20.0
                };
        }
        let last_gap = if paper {
            25.0
        } else if studio {
            15.0
        } else {
            20.0
        };
        c.document.setFrame(rect(
            0.0,
            0.0,
            pane,
            y - last_gap
                + if paper {
                    64.0
                } else if studio {
                    48.0
                } else {
                    56.0
                },
        ));
        *self.ivars().offsets.borrow_mut() = offsets;
        c.scroll
            .contentView()
            .scrollToPoint(NSPoint::new(0.0, clip_top));
        c.scroll.reflectScrolledClipView(&c.scroll.contentView());
        self.ivars().layouting.set(false);
        self.setNeedsDisplay(true);
    }
    fn header_height(&self) -> f64 {
        let general = &self.ivars().projection.borrow().general;
        let native = general.interface_theme != "paper" && general.interface_theme != "studio";
        let shell = self.shell_width();
        if native {
            if shell <= 700.0 || self.ivars().viewport_width.get().unwrap_or(shell) <= 700.0 {
                104.0
            } else {
                107.0
            }
        } else {
            82.0
        }
    }
    pub(crate) fn jump(&self, id: &str) {
        let Some(index) = self
            .ivars()
            .kind
            .sections()
            .iter()
            .position(|(key, _)| *key == id)
        else {
            return;
        };
        self.reflow();
        self.ivars().layouting.set(true);
        if let Some(c) = self.ivars().contents.borrow().as_ref() {
            let offset = self.ivars().offsets.borrow()[index];
            let max = (c.document.bounds().size.height
                - c.scroll.contentView().bounds().size.height)
                .max(0.0);
            let target = (offset - self.header_height() - 16.0).max(0.0).min(max);
            c.scroll
                .contentView()
                .scrollToPoint(NSPoint::new(0.0, target));
            c.scroll.reflectScrolledClipView(&c.scroll.contentView());
            self.ivars()
                .navigation_offset
                .set(Some(c.scroll.contentView().bounds().origin.y));
        }
        self.ivars().layouting.set(false);
        self.ivars().active.set(index);
        self.reflow();
    }
    fn did_scroll(&self) {
        if self.ivars().layouting.get() {
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
        let mut active = 0;
        for (index, y) in self.ivars().offsets.borrow().iter().enumerate() {
            if *y <= offset + self.header_height() + 24.0 {
                active = index;
            }
        }
        self.ivars().active.set(active);
        self.reflow();
    }
    pub(crate) fn dispose(&self) {
        if !self.ivars().disposed.replace(true) {
            unsafe {
                NSNotificationCenter::defaultCenter().removeObserver(self);
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn test_toggle(&self, field: FeatureField) {
        let button = self
            .ivars()
            .contents
            .borrow()
            .as_ref()
            .unwrap()
            .cards
            .iter()
            .flat_map(|c| &c.rows)
            .find_map(|row| {
                if row.field == field {
                    match &row.control {
                        Control::Toggle(button) => Some(button.clone()),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .unwrap();
        unsafe {
            button.performClick(None);
        }
    }
    #[cfg(test)]
    pub(crate) fn test_choose(&self, field: FeatureField, value: &str) {
        let (index, choice) = self
            .ivars()
            .contents
            .borrow()
            .as_ref()
            .unwrap()
            .cards
            .iter()
            .enumerate()
            .flat_map(|(ci, c)| c.rows.iter().enumerate().map(move |(ri, r)| (ci, ri, r)))
            .find_map(|(ci, ri, row)| {
                if row.field == field {
                    Some((
                        (ci * 100 + ri) as isize,
                        field
                            .choices()
                            .iter()
                            .position(|(id, _)| *id == value)
                            .unwrap(),
                    ))
                } else {
                    None
                }
            })
            .unwrap();
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                self.mtm().alloc(),
                &NSString::from_str(value),
                Some(sel!(selectFeatureChoice:)),
                &NSString::new(),
            )
        };
        item.setTag(index * 100 + choice as isize);
        unsafe {
            objc2_app_kit::NSApplication::sharedApplication(self.mtm()).sendAction_to_from(
                sel!(selectFeatureChoice:),
                Some(self),
                Some(&item),
            );
        }
    }
    #[cfg(test)]
    pub(crate) fn test_range(&self, field: FeatureField, value: f64) {
        let range = self
            .ivars()
            .contents
            .borrow()
            .as_ref()
            .unwrap()
            .cards
            .iter()
            .flat_map(|c| &c.rows)
            .find_map(|row| {
                if row.field == field {
                    match &row.control {
                        Control::Range(range, _) => Some(range.clone()),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .unwrap();
        range.setDoubleValue(value);
        unsafe {
            objc2_app_kit::NSApplication::sharedApplication(self.mtm()).sendAction_to_from(
                sel!(changeFeatureRange:),
                Some(self),
                Some(&range),
            );
        }
    }
    #[cfg(test)]
    pub(crate) fn test_projection(&self) -> FeatureProjection {
        self.ivars().projection.borrow().clone()
    }
    #[cfg(test)]
    pub(crate) fn test_kind(&self) -> FeatureKind {
        self.ivars().kind
    }
    #[cfg(test)]
    pub(crate) fn test_scroll_to(&self, y: f64) {
        {
            let c = self.ivars().contents.borrow();
            let c = c.as_ref().unwrap();
            c.scroll.contentView().scrollToPoint(NSPoint::new(0.0, y));
        }
        self.did_scroll();
    }
    #[cfg(test)]
    pub(crate) fn test_active_section(&self) -> &'static str {
        self.ivars().kind.sections()[self.ivars().active.get()].0
    }
    #[cfg(test)]
    pub(crate) fn test_section_offset(&self, id: &str) -> f64 {
        let index = self
            .ivars()
            .kind
            .sections()
            .iter()
            .position(|(key, _)| *key == id)
            .unwrap();
        self.ivars().offsets.borrow()[index]
    }
    #[cfg(test)]
    pub(crate) fn test_geometry(&self) -> serde_json::Value {
        let c = self.ivars().contents.borrow();
        let c = c.as_ref().unwrap();
        let frame = |r: NSRect| serde_json::json!({"x":r.origin.x,"y":r.origin.y,"width":r.size.width,"height":r.size.height});
        let view_frame =
            |view: &NSView| frame(self.convertRect_fromView(view.bounds(), Some(view)));
        serde_json::json!({
            "cards":c.cards.iter().map(|card|view_frame(&card.surface)).collect::<Vec<_>>(),
            "rows":c.cards.iter().enumerate().map(|(index,card)| {
                if index == 0 { return self.ivars().hotkeys.test_geometry(self); }
                card.rows.iter().map(|row| {
                    let control = match &row.control {
                        Control::Toggle(button) | Control::Choice(button) => view_frame(button),
                        Control::Range(range,_) => {
                            let b = range.bounds();
                            frame(self.convertRect_fromView(rect(0.0,(b.size.height-6.0)*0.5,b.size.width,6.0),Some(range)))
                        }
                    };
                    serde_json::json!({"controls":[control],"label":view_frame(&row.label),"text":row.label.stringValue().to_string()})
                }).collect()
            }).collect::<Vec<_>>(),
            "tabs":c.tabs.iter().map(|tab| view_frame(tab)).collect::<Vec<_>>(),
            "header":view_frame(&c.header),
            "shellWidth":self.shell_width(),
            "viewportWidth":self.ivars().viewport_width.get().unwrap_or_else(|| self.shell_width()),
            "active":self.ivars().active.get(),
            "clip":frame(c.scroll.contentView().bounds()),
            "offsets":self.ivars().offsets.borrow().clone()
        })
    }
}
impl Drop for FeatureView {
    fn drop(&mut self) {
        self.dispose();
    }
}
