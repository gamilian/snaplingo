use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject, Sel};
use objc2::{
    define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSApplicationActivationOptions, NSBackingStoreType, NSBezelStyle, NSBezierPath, NSBorderType,
    NSBox, NSBoxType, NSButton, NSCellImagePosition, NSColor, NSEvent, NSFloatingWindowLevel,
    NSFont, NSImage, NSImageScaling, NSImageView, NSMenuItem, NSPanel, NSPopUpButton,
    NSRunningApplication, NSScreen, NSScrollerStyle, NSTextAlignment, NSTextDelegate, NSTextField,
    NSTextInputClient, NSTextView, NSTextViewDelegate, NSTitlePosition, NSView, NSWindow,
    NSWindowAnimationBehavior, NSWindowCollectionBehavior, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{
    NSData, NSNotification, NSObjectProtocol, NSPoint, NSRange, NSRect, NSSize, NSString,
    NSUserDefaults,
};

use snaplingo_core::result::OcrCopyToken;
use snaplingo_core::result::ResultWindowMode;
use snaplingo_core::result::ResultWindowOrigin;
use snaplingo_core::result::ResultWindowSnapshot;
use snaplingo_core::result::TranslationRow;

pub(crate) use super::super::controller::Action;
use snaplingo_ui::result::{
    ResultInput, ResultLayout, ResultMeasurements, ResultRow, ResultRowLayout, ResultTarget,
    ResultUi, LANGUAGES,
};
use snaplingo_ui::{Rect, Size, Snapshot};

define_class!(
    #[unsafe(super(NSPanel))]
    #[name = "SnapLingoResultPanel"]
    #[thread_kind = MainThreadOnly]
    struct ResultPanel;
    unsafe impl NSObjectProtocol for ResultPanel {}
    impl ResultPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool { true }
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool { false }
    }
);

define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoResultDocument"]
    #[thread_kind = MainThreadOnly]
    struct DocumentView;
    unsafe impl NSObjectProtocol for DocumentView {}
    impl DocumentView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
    }
);

define_class!(
    #[unsafe(super(NSTextField))]
    #[name = "SnapLingoResultLabel"]
    #[thread_kind = MainThreadOnly]
    struct Label;
    unsafe impl NSObjectProtocol for Label {}
    impl Label {
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView { std::ptr::null_mut() }
    }
);

struct SourceIvars {
    dispatch: Rc<dyn Fn(Action)>,
    updating: Rc<Cell<bool>>,
    input_depth: Cell<usize>,
    escape_in_composition: Cell<bool>,
}
define_class!(
    #[unsafe(super(NSTextView))]
    #[name = "SnapLingoResultSource"]
    #[thread_kind = MainThreadOnly]
    #[ivars = SourceIvars]
    struct SourceView;
    unsafe impl NSObjectProtocol for SourceView {}
    impl SourceView {
        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            self.ivars().escape_in_composition.set(event.keyCode() == 53 && self.hasMarkedText());
            unsafe { let _: () = msg_send![super(self), keyDown: event]; }
            self.ivars().escape_in_composition.set(false);
        }
        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self, text: &AnyObject, range: NSRange) {
            self.ivars().input_depth.set(self.ivars().input_depth.get() + 1);
            unsafe { let _: () = msg_send![super(self), insertText: text, replacementRange: range]; }
            self.ivars().input_depth.set(self.ivars().input_depth.get() - 1);
            self.edited();
        }
        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        fn mark_text(&self, text: &AnyObject, selected: NSRange, replacement: NSRange) {
            self.ivars().input_depth.set(self.ivars().input_depth.get() + 1);
            unsafe { let _: () = msg_send![super(self), setMarkedText: text, selectedRange: selected, replacementRange: replacement]; }
            self.ivars().input_depth.set(self.ivars().input_depth.get() - 1);
            self.edited();
        }
        #[unsafe(method(didChangeText))]
        fn did_change_text(&self) {
            unsafe { let _: () = msg_send![super(self), didChangeText]; }
            self.edited();
        }
        #[unsafe(method(unmarkText))]
        fn unmark_text(&self) {
            unsafe { let _: () = msg_send![super(self), unmarkText]; }
            self.edited();
        }
    }
);
impl SourceView {
    fn edited(&self) {
        if !self.ivars().updating.get() && self.ivars().input_depth.get() == 0 {
            (self.ivars().dispatch)(Action::Edit(
                self.string().to_string(),
                self.hasMarkedText(),
            ));
        }
    }
}

struct RootIvars {
    dispatch: Rc<dyn Fn(Action)>,
    updating: Rc<Cell<bool>>,
    pinned: Cell<bool>,
    dragging: Cell<bool>,
    dialog_open: Cell<bool>,
    hide_on_blur: Cell<bool>,
    visible_since: Cell<Option<Instant>>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoResultView"]
    #[thread_kind = MainThreadOnly]
    #[ivars = RootIvars]
    struct ResultView;
    unsafe impl NSObjectProtocol for ResultView {}
    unsafe impl NSTextDelegate for ResultView {}
    unsafe impl NSTextViewDelegate for ResultView {
        #[unsafe(method(textView:doCommandBySelector:))]
        fn text_command(&self, text: &NSTextView, command: Sel) -> bool {
            if command == sel!(cancelOperation:) {
                let composing = text.hasMarkedText() || text.downcast_ref::<SourceView>()
                    .is_some_and(|source| source.ivars().escape_in_composition.get());
                if !composing { (self.ivars().dispatch)(Action::Close); }
                true
            } else {
                false
            }
        }
    }
    unsafe impl NSWindowDelegate for ResultView {
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, _window: &NSWindow) -> bool {
            (self.ivars().dispatch)(Action::Close);
            false
        }
        #[unsafe(method(windowDidResignKey:))]
        fn resigned_key(&self, _notification: &NSNotification) {
            let state = self.ivars();
            if !state.updating.get() && !state.pinned.get() && !state.dragging.get()
                && !state.dialog_open.get() && state.hide_on_blur.get()
                && state.visible_since.get().is_some_and(|start| start.elapsed() >= Duration::from_millis(300))
            {
                (state.dispatch)(Action::Close);
            }
        }
    }
    impl ResultView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool { true }
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool { true }
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let bounds = self.bounds();
            let outline = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(bounds, 12.0, 12.0);
            NSColor::windowBackgroundColor().setFill();
            outline.fill();
            NSColor::separatorColor().setStroke();
            outline.setLineWidth(1.0);
            outline.stroke();
            let line = NSBezierPath::bezierPath();
            line.moveToPoint(NSPoint::new(0.0, 48.0));
            line.lineToPoint(NSPoint::new(bounds.size.width, 48.0));
            line.stroke();
        }
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            if point.y > 48.0 { return; }
            if let Some(window) = self.window() {
                self.ivars().dragging.set(true);
                window.performWindowDragWithEvent(event);
                self.ivars().dragging.set(false);
                let frame = window.frame();
                let scale = window.backingScaleFactor();
                let height = primary_height(self.mtm());
                (self.ivars().dispatch)(Action::Moved(
                    (frame.origin.x * scale).round() as i32,
                    ((height - frame.origin.y - frame.size.height) * scale).round() as i32,
                ));
            }
        }
        #[unsafe(method(cancelOperation:))]
        fn cancel_operation(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            let composing = self.window().and_then(|window| window.firstResponder())
                .is_some_and(|responder| responder.downcast_ref::<SourceView>().is_some_and(|source| source.hasMarkedText() || source.ivars().escape_in_composition.get()));
            if !composing { (self.ivars().dispatch)(Action::Close); }
        }
        #[unsafe(method(performAction:))]
        fn perform_action(&self, sender: &NSButton) { activate(sender.tag()); }
        #[unsafe(method(languageChanged:))]
        fn language_changed(&self, sender: &NSPopUpButton) {
            let code = sender.selectedItem().and_then(|item| item.representedObject())
                .and_then(|value| value.downcast_ref::<NSString>().map(ToString::to_string));
            if let Some(code) = code {
                (self.ivars().dispatch)(if sender.tag() == 0 { Action::SourceLanguage(code) } else { Action::TargetLanguage(code) });
            }
        }
        #[unsafe(method(copyFragment:))]
        fn copy_fragment(&self, sender: &NSMenuItem) {
            if let Some(value) = sender.representedObject().and_then(|value| value.downcast_ref::<NSString>().map(ToString::to_string)) {
                (self.ivars().dispatch)(Action::Copy(value));
            }
        }
    }
);

fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(x, y),
        NSSize::new(width.max(0.0), height.max(0.0)),
    )
}
fn place(view: &NSView, x: f64, y: f64, width: f64, height: f64) {
    let next = rect(x, y, width, height);
    if view.frame() != next {
        view.setFrame(next);
    }
}
fn apply(view: &NSView, frame: Rect) {
    place(
        view,
        f64::from(frame.x),
        f64::from(frame.y),
        f64::from(frame.width),
        f64::from(frame.height),
    );
}
fn label(parent: &NSView, text: &str, size: f64) -> Retained<Label> {
    let field = Label::alloc(parent.mtm()).set_ivars(());
    let field: Retained<Label> =
        unsafe { msg_send![super(field), initWithFrame: rect(0.0, 0.0, 1.0, 20.0)] };
    field.setBezeled(false);
    field.setEditable(false);
    field.setSelectable(false);
    field.setDrawsBackground(false);
    field.setFont(Some(&NSFont::systemFontOfSize(size)));
    field.setTextColor(Some(&NSColor::secondaryLabelColor()));
    field.setStringValue(&NSString::from_str(text));
    parent.addSubview(&field);
    field
}
fn set_label(field: &NSTextField, text: &str) {
    if field.stringValue().to_string() != text {
        field.setStringValue(&NSString::from_str(text));
    }
}
fn button(
    parent: &NSView,
    root: &ResultView,
    title: &str,
    symbol: Option<&str>,
    tag: isize,
) -> Retained<NSButton> {
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(root),
            Some(sel!(performAction:)),
            root.mtm(),
        )
    };
    button.setTag(tag);
    button.setFont(Some(&NSFont::systemFontOfSize(12.0)));
    button.setBezelStyle(NSBezelStyle::Push);
    button.setToolTip(Some(&NSString::from_str(title)));
    if let Some(symbol) = symbol {
        if let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str(symbol),
            Some(&NSString::from_str(title)),
        ) {
            button.setImage(Some(&image));
            button.setImagePosition(NSCellImagePosition::ImageOnly);
            button.setBordered(false);
        }
    }
    parent.addSubview(&button);
    button
}
fn card(parent: &NSView) -> (Retained<NSBox>, Retained<DocumentView>) {
    let mtm = parent.mtm();
    let box_ = NSBox::initWithFrame(NSBox::alloc(mtm), rect(0.0, 0.0, 1.0, 1.0));
    box_.setBoxType(NSBoxType::Custom);
    box_.setTitlePosition(NSTitlePosition::NoTitle);
    box_.setContentViewMargins(NSSize::new(0.0, 0.0));
    box_.setCornerRadius(8.0);
    box_.setBorderWidth(1.0);
    box_.setBorderColor(&NSColor::separatorColor());
    box_.setFillColor(&NSColor::textBackgroundColor());
    let document = document_view(mtm, rect(0.0, 0.0, 1.0, 1.0));
    box_.setContentView(Some(&document));
    parent.addSubview(&box_);
    (box_, document)
}
fn document_view(mtm: MainThreadMarker, frame: NSRect) -> Retained<DocumentView> {
    let document = DocumentView::alloc(mtm).set_ivars(());
    unsafe { msg_send![super(document), initWithFrame: frame] }
}
fn configure_text(view: &NSTextView, root: &ResultView, editable: bool) {
    view.setRichText(false);
    view.setEditable(editable);
    view.setSelectable(true);
    view.setDrawsBackground(false);
    view.setUsesFontPanel(false);
    view.setAllowsUndo(editable);
    view.setHorizontallyResizable(false);
    view.setVerticallyResizable(false);
    view.setTextContainerInset(NSSize::new(10.0, 8.0));
    view.setDelegate(Some(ProtocolObject::from_ref(root)));
    view.setAutomaticQuoteSubstitutionEnabled(false);
    view.setAutomaticDashSubstitutionEnabled(false);
    view.setAutomaticTextReplacementEnabled(false);
    view.setAutomaticSpellingCorrectionEnabled(false);
    view.setContinuousSpellCheckingEnabled(false);
    if let Some(container) = unsafe { view.textContainer() } {
        container.setLineFragmentPadding(0.0);
        container.setWidthTracksTextView(true);
    }
}
fn text_view(parent: &NSView, root: &ResultView) -> Retained<NSTextView> {
    let view = NSTextView::initWithFrame(NSTextView::alloc(root.mtm()), rect(0.0, 0.0, 1.0, 1.0));
    configure_text(&view, root, false);
    parent.addSubview(&view);
    view
}
fn update_text(view: &NSTextView, text: &str, source: bool, force: bool) {
    if force && view.hasMarkedText() {
        view.unmarkText();
    }
    if !view.hasMarkedText() {
        if view.string().to_string() != text {
            view.setString(&NSString::from_str(text));
        }
        let size = f64::from(snaplingo_ui::result::text_size(text, source));
        if view.font().is_none_or(|font| font.pointSize() != size) {
            view.setFont(Some(&NSFont::systemFontOfSize(size)));
        }
    }
}
fn text_height(view: &NSTextView, width: f64, minimum: f64) -> f64 {
    view.setFrameSize(NSSize::new(width, view.frame().size.height.max(minimum)));
    let mut height = minimum;
    if let Some(container) = unsafe { view.textContainer() } {
        container.setContainerSize(NSSize::new((width - 20.0).max(1.0), 1_000_000.0));
        // The layout manager and text container are owned by this NSTextView.
        if let Some(layout) = unsafe { view.layoutManager() } {
            layout.ensureLayoutForTextContainer(&container);
            height = height.max(
                layout
                    .usedRectForTextContainer(&container)
                    .size
                    .height
                    .ceil()
                    + 18.0,
            );
        }
    }
    height
}
fn language_select(parent: &NSView, root: &ResultView, tag: isize) -> Retained<NSPopUpButton> {
    let button = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(root.mtm()),
        rect(0.0, 0.0, 1.0, 1.0),
        false,
    );
    button.setTag(tag);
    button.setFont(Some(&NSFont::systemFontOfSize(13.0)));
    button.setAutoenablesItems(false);
    unsafe {
        button.setTarget(Some(root));
        button.setAction(Some(sel!(languageChanged:)));
    }
    for (code, title) in LANGUAGES {
        add_language(&button, code, title);
    }
    parent.addSubview(&button);
    button
}
fn add_language(button: &NSPopUpButton, code: &str, title: &str) {
    button.addItemWithTitle(&NSString::from_str(title));
    unsafe {
        button
            .lastItem()
            .unwrap()
            .setRepresentedObject(Some(&NSString::from_str(code)));
    }
}
fn select_language(button: &NSPopUpButton, code: &str) {
    let index = (0..button.numberOfItems())
        .find(|index| {
            button
                .itemAtIndex(*index)
                .and_then(|item| item.representedObject())
                .is_some_and(|value| {
                    value
                        .downcast_ref::<NSString>()
                        .is_some_and(|value| value.to_string() == code)
                })
        })
        .unwrap_or_else(|| {
            add_language(
                button,
                code,
                if code == "zh" { "中文 Chinese" } else { code },
            );
            button.numberOfItems() - 1
        });
    if button.indexOfSelectedItem() != index {
        button.selectItemAtIndex(index);
    }
}

struct TranslationCard {
    id: String,
    box_: Retained<NSBox>,
    title: Retained<NSButton>,
    status: Retained<Label>,
    buttons: [Retained<NSButton>; 4],
    text: Retained<NSTextView>,
}
impl TranslationCard {
    fn new(parent: &NSView, root: &ResultView, row: &TranslationRow, index: usize) -> Self {
        let (box_, content) = card(parent);
        let tag = 100 + index as isize * 10;
        let title = button(&content, root, &row.name, None, tag);
        title.setBordered(false);
        title.setAlignment(NSTextAlignment::Left);
        title.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
        title.setContentTintColor(Some(&NSColor::labelColor()));
        let status = label(&content, "", 11.0);
        let buttons = [
            button(&content, root, "重试", Some("arrow.clockwise"), tag + 1),
            button(&content, root, "朗读", Some("speaker.wave.2"), tag + 2),
            button(&content, root, "收藏", Some("star"), tag + 3),
            button(&content, root, "复制", Some("doc.on.doc"), tag + 4),
        ];
        let text = text_view(&content, root);
        Self {
            id: row.id.clone(),
            box_,
            title,
            status,
            buttons,
            text,
        }
    }
    fn layout(
        &self,
        row: &ResultRow,
        pending_favorite: bool,
        layout: &ResultRowLayout,
        expanded: bool,
    ) {
        let presentation = row.presentation(pending_favorite);
        set_label(&self.status, presentation.status);
        let status_color = if presentation.error {
            NSColor::systemRedColor()
        } else {
            NSColor::secondaryLabelColor()
        };
        self.status.setTextColor(Some(&status_color));
        let title = format!("{} {}", if expanded { "▾" } else { "▸" }, row.name);
        if self.title.title().to_string() != title {
            self.title.setTitle(&NSString::from_str(&title));
        }
        apply(&self.title, layout.title);
        apply(&self.status, layout.status);
        for ((button, state), frame) in self
            .buttons
            .iter()
            .zip(&presentation.buttons)
            .zip(layout.buttons)
        {
            apply(button, frame);
            button.setEnabled(state.enabled);
            button.setToolTip(Some(&NSString::from_str(state.label)));
        }
        update_text(&self.text, presentation.text, false, false);
        let text_color = if presentation.error {
            NSColor::systemRedColor()
        } else {
            NSColor::labelColor()
        };
        self.text.setTextColor(Some(&text_color));
        self.text.setHidden(!expanded);
        apply(&self.text, layout.text);
        apply(&self.box_, layout.frame);
    }
}

struct NativeResult {
    window: Retained<ResultPanel>,
    root: Retained<ResultView>,
    title: Retained<Label>,
    subtitle: Retained<Label>,
    pin: Retained<NSButton>,
    close: Retained<NSButton>,
    scroll: Retained<objc2_app_kit::NSScrollView>,
    document: Retained<DocumentView>,
    source_box: Retained<NSBox>,
    source: Retained<SourceView>,
    placeholder: Retained<Label>,
    source_meta: Retained<Label>,
    source_buttons: Vec<Retained<NSButton>>,
    collapsed_source: Retained<NSButton>,
    source_language: Retained<NSPopUpButton>,
    target_language: Retained<NSPopUpButton>,
    swap: Retained<NSButton>,
    cards: Vec<TranslationCard>,
    ocr_language: Retained<Label>,
    image_title: Retained<Label>,
    image: Retained<NSImageView>,
    upload: Retained<NSButton>,
    ocr_title: Retained<Label>,
    ocr_box: Retained<NSBox>,
    ocr_text: Retained<NSTextView>,
    ocr_placeholder: Retained<Label>,
    ocr_meta: Retained<Label>,
    ocr_buttons: Vec<Retained<NSButton>>,
    fragments: Retained<NSPopUpButton>,
    error: Retained<NSTextView>,
    decoded_image: Option<Retained<NSImage>>,
    decoded_source: Option<Arc<str>>,
    tokens: Vec<OcrCopyToken>,
    token_text: String,
    ui: ResultUi,
    view: ResultWindowSnapshot,
    defaults: Retained<NSUserDefaults>,
    last_position: String,
}

thread_local! { static WINDOW: RefCell<Option<NativeResult>> = const { RefCell::new(None) }; }

pub(crate) fn open(
    view: ResultWindowSnapshot,
    defaults: Retained<NSUserDefaults>,
    dispatch: impl Fn(Action) + 'static,
) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("Result window must open on the main thread")?;
    if WINDOW.with(|window| window.borrow().is_some()) {
        return Ok(());
    }
    let frame = rect(120.0, 120.0, 660.0, 220.0);
    let window = ResultPanel::alloc(mtm).set_ivars(());
    let window: Retained<ResultPanel> = unsafe {
        msg_send![
            super(window), initWithContentRect: frame,
            styleMask: NSWindowStyleMask::Borderless,
            backing: NSBackingStoreType::Buffered, defer: false,
        ]
    };
    window.setTitle(&NSString::from_str("SnapLingo Result"));
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setHasShadow(true);
    window.setHidesOnDeactivate(false);
    window.setCanHide(false);
    window.setExcludedFromWindowsMenu(true);
    window.setAnimationBehavior(NSWindowAnimationBehavior::None);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::Transient
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    let dispatch: Rc<dyn Fn(Action)> = Rc::new(dispatch);
    let updating = Rc::new(Cell::new(false));
    let root = ResultView::alloc(mtm).set_ivars(RootIvars {
        dispatch: dispatch.clone(),
        updating: updating.clone(),
        pinned: Cell::new(false),
        dragging: Cell::new(false),
        dialog_open: Cell::new(false),
        hide_on_blur: Cell::new(true),
        visible_since: Cell::new(None),
    });
    let root: Retained<ResultView> =
        unsafe { msg_send![super(root), initWithFrame: rect(0.0, 0.0, 660.0, 220.0)] };
    window.setContentView(Some(&root));
    window.setDelegate(Some(ProtocolObject::from_ref(&*root)));
    let pin = button(&root, &root, "固定窗口", Some("pin"), 0);
    let close = button(&root, &root, "关闭", Some("xmark"), 1);
    let title = label(&root, "翻译", 16.0);
    title.setFont(Some(&NSFont::boldSystemFontOfSize(16.0)));
    title.setTextColor(Some(&NSColor::labelColor()));
    let subtitle = label(&root, "", 12.0);
    let scroll = objc2_app_kit::NSScrollView::initWithFrame(
        objc2_app_kit::NSScrollView::alloc(mtm),
        rect(0.0, 48.0, 660.0, 172.0),
    );
    scroll.setBorderType(NSBorderType::NoBorder);
    scroll.setDrawsBackground(false);
    scroll.setHasVerticalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setScrollerStyle(NSScrollerStyle::Overlay);
    let document = document_view(mtm, rect(0.0, 0.0, 636.0, 100.0));
    scroll.setDocumentView(Some(&document));
    root.addSubview(&scroll);
    let (source_box, source_content) = card(&document);
    let source = SourceView::alloc(mtm).set_ivars(SourceIvars {
        dispatch,
        updating,
        input_depth: Cell::new(0),
        escape_in_composition: Cell::new(false),
    });
    let source: Retained<SourceView> =
        unsafe { msg_send![super(source), initWithFrame: rect(0.0, 0.0, 632.0, 52.0)] };
    configure_text(&source, &root, true);
    source_content.addSubview(&source);
    let placeholder = label(&source_content, "输入需要翻译的文本…", 14.0);
    let source_meta = label(&source_content, "", 11.0);
    let source_buttons = vec![
        button(&source_content, &root, "折叠", None, 2),
        button(&source_content, &root, "收藏全部结果", Some("star"), 7),
        button(&source_content, &root, "复制原文", Some("doc.on.doc"), 3),
        button(
            &source_content,
            &root,
            "朗读原文",
            Some("speaker.wave.2"),
            4,
        ),
        button(
            &source_content,
            &root,
            "翻译 / 重试",
            Some("arrow.clockwise"),
            5,
        ),
        button(&source_content, &root, "清空原文", Some("xmark.circle"), 6),
    ];
    let collapsed_source = button(&document, &root, "展开原文", None, 2);
    let source_language = language_select(&document, &root, 0);
    let target_language = language_select(&document, &root, 1);
    let swap = button(
        &document,
        &root,
        "切换语言",
        Some("arrow.left.arrow.right"),
        13,
    );
    let ocr_language = label(&document, "", 11.0);
    let image_title = label(&document, "原图", 13.0);
    let image = NSImageView::initWithFrame(NSImageView::alloc(mtm), rect(0.0, 0.0, 1.0, 1.0));
    image.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
    document.addSubview(&image);
    let upload = button(&document, &root, "上传图片 OCR", None, 8);
    let ocr_title = label(&document, "识别文本", 13.0);
    let (ocr_box, ocr_content) = card(&document);
    let ocr_text = text_view(&ocr_content, &root);
    let ocr_placeholder = label(&ocr_content, "OCR 结果…", 14.0);
    let ocr_meta = label(&ocr_content, "", 11.0);
    let ocr_buttons = vec![
        button(&ocr_content, &root, "朗读", Some("speaker.wave.2"), 10),
        button(&ocr_content, &root, "收藏", Some("star"), 11),
        button(&ocr_content, &root, "复制", Some("doc.on.doc"), 9),
        button(&ocr_content, &root, "清空", Some("xmark.circle"), 12),
    ];
    let fragments = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        rect(0.0, 0.0, 1.0, 1.0),
        true,
    );
    fragments.setFont(Some(&NSFont::systemFontOfSize(12.0)));
    fragments.setAutoenablesItems(false);
    ocr_content.addSubview(&fragments);
    let error = text_view(&document, &root);
    error.setTextColor(Some(&NSColor::systemRedColor()));
    let initial = view.clone();
    WINDOW.with(|all| {
        *all.borrow_mut() = Some(NativeResult {
            window,
            root,
            title,
            subtitle,
            pin,
            close,
            scroll,
            document,
            source_box,
            source,
            placeholder,
            source_meta,
            source_buttons,
            collapsed_source,
            source_language,
            target_language,
            swap,
            cards: vec![],
            ocr_language,
            image_title,
            image,
            upload,
            ocr_title,
            ocr_box,
            ocr_text,
            ocr_placeholder,
            ocr_meta,
            ocr_buttons,
            fragments,
            error,
            decoded_image: None,
            decoded_source: None,
            tokens: vec![],
            token_text: String::new(),
            ui: ResultUi::new(super::super::presentation::project(
                &initial,
                Snapshot::default(),
            )),
            view: initial,
            defaults,
            last_position: String::new(),
        })
    });
    update(view);
    Ok(())
}

impl NativeResult {
    fn release_image(&mut self) {
        self.image.setImage(None);
        self.decoded_image = None;
        self.decoded_source = None;
    }
    fn collapsed_setting(&self) -> &str {
        match self.view.origin {
            ResultWindowOrigin::Selection => {
                &self.view.preferences.translation.selection_input_state
            }
            ResultWindowOrigin::Screenshot => {
                &self.view.preferences.translation.screenshot_input_state
            }
            _ => "expanded",
        }
    }
    fn source_key(&self) -> Retained<NSString> {
        let origin = match self.view.origin {
            ResultWindowOrigin::Selection => "selection",
            ResultWindowOrigin::Screenshot => "screenshot",
            ResultWindowOrigin::Input => "input",
            ResultWindowOrigin::Ocr => "ocr",
        };
        NSString::from_str(&format!("snaplingo.result-window.source-input.{origin}"))
    }
    fn restore_collapsed(&mut self) {
        let collapsed = match self.collapsed_setting() {
            "collapsed" => true,
            "last" => self
                .defaults
                .stringForKey(&self.source_key())
                .is_some_and(|value| value.to_string() == "collapsed"),
            _ => false,
        };
        self.ui.restore_source_collapsed(collapsed);
    }

    fn render(&mut self, new_presentation: bool) {
        let view = &self.view;
        self.root.ivars().updating.set(true);
        self.root.ivars().dialog_open.set(view.dialog_open);
        self.root
            .ivars()
            .hide_on_blur
            .set(view.preferences.translation.hide_on_blur);
        let translation = view.mode == ResultWindowMode::Translation;
        let theme = match view.preferences.general.theme.as_str() {
            "light" => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameAqua }),
            "dark" => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }),
            _ => None,
        };
        self.window.setAppearance(theme.as_deref());
        self.window
            .setLevel(if view.preferences.translation.window_always_on_top {
                NSFloatingWindowLevel
            } else {
                0
            });
        if !translation && self.token_text != view.ocr_text {
            let tokens = super::tokens::tokens(&view.ocr_text);
            self.fragments.removeAllItems();
            self.fragments
                .addItemWithTitle(&NSString::from_str(&format!(
                    "推荐复制片段 · {} 个",
                    tokens.len()
                )));
            for token in &tokens {
                self.fragments
                    .addItemWithTitle(&NSString::from_str(&format!(
                        "{}  {}",
                        token.label, token.value
                    )));
                let item = self.fragments.lastItem().unwrap();
                unsafe {
                    item.setTarget(Some(&self.root));
                    item.setAction(Some(sel!(copyFragment:)));
                    item.setRepresentedObject(Some(&NSString::from_str(&token.value)));
                }
            }
            self.tokens = tokens;
            self.token_text = view.ocr_text.clone();
        }
        let mut model = super::super::presentation::project(view, Snapshot::default());
        model.ocr_tokens = self
            .tokens
            .iter()
            .map(|token| snaplingo_ui::result::ResultCopyToken {
                label: token.label.clone(),
                value: token.value.clone(),
            })
            .collect();
        self.ui.replace(model);
        let presentation = self.ui.snapshot().presentation();
        set_label(&self.title, presentation.title);
        set_label(&self.subtitle, &presentation.subtitle);
        self.source_box
            .setHidden(!translation || self.ui.source_collapsed());
        self.collapsed_source
            .setHidden(!translation || !self.ui.source_collapsed());
        self.source_language.setHidden(!translation);
        self.target_language.setHidden(!translation);
        self.swap.setHidden(!translation);
        self.ocr_language
            .setHidden(translation || view.ocr_language.is_none());
        self.image_title.setHidden(translation);
        self.image
            .setHidden(translation || view.ocr_image.is_none());
        self.upload.setHidden(translation);
        self.ocr_title.setHidden(translation);
        self.ocr_box.setHidden(translation);
        self.error.setHidden(view.error.is_none());
        if self
            .cards
            .iter()
            .map(|card| &card.id)
            .ne(view.rows.iter().map(|row| &row.id))
        {
            for card in self.cards.drain(..) {
                card.box_.removeFromSuperview();
            }
            self.cards = view
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| TranslationCard::new(&self.document, &self.root, row, index))
                .collect();
        }
        for card in &self.cards {
            card.box_.setHidden(!translation);
        }
        if translation {
            update_text(
                &self.source,
                &view.source_text,
                true,
                new_presentation || !view.composing,
            );
            self.source.setTextColor(Some(&NSColor::labelColor()));
            self.placeholder.setHidden(!view.source_text.is_empty());
            set_label(&self.source_meta, &presentation.source_metadata);
            for (button, state) in self.source_buttons.iter().zip(&presentation.source_buttons) {
                button.setEnabled(state.enabled);
                button.setToolTip(Some(&NSString::from_str(state.label)));
            }
            self.source_buttons[4]
                .setEnabled(presentation.source_buttons[4].enabled && !self.source.hasMarkedText());
            self.collapsed_source
                .setTitle(&NSString::from_str(&presentation.collapsed_source));
            select_language(&self.source_language, &view.source_lang);
            select_language(&self.target_language, &view.target_lang);
        } else {
            update_text(&self.ocr_text, &view.ocr_text, true, false);
            self.ocr_placeholder.setHidden(!view.ocr_text.is_empty());
            set_label(&self.image_title, presentation.image_title);
            set_label(&self.ocr_title, presentation.ocr_title);
            self.ocr_text.setTextColor(Some(&NSColor::labelColor()));
            set_label(&self.ocr_meta, &presentation.ocr_metadata);
            set_label(&self.ocr_language, &presentation.ocr_language);
            for (button, state) in self.ocr_buttons.iter().zip(&presentation.ocr_buttons) {
                button.setEnabled(state.enabled);
                button.setToolTip(Some(&NSString::from_str(state.label)));
            }
            self.upload.setEnabled(presentation.upload.enabled);
            self.upload
                .setTitle(&NSString::from_str(presentation.upload.label));
        }

        if !translation && self.decoded_source != view.ocr_image {
            self.image.setImage(None);
            self.decoded_image = view.ocr_image.as_deref().and_then(|text| {
                let base64 = text.split_once(',').map_or(text, |(_, data)| data);
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(base64)
                    .ok()?;
                NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(&bytes))
            });
            self.decoded_source = view.ocr_image.clone();
            self.image.setImage(self.decoded_image.as_deref());
        } else if translation {
            self.image.setImage(None);
            self.decoded_image = None;
            self.decoded_source = None;
        }
        self.fragments.setHidden(self.tokens.is_empty());
        if let Some(error) = &view.error {
            update_text(&self.error, error, false, false);
        }

        let position = position_setting(view);
        let screen = if new_presentation || position != self.last_position {
            placement_screen(view, self.root.mtm())
        } else {
            self.window
                .screen()
                .unwrap_or_else(|| placement_screen(view, self.root.mtm()))
        };
        let work = screen.visibleFrame();
        let width = (if translation {
            view.preferences.translation.window_width as f64
        } else {
            660.0
        })
        .clamp(300.0, 1000.0)
        .min(work.size.width - 16.0);
        let content_width = width - 24.0;
        let measured = ResultMeasurements {
            source: if translation {
                text_height(&self.source, content_width - 4., 52.) as f32
            } else {
                0.
            },
            ocr: if translation {
                0.
            } else {
                text_height(&self.ocr_text, content_width - 4., 72.) as f32
            },
            error: if view.error.is_some() {
                text_height(&self.error, content_width, 40.) as f32
            } else {
                0.
            },
            rows: self
                .cards
                .iter()
                .filter(|_| translation)
                .zip(&self.ui.snapshot().rows)
                .map(|(card, row)| {
                    update_text(
                        &card.text,
                        row.presentation(view.favorite_pending).text,
                        false,
                        false,
                    );
                    text_height(&card.text, content_width - 4., 44.) as f32
                })
                .collect(),
            image: self.decoded_image.as_ref().map(|image| Size {
                width: image.size().width as f32,
                height: image.size().height as f32,
            }),
        };
        let layout = self.ui.layout(width as f32, &measured);
        if let Some(source) = &layout.source {
            apply(&self.source_box, source.frame);
            apply(&self.source, source.text);
            apply(&self.placeholder, source.placeholder);
            apply(&self.source_meta, source.metadata);
            for (button, frame) in self.source_buttons.iter().zip(source.buttons) {
                apply(button, frame);
            }
        }
        if let Some(frame) = layout.collapsed_source {
            apply(&self.collapsed_source, frame);
        }
        if let Some([source, swap, target]) = layout.languages {
            apply(&self.source_language, source);
            apply(&self.swap, swap);
            apply(&self.target_language, target);
        }
        if let Some(frame) = layout.error {
            apply(&self.error, frame);
        }
        for ((row, card), frame) in self
            .ui
            .snapshot()
            .rows
            .iter()
            .zip(&self.cards)
            .zip(&layout.rows)
        {
            card.layout(
                row,
                view.favorite_pending,
                frame,
                self.ui.row_expanded(&row.id),
            );
        }
        if let Some(ocr) = &layout.ocr {
            if let Some(frame) = ocr.language {
                apply(&self.ocr_language, frame);
            }
            apply(&self.image_title, ocr.image_title);
            if let Some(frame) = ocr.image {
                apply(&self.image, frame);
            }
            apply(&self.upload, ocr.upload);
            apply(&self.ocr_title, ocr.title);
            apply(&self.ocr_box, ocr.frame);
            apply(&self.ocr_text, ocr.text);
            apply(&self.ocr_placeholder, ocr.placeholder);
            apply(&self.fragments, ocr.fragments);
            apply(&self.ocr_meta, ocr.metadata);
            for (button, frame) in self.ocr_buttons.iter().zip(ocr.buttons) {
                apply(button, frame);
            }
        }
        let y = f64::from(layout.content_height);
        let maximum = if translation {
            work.size.height * view.preferences.translation.max_window_height_ratio as f64 / 100.0
        } else {
            work.size.height - 32.0
        };
        let height = f64::from(layout.window_height(maximum as f32));
        let mut frame = self.window.frame();
        frame.origin.y += frame.size.height - height;
        frame.size = NSSize::new(width, height);
        if new_presentation || position != self.last_position {
            frame.origin = placement_origin(view, &screen, frame.size, self.root.mtm());
            self.last_position = position.to_owned();
        }
        frame.origin.x = frame.origin.x.clamp(
            work.origin.x,
            (work.origin.x + work.size.width - width).max(work.origin.x),
        );
        frame.origin.y = frame.origin.y.clamp(
            work.origin.y,
            (work.origin.y + work.size.height - height).max(work.origin.y),
        );
        if self.window.frame() != frame {
            self.window.setFrame_display(frame, true);
        }
        let header = ResultLayout::header(Size {
            width: width as f32,
            height: height as f32,
        });
        apply(&self.pin, header.pin);
        apply(&self.title, header.title);
        apply(&self.subtitle, header.subtitle);
        apply(&self.close, header.close);
        apply(&self.scroll, header.viewport);
        place(&self.document, 0., 0., content_width, y.max(height - 72.));
        self.root.ivars().updating.set(false);
    }
}

pub(crate) fn update(view: ResultWindowSnapshot) {
    WINDOW.with(|all| {
        let mut all = all.borrow_mut();
        let Some(window) = all.as_mut() else {
            return;
        };
        if view.revision < window.view.revision {
            return;
        }
        let fresh = view.presentation != window.view.presentation
            || view.visible && !window.window.isVisible();
        let collapse_changed = view.origin != window.view.origin
            || view.preferences.translation.selection_input_state
                != window.view.preferences.translation.selection_input_state
            || view.preferences.translation.screenshot_input_state
                != window.view.preferences.translation.screenshot_input_state;
        window.view = view;
        if !window.view.visible {
            window.root.ivars().updating.set(true);
            window.window.orderOut(None);
            window.root.ivars().pinned.set(false);
            window.ui.replace(super::super::presentation::project(
                &window.view,
                Snapshot::default(),
            ));
            window.pin.setToolTip(Some(&NSString::from_str("固定窗口")));
            if let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                &NSString::from_str("pin"),
                Some(&NSString::from_str("固定窗口")),
            ) {
                window.pin.setImage(Some(&image));
            }
            window.root.ivars().visible_since.set(None);
            window.release_image();
            window.root.ivars().updating.set(false);
            return;
        }
        if fresh || collapse_changed {
            window.restore_collapsed();
        }
        window.render(fresh);
        if fresh {
            window.root.ivars().updating.set(true);
            NSRunningApplication::currentApplication()
                .activateWithOptions(NSApplicationActivationOptions::empty());
            window.window.makeKeyAndOrderFront(None);
            window.root.ivars().visible_since.set(Some(Instant::now()));
            if window.view.mode == ResultWindowMode::Translation && !window.ui.source_collapsed() {
                window.window.makeFirstResponder(Some(&window.source));
            } else {
                window.window.makeFirstResponder(Some(&window.ocr_text));
            }
            window.root.ivars().updating.set(false);
        }
    });
}

fn activate(tag: isize) {
    let effect = WINDOW.with(|all| {
        let mut all = all.borrow_mut();
        let window = all.as_mut()?;
        let target = match tag {
            0 => ResultTarget::Pin,
            1 => ResultTarget::Close,
            2 => ResultTarget::ToggleSource,
            3 => ResultTarget::CopySource,
            4 => ResultTarget::SpeakSource,
            5 => ResultTarget::Translate,
            6 => ResultTarget::ClearSource,
            7 => ResultTarget::FavoriteAll,
            8 => ResultTarget::FileOcr,
            9 => ResultTarget::CopyOcr,
            10 => ResultTarget::SpeakOcr,
            11 => ResultTarget::FavoriteOcr,
            12 => ResultTarget::ClearOcr,
            13 => ResultTarget::Swap,
            tag if tag >= 100 => {
                let row = window.view.rows.get(((tag - 100) / 10) as usize)?;
                match (tag - 100) % 10 {
                    0 => ResultTarget::ToggleRow(row.id.clone()),
                    1 => ResultTarget::Retry(row.id.clone()),
                    2 => ResultTarget::SpeakResult(row.id.clone()),
                    3 => ResultTarget::FavoriteResult(row.id.clone()),
                    4 => ResultTarget::CopyResult(row.id.clone()),
                    _ => return None,
                }
            }
            _ => return None,
        };
        let intent = window
            .ui
            .update(ResultInput::Activate(target))
            .into_iter()
            .next();
        if tag == 0 {
            let pinned = window.ui.pinned();
            window.root.ivars().pinned.set(pinned);
            window.pin.setToolTip(Some(&NSString::from_str(if pinned {
                "取消固定"
            } else {
                "固定窗口"
            })));
            if let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                &NSString::from_str(if pinned { "pin.fill" } else { "pin" }),
                Some(&NSString::from_str("固定窗口")),
            ) {
                window.pin.setImage(Some(&image));
            }
            return None;
        }
        if tag == 2 {
            unsafe {
                window.defaults.setObject_forKey(
                    Some(&NSString::from_str(if window.ui.source_collapsed() {
                        "collapsed"
                    } else {
                        "expanded"
                    })),
                    &window.source_key(),
                );
            }
        }
        if tag == 2 || tag >= 100 && (tag - 100) % 10 == 0 {
            window.render(false);
            if tag == 2 && !window.ui.source_collapsed() {
                window.window.makeFirstResponder(Some(&window.source));
            }
            return None;
        }
        let action = super::super::presentation::action(intent?, &window.view)?;
        Some((window.root.ivars().dispatch.clone(), action))
    });
    if let Some((dispatch, action)) = effect {
        dispatch(action);
    }
}

fn position_setting(view: &ResultWindowSnapshot) -> &str {
    if view.mode == ResultWindowMode::Ocr {
        &view.preferences.ocr.window_position
    } else if view.origin == ResultWindowOrigin::Input {
        &view.preferences.translation.input_window_position
    } else {
        &view.preferences.translation.selection_window_position
    }
}
fn primary_height(mtm: MainThreadMarker) -> f64 {
    NSScreen::screens(mtm)
        .firstObject()
        .map_or(0.0, |screen| screen.frame().size.height)
}
fn contains(frame: NSRect, point: NSPoint) -> bool {
    point.x >= frame.origin.x
        && point.x <= frame.origin.x + frame.size.width
        && point.y >= frame.origin.y
        && point.y <= frame.origin.y + frame.size.height
}
fn placement_screen(view: &ResultWindowSnapshot, mtm: MainThreadMarker) -> Retained<NSScreen> {
    let screens = NSScreen::screens(mtm);
    let primary = primary_height(mtm);
    let point = NSEvent::mouseLocation();
    screens
        .iter()
        .find(|screen| {
            if position_setting(view) == "last-position" {
                if let (Some(x), Some(y)) = (
                    view.preferences.general.last_result_window_x,
                    view.preferences.general.last_result_window_y,
                ) {
                    let scale = screen.backingScaleFactor();
                    return contains(
                        screen.frame(),
                        NSPoint::new(x as f64 / scale, primary - y as f64 / scale),
                    );
                }
            }
            contains(screen.frame(), point)
        })
        .unwrap_or_else(|| NSScreen::mainScreen(mtm).expect("AppKit has a display"))
}
fn placement_origin(
    view: &ResultWindowSnapshot,
    screen: &NSScreen,
    size: NSSize,
    mtm: MainThreadMarker,
) -> NSPoint {
    let work = screen.visibleFrame();
    let cursor = NSEvent::mouseLocation();
    match position_setting(view) {
        "cursor" => NSPoint::new(cursor.x, cursor.y - size.height),
        "below-cursor" => NSPoint::new(cursor.x, cursor.y - size.height - 12.0),
        "last-position"
            if view.preferences.general.last_result_window_x.is_some()
                && view.preferences.general.last_result_window_y.is_some() =>
        {
            let scale = screen.backingScaleFactor();
            NSPoint::new(
                view.preferences.general.last_result_window_x.unwrap() as f64 / scale,
                primary_height(mtm)
                    - view.preferences.general.last_result_window_y.unwrap() as f64 / scale
                    - size.height,
            )
        }
        _ => NSPoint::new(
            work.origin.x + (work.size.width - size.width) / 2.0,
            work.origin.y + (work.size.height - size.height) / 2.0,
        ),
    }
}

#[cfg(test)]
pub(crate) fn test_destroy() {
    let window = WINDOW.with(|all| all.borrow_mut().take());
    if let Some(window) = window {
        window.window.setDelegate(None);
        window.window.orderOut(None);
        window.window.close();
    }
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct TestState {
    pub visible: bool,
    pub frame: NSRect,
    pub level: isize,
    pub document_height: f64,
    pub source: String,
    pub source_height: f64,
    pub composing: bool,
    pub collapsed: bool,
    pub pinned: bool,
    pub ocr_text: String,
    pub decoded_image: bool,
    pub fragments: usize,
    pub results: Vec<String>,
}
#[cfg(test)]
pub(crate) fn test_state() -> TestState {
    WINDOW.with(|all| {
        let all = all.borrow();
        let window = all.as_ref().unwrap();
        TestState {
            visible: window.window.isVisible(),
            frame: window.window.frame(),
            level: window.window.level(),
            document_height: window.document.frame().size.height,
            source: window.source.string().to_string(),
            source_height: window.source.frame().size.height,
            composing: window.source.hasMarkedText(),
            collapsed: window.ui.source_collapsed(),
            pinned: window.root.ivars().pinned.get(),
            ocr_text: window.ocr_text.string().to_string(),
            decoded_image: window.decoded_image.is_some(),
            fragments: window.tokens.len(),
            results: window
                .cards
                .iter()
                .map(|card| card.text.string().to_string())
                .collect(),
        }
    })
}
#[cfg(test)]
pub(crate) fn test_pointer_event(origin: NSPoint, point: NSPoint, down: bool) -> (NSPoint, isize) {
    let (window, root) = WINDOW.with(|all| {
        let all = all.borrow();
        let window = all.as_ref().unwrap();
        (window.window.clone(), window.root.clone())
    });
    window.setFrameOrigin(origin);
    let location = root.convertPoint_toView(point, None);
    let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        if down { objc2_app_kit::NSEventType::LeftMouseDown } else { objc2_app_kit::NSEventType::LeftMouseUp },
        location, objc2_app_kit::NSEventModifierFlags::empty(), 0., window.windowNumber(), None, 0, 1, if down { 1. } else { 0. },
    ).unwrap();
    if down {
        root.mouseDown(&event);
    } else {
        root.mouseUp(&event);
    }
    (location, window.windowNumber())
}
#[cfg(test)]
pub(crate) fn test_click(tag: isize) {
    let button = WINDOW.with(|all| {
        let all = all.borrow();
        let window = all.as_ref().unwrap();
        if tag == 0 {
            return Some(window.pin.clone());
        }
        if tag == 1 {
            return Some(window.close.clone());
        }
        if tag == 2 && window.ui.source_collapsed() {
            return Some(window.collapsed_source.clone());
        }
        if tag == 8 {
            return Some(window.upload.clone());
        }
        if tag == 13 {
            return Some(window.swap.clone());
        }
        window
            .source_buttons
            .iter()
            .chain(&window.ocr_buttons)
            .find(|button| button.tag() == tag)
            .cloned()
            .or_else(|| {
                window
                    .cards
                    .iter()
                    .flat_map(|card| std::iter::once(&card.title).chain(card.buttons.iter()))
                    .find(|button| button.tag() == tag)
                    .cloned()
            })
    });
    if let Some(button) = button {
        unsafe {
            button.performClick(None);
        }
    }
}
#[cfg(test)]
pub(crate) fn test_source(text: &str, marked: bool) {
    let source = WINDOW.with(|all| all.borrow().as_ref().unwrap().source.clone());
    let string = NSString::from_str(text);
    let range = objc2_foundation::NSRange::new(0, source.string().length());
    unsafe {
        if marked {
            source.setMarkedText_selectedRange_replacementRange(
                &string,
                objc2_foundation::NSRange::new(string.length(), 0),
                range,
            );
        } else {
            source.insertText_replacementRange(&string, range);
        }
    }
}
#[cfg(test)]
pub(crate) fn test_escape() {
    let window = WINDOW.with(|all| all.borrow().as_ref().unwrap().window.clone());
    let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        objc2_app_kit::NSEventType::KeyDown, NSPoint::new(0.0, 0.0), objc2_app_kit::NSEventModifierFlags::empty(),
        0.0, window.windowNumber(), None, &NSString::from_str("\u{1b}"), &NSString::from_str("\u{1b}"), false, 53,
    ).unwrap();
    window.sendEvent(&event);
}
#[cfg(test)]
pub(crate) fn test_blur(age: Duration, dragging: bool) {
    let root = WINDOW.with(|all| all.borrow().as_ref().unwrap().root.clone());
    root.ivars().visible_since.set(Some(Instant::now() - age));
    root.ivars().dragging.set(dragging);
    let notification = unsafe {
        NSNotification::notificationWithName_object(
            &NSString::from_str("NSWindowDidResignKeyNotification"),
            None,
        )
    };
    unsafe {
        let _: () = msg_send![&root, windowDidResignKey: &*notification];
    }
    root.ivars().dragging.set(false);
}
#[cfg(test)]
pub(crate) fn test_copy_fragment(index: usize) {
    let (root, item) = WINDOW.with(|all| {
        let all = all.borrow();
        let window = all.as_ref().unwrap();
        (
            window.root.clone(),
            window.fragments.itemAtIndex(index as isize + 1).unwrap(),
        )
    });
    unsafe {
        let _: () = msg_send![&root, copyFragment: &*item];
    }
}
#[cfg(test)]
pub(crate) fn test_snapshot() -> Vec<u8> {
    WINDOW.with(|all| {
        crate::integrations::system::native_view_snapshot::snapshot(
            &all.borrow().as_ref().unwrap().root,
        )
    })
}

#[cfg(test)]
pub(crate) fn test_geometry() -> serde_json::Value {
    WINDOW.with_borrow(|window| {
        let window = window.as_ref().unwrap();
        let frame = |view: &NSView| {
            let rect = view.frame();
            serde_json::json!([rect.origin.x, rect.origin.y, rect.size.width, rect.size.height])
        };
        let buttons = |buttons: &[Retained<NSButton>]| {
            buttons.iter().map(|button| serde_json::json!({
                "frame": frame(button), "enabled": button.isEnabled(),
                "label": button.toolTip().map(|label| label.to_string()),
            })).collect::<Vec<_>>()
        };
        serde_json::json!({
            "width": window.window.frame().size.width,
            "height": window.window.frame().size.height,
            "title": frame(&window.title), "subtitle": frame(&window.subtitle),
            "pin": frame(&window.pin), "close": frame(&window.close),
            "scroll": frame(&window.scroll), "document": frame(&window.document),
            "source": {
                "frame": frame(&window.source_box), "text": frame(&window.source),
                "font": window.source.font().map(|font| font.pointSize()),
                "placeholder": frame(&window.placeholder), "metadata": frame(&window.source_meta),
                "buttons": buttons(&window.source_buttons), "collapsed": window.ui.source_collapsed(),
                "toggle": frame(&window.collapsed_source),
                "languages": [frame(&window.source_language), frame(&window.swap), frame(&window.target_language)],
            },
            "rows": window.cards.iter().map(|card| serde_json::json!({
                "id": card.id, "frame": frame(&card.box_), "title": frame(&card.title),
                "status": frame(&card.status), "text": frame(&card.text),
                "font": card.text.font().map(|font| font.pointSize()),
                "expanded": window.ui.row_expanded(&card.id), "buttons": buttons(&card.buttons),
            })).collect::<Vec<_>>(),
            "ocr": {
                "language": frame(&window.ocr_language), "image_title": frame(&window.image_title),
                "image": frame(&window.image), "upload": frame(&window.upload),
                "title": frame(&window.ocr_title), "frame": frame(&window.ocr_box),
                "text": frame(&window.ocr_text), "placeholder": frame(&window.ocr_placeholder),
                "metadata": frame(&window.ocr_meta), "fragments": frame(&window.fragments),
                "font": window.ocr_text.font().map(|font| font.pointSize()),
                "buttons": buttons(&window.ocr_buttons),
            },
        })
    })
}
