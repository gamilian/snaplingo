use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSBackingStoreType, NSBezierPath, NSBorderType, NSButton, NSColor,
    NSControlTextEditingDelegate, NSEventModifierFlags, NSFont, NSFontWeightRegular,
    NSFontWeightSemibold, NSGraphicsContext, NSMutableParagraphStyle, NSPanel,
    NSParagraphStyleAttributeName, NSScrollView, NSScrollerStyle, NSShadow, NSTableColumn,
    NSTableView, NSTableViewColumnAutoresizingStyle, NSTableViewDataSource, NSTableViewDelegate,
    NSTableViewGridLineStyle, NSTableViewSelectionHighlightStyle, NSTableViewStyle,
    NSTextAlignment, NSTextField, NSTextFieldDelegate, NSTextView, NSView, NSWindow,
    NSWindowDelegate, NSWindowOrderingMode, NSWindowStyleMask,
};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};
use serde::Deserialize;

use super::super::native_theme::{self, Palette};
use crate::domain::GeneralSettings;

#[derive(Deserialize)]
struct Component {
    name: String,
    version: String,
    license: String,
    source: String,
    notices: Vec<String>,
}

#[derive(Deserialize)]
struct Inventory {
    components: Vec<Component>,
    texts: HashMap<String, String>,
}

fn inventory() -> &'static Inventory {
    static INVENTORY: OnceLock<Inventory> = OnceLock::new();
    INVENTORY.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../../assets/licenses/thirdPartyNotices.json"
        ))
        .expect("Bundled third-party notices are valid")
    })
}

fn filtered(query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    inventory()
        .components
        .iter()
        .enumerate()
        .filter_map(|(index, component)| {
            format!("{} {}", component.name, component.license)
                .to_lowercase()
                .contains(&query)
                .then_some(index)
        })
        .collect()
}

fn notice_text(component: &Component) -> String {
    if component.notices.is_empty() {
        format!(
            "该依赖的许可标识为 {}，完整许可请查看来源仓库。",
            component.license
        )
    } else {
        component
            .notices
            .iter()
            .map(|id| inventory().texts[id].as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
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

struct Contents {
    title: Retained<NSTextField>,
    caption: Retained<NSTextField>,
    close: Retained<NSButton>,
    search: Retained<NSTextField>,
    clear_search: Retained<NSButton>,
    table: Retained<NSTableView>,
    table_scroll: Retained<NSScrollView>,
    headings: Vec<Retained<NSTextField>>,
    empty: Retained<NSTextField>,
    detail_scroll: Retained<NSScrollView>,
    detail: Retained<Document>,
    back: Retained<NSButton>,
    name: Retained<NSTextField>,
    version: Retained<NSTextField>,
    source: Retained<NSButton>,
    text: Retained<NSTextView>,
    extra_text: RefCell<Vec<Retained<NSTextView>>>,
    error: Retained<NSTextField>,
}

struct NoticesIvars {
    palette: Palette,
    filtered: RefCell<Vec<usize>>,
    selected: Cell<Option<usize>>,
    contents: RefCell<Option<Contents>>,
}

struct DocumentIvars {
    palette: Palette,
    blocks: RefCell<Vec<NSRect>>,
}
define_class!(
    #[unsafe(super(NSPanel))]
    #[name = "SnapLingoNoticesPanel"]
    #[thread_kind = MainThreadOnly]
    struct NoticesPanel;
    unsafe impl NSObjectProtocol for NoticesPanel {}
    impl NoticesPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool { true }
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool { false }
    }
);

define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoNoticesDocument"]
    #[thread_kind = MainThreadOnly]
    #[ivars = DocumentIvars]
    struct Document;
    unsafe impl NSObjectProtocol for Document {}
    impl Document {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(drawRect:))]
        fn draw(&self, _: NSRect) {
            for &block in self.ivars().blocks.borrow().iter() { fill(block, 10.0, self.ivars().palette.sunken); }
        }
    }
);

define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoNoticesView"]
    #[thread_kind = MainThreadOnly]
    #[ivars = NoticesIvars]
    struct NoticesView;
    unsafe impl NSObjectProtocol for NoticesView {}
    unsafe impl NSControlTextEditingDelegate for NoticesView {
        #[unsafe(method(controlTextDidChange:))]
        fn search_changed(&self, _: &NSNotification) {
            let query = self.ivars().contents.borrow().as_ref().unwrap().search.stringValue().to_string();
            self.search(&query);
        }
    }
    unsafe impl NSTextFieldDelegate for NoticesView {}
    unsafe impl NSTableViewDataSource for NoticesView {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn rows(&self, _: &NSTableView) -> isize { self.ivars().filtered.borrow().len() as isize }
    }
    unsafe impl NSTableViewDelegate for NoticesView {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn cell(&self, _: &NSTableView, column: Option<&NSTableColumn>, row: isize) -> Option<Retained<NSView>> {
            self.table_cell(column, row)
        }
    }
    unsafe impl NSWindowDelegate for NoticesView {
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, _: &NSWindow) -> bool { self.dismiss(); false }
    }
    impl NoticesView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(drawRect:))]
        fn draw(&self, _: NSRect) {
            let p = self.ivars().palette;
            let bounds = self.bounds();
            fill(bounds, 13.0, p.surface);
            native_theme::color(p.line).setStroke();
            NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect(0.5, 0.5, bounds.size.width - 1.0, bounds.size.height - 1.0), 13.0, 13.0).stroke();
            fill(rect(0.0, 71.0, bounds.size.width, 1.0), 0.0, p.line);
            let search = rect(21.0, 92.0, bounds.size.width - 42.0, 36.0);
            fill(search, 10.0, p.canvas);
            native_theme::color(p.line).setStroke();
            NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(search, 10.0, 10.0).stroke();
            if self.ivars().contents.borrow().as_ref().is_some_and(|c| !c.search.stringValue().is_empty()) {
                fill(rect(bounds.size.width - 47.0, 105.0, 10.0, 10.0), 5.0, p.faint);
            }
            if self.ivars().selected.get().is_none() {
                NSGraphicsContext::saveGraphicsState_class();
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect(21.5, 144.5, bounds.size.width - 43.0, bounds.size.height - 165.0), 10.0, 10.0).addClip();
                fill(rect(22.0, 145.0, bounds.size.width - 44.0, 39.0), 0.0, p.sunken);
                NSGraphicsContext::restoreGraphicsState_class();
                native_theme::color(p.line).setStroke();
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect(21.5, 144.5, bounds.size.width - 43.0, bounds.size.height - 165.0), 10.0, 10.0).stroke();
            }
        }
        #[unsafe(method(closeNotices:))]
        fn close_notices(&self, _: &AnyObject) { self.dismiss(); }
        #[unsafe(method(cancelOperation:))]
        fn cancel_operation(&self, _: Option<&AnyObject>) { self.dismiss(); }
        #[unsafe(method(selectComponent:))]
        fn select_component(&self, sender: &NSButton) { self.select(sender.tag() as usize); }
        #[unsafe(method(backToList:))]
        fn back_to_list(&self, _: &AnyObject) { self.ivars().selected.set(None); self.update(); }
        #[unsafe(method(clearSearch:))]
        fn clear_search(&self, _: &AnyObject) {
            if let Some(c) = self.ivars().contents.borrow().as_ref() { c.search.setStringValue(&NSString::from_str("")); }
            self.search("");
        }
        #[unsafe(method(openSource:))]
        fn open_source(&self, _: &AnyObject) {
            if let Some(index) = self.ivars().selected.get() {
                if let Err(error) = super::super::external_open::open_url(&inventory().components[index].source) {
                    if let Some(contents) = self.ivars().contents.borrow().as_ref() {
                        contents.error.setStringValue(&NSString::from_str(&error.to_string()));
                    }
                }
            }
        }
    }
);

fn label(parent: &NSView, value: &str, size: f64, color: [u8; 3]) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&NSString::from_str(value), parent.mtm());
    field.setFont(Some(&NSFont::systemFontOfSize(size)));
    field.setTextColor(Some(&native_theme::color(color)));
    field.setSelectable(false);
    parent.addSubview(&field);
    field
}

fn button(
    parent: &NSView,
    target: &NoticesView,
    title: &str,
    action: objc2::runtime::Sel,
) -> Retained<NSButton> {
    let button = NSButton::initWithFrame(parent.mtm().alloc(), rect(0.0, 0.0, 1.0, 1.0));
    button.setTitle(&NSString::from_str(title));
    button.setBordered(false);
    button.setAlignment(NSTextAlignment::Left);
    button.setFont(Some(&NSFont::systemFontOfSize(10.5)));
    unsafe {
        button.setTarget(Some(target));
        button.setAction(Some(action));
    }
    parent.addSubview(&button);
    button
}

fn scroll(parent: &NSView, frame: NSRect) -> Retained<NSScrollView> {
    let scroll = NSScrollView::initWithFrame(parent.mtm().alloc(), frame);
    scroll.setHasVerticalScroller(true);
    scroll.setScrollerStyle(NSScrollerStyle::Overlay);
    scroll.setDrawsBackground(false);
    scroll.setBorderType(NSBorderType::NoBorder);
    parent.addSubview(&scroll);
    scroll
}

impl NoticesView {
    fn dismiss(&self) {
        let _keep_alive: Retained<Self> = self.into();
        close();
    }

    fn table_cell(&self, column: Option<&NSTableColumn>, row: isize) -> Option<Retained<NSView>> {
        let index = *self
            .ivars()
            .filtered
            .borrow()
            .get(usize::try_from(row).ok()?)?;
        let component = &inventory().components[index];
        let column = column?;
        let key = column.identifier().to_string();
        let cell = NSView::initWithFrame(self.mtm().alloc(), rect(0.0, 0.0, column.width(), 40.0));
        let p = self.ivars().palette;
        if key == "name" {
            let name = button(&cell, self, &component.name, sel!(selectComponent:));
            name.setTag(index as isize);
            name.setFrame(rect(14.0, 9.0, column.width() - 24.0, 20.0));
            // Cell buttons remain real AppKit controls for keyboard and accessibility.
            name.setContentTintColor(Some(&native_theme::color(p.accent)));
        } else {
            let text = if key == "version" {
                &component.version
            } else {
                &component.license
            };
            let label = label(&cell, text, 10.5, p.muted);
            label.setFrame(rect(16.0, 10.0, column.width() - 24.0, 20.0));
        }
        Some(cell)
    }

    fn new(mtm: MainThreadMarker, frame: NSRect, p: Palette) -> Retained<Self> {
        let view = Self::alloc(mtm).set_ivars(NoticesIvars {
            palette: p,
            filtered: RefCell::new(filtered("")),
            selected: Cell::new(None),
            contents: RefCell::new(None),
        });
        let view: Retained<Self> = unsafe { msg_send![super(view), initWithFrame: frame] };
        let width = frame.size.width;
        let height = frame.size.height;
        let title = label(&view, "第三方组件与许可", 13.0, p.ink);
        title.setFont(Some(&NSFont::systemFontOfSize_weight(13.0, unsafe {
            NSFontWeightSemibold
        })));
        title.setFrame(rect(25.0, 17.0, width - 110.0, 20.0));
        let caption = label(&view, "查看组件来源和随附许可声明。", 10.5, p.muted);
        caption.setFrame(rect(25.0, 40.0, width - 110.0, 18.0));
        let close = button(&view, &view, "关闭", sel!(closeNotices:));
        close.setFrame(rect(width - 72.0, 20.0, 48.0, 31.0));
        close.setAlignment(NSTextAlignment::Center);
        close.setContentTintColor(Some(&native_theme::color(p.muted)));
        close.setKeyEquivalent(&NSString::from_str("\u{1b}"));
        close.setKeyEquivalentModifierMask(NSEventModifierFlags::empty());
        let search = NSTextField::initWithFrame(mtm.alloc(), rect(33.0, 100.0, width - 66.0, 20.0));
        search.setBezeled(false);
        search.setBordered(false);
        search.setDrawsBackground(false);
        search.setFont(Some(&NSFont::systemFontOfSize(10.5)));
        search.setTextColor(Some(&native_theme::color(p.ink)));
        search.setPlaceholderString(Some(&NSString::from_str("搜索组件或许可证…")));
        unsafe {
            search.setDelegate(Some(ProtocolObject::from_ref(&*view)));
        }
        view.addSubview(&search);
        let clear_search = button(&view, &view, "×", sel!(clearSearch:));
        clear_search.setFont(Some(&NSFont::systemFontOfSize(9.0)));
        clear_search.setAlignment(NSTextAlignment::Center);
        clear_search.setContentTintColor(Some(&native_theme::color(p.canvas)));
        clear_search.setFrame(rect(width - 50.0, 100.0, 16.0, 20.0));
        let table_scroll = scroll(&view, rect(22.0, 184.0, width - 44.0, height - 205.0));
        let table = NSTableView::initWithFrame(mtm.alloc(), table_scroll.bounds());
        table.setStyle(NSTableViewStyle::Plain);
        table.setHeaderView(None);
        table.setRowHeight(40.0);
        table.setIntercellSpacing(NSSize::new(0.0, 0.0));
        table.setBackgroundColor(&native_theme::color(p.surface));
        table.setGridColor(&native_theme::color(p.line));
        table.setGridStyleMask(NSTableViewGridLineStyle::SolidHorizontalGridLineMask);
        table.setSelectionHighlightStyle(NSTableViewSelectionHighlightStyle::None);
        table.setColumnAutoresizingStyle(NSTableViewColumnAutoresizingStyle::NoColumnAutoresizing);
        let mut headings = Vec::new();
        let mut x = 22.0;
        for (id, title, fraction) in [
            ("name", "组件", 0.44),
            ("version", "版本", 0.20),
            ("license", "许可", 0.36),
        ] {
            let column = NSTableColumn::initWithIdentifier(mtm.alloc(), &NSString::from_str(id));
            column.setTitle(&NSString::from_str(title));
            column.setWidth((width - 42.0) * fraction);
            column.setMinWidth(20.0);
            column.setEditable(false);
            table.addTableColumn(&column);
            let heading = label(&view, title, 10.5, p.muted);
            heading.setFrame(rect(x + 16.0, 155.0, column.width() - 24.0, 20.0));
            headings.push(heading);
            x += column.width();
        }
        unsafe {
            table.setDataSource(Some(ProtocolObject::from_ref(&*view)));
            table.setDelegate(Some(ProtocolObject::from_ref(&*view)));
        }
        table_scroll.setDocumentView(Some(&table));
        let empty = label(&view, "没有匹配的组件", 12.0, p.muted);
        empty.setFrame(rect(40.0, 204.0, width - 80.0, 24.0));
        let detail_scroll = scroll(&view, rect(21.0, 145.0, width - 42.0, height - 166.0));
        let detail = Document::alloc(mtm).set_ivars(DocumentIvars {
            palette: p,
            blocks: RefCell::new(Vec::new()),
        });
        let detail: Retained<Document> =
            unsafe { msg_send![super(detail), initWithFrame: rect(0.0, 0.0, width - 40.0, 1.0)] };
        detail_scroll.setDocumentView(Some(&detail));
        let back = button(&detail, &view, "← 返回组件列表", sel!(backToList:));
        back.setContentTintColor(Some(&native_theme::color(p.accent)));
        back.setFrame(rect(0.0, 0.0, 110.0, 24.0));
        let name = label(&detail, "", 12.5, p.ink);
        name.setFont(Some(&NSFont::systemFontOfSize_weight(12.5, unsafe {
            NSFontWeightSemibold
        })));
        name.setFrame(rect(0.0, 33.0, width - 42.0, 22.0));
        let version = label(&detail, "", 12.5, p.muted);
        let source = button(&detail, &view, "", sel!(openSource:));
        source.setFrame(rect(0.0, 59.0, width - 42.0, 20.0));
        source.setContentTintColor(Some(&native_theme::color(p.accent)));
        let text = NSTextView::initWithFrame(mtm.alloc(), rect(0.0, 108.0, width - 40.0, 1.0));
        text.setRichText(false);
        text.setEditable(false);
        text.setSelectable(true);
        text.setDrawsBackground(true);
        text.setBackgroundColor(&native_theme::color(p.sunken));
        text.setTextColor(Some(&native_theme::color(p.muted)));
        text.setFont(Some(
            &NSFont::userFixedPitchFontOfSize(10.5)
                .unwrap_or_else(|| NSFont::systemFontOfSize(10.5)),
        ));
        text.setTextContainerInset(NSSize::new(16.0, 16.0));
        text.setHorizontallyResizable(false);
        text.setVerticallyResizable(false);
        detail.addSubview(&text);
        let error = label(&detail, "", 12.0, p.danger);
        error.setFrame(rect(0.0, 96.0, width - 40.0, 20.0));
        *view.ivars().contents.borrow_mut() = Some(Contents {
            title,
            caption,
            close,
            search,
            clear_search,
            table,
            table_scroll,
            headings,
            empty,
            detail_scroll,
            detail,
            back,
            name,
            version,
            source,
            text,
            extra_text: RefCell::new(Vec::new()),
            error,
        });
        view.update();
        view
    }

    fn reflow(&self, frame: NSRect) {
        self.setFrame(frame);
        let width = frame.size.width;
        let height = frame.size.height;
        let contents = self.ivars().contents.borrow();
        let Some(c) = contents.as_ref() else {
            return;
        };
        c.title.setFrame(rect(25.0, 17.0, width - 110.0, 20.0));
        c.caption.setFrame(rect(25.0, 40.0, width - 110.0, 18.0));
        c.close.setFrame(rect(width - 72.0, 20.0, 48.0, 31.0));
        c.search.setFrame(rect(33.0, 100.0, width - 66.0, 20.0));
        c.clear_search
            .setFrame(rect(width - 50.0, 100.0, 16.0, 20.0));
        c.table_scroll
            .setFrame(rect(22.0, 184.0, width - 44.0, height - 205.0));
        c.empty.setFrame(rect(40.0, 204.0, width - 80.0, 24.0));
        c.detail_scroll
            .setFrame(rect(21.0, 145.0, width - 42.0, height - 166.0));
        c.name.setFrame(rect(0.0, 33.0, width - 42.0, 22.0));
        c.source.setFrame(rect(0.0, 59.0, width - 42.0, 20.0));
        c.error.setFrame(rect(0.0, 96.0, width - 40.0, 20.0));
        drop(contents);
        self.update();
    }

    fn search(&self, query: &str) {
        *self.ivars().filtered.borrow_mut() = filtered(query);
        self.ivars().selected.set(None);
        self.update();
    }

    fn select(&self, index: usize) {
        self.ivars().selected.set(Some(index));
        self.update();
    }

    fn update(&self) {
        let contents = self.ivars().contents.borrow();
        let Some(c) = contents.as_ref() else {
            return;
        };
        let selected = self.ivars().selected.get();
        c.clear_search.setHidden(c.search.stringValue().is_empty());
        c.table_scroll.setHidden(selected.is_some());
        c.detail_scroll.setHidden(selected.is_none());
        for heading in &c.headings {
            heading.setHidden(selected.is_some());
        }
        c.empty
            .setHidden(selected.is_some() || !self.ivars().filtered.borrow().is_empty());
        if let Some(index) = selected {
            let component = &inventory().components[index];
            c.name.setStringValue(&NSString::from_str(&component.name));
            c.name.sizeToFit();
            let name_width = c.name.frame().size.width;
            c.name.setFrame(rect(0.0, 33.0, name_width, 22.0));
            c.version
                .setStringValue(&NSString::from_str(&component.version));
            c.version
                .setFrame(rect(name_width + 4.0, 33.0, 180.0, 22.0));
            c.source.setTitle(&NSString::from_str(&format!(
                "查看来源 · {}",
                component.license
            )));
            c.error.setStringValue(&NSString::from_str(""));
            for extra in c.extra_text.borrow_mut().drain(..) {
                extra.removeFromSuperview();
            }
            c.detail.ivars().blocks.borrow_mut().clear();
            let width = self.bounds().size.width - 42.0;
            let licensed = !component.notices.is_empty();
            let fallback = notice_text(component);
            let texts: Vec<&str> = if licensed {
                component
                    .notices
                    .iter()
                    .map(|id| inventory().texts[id].as_str())
                    .collect()
            } else {
                vec![fallback.as_str()]
            };
            let mut top = if licensed { 94.0 } else { 98.0 };
            for (i, text) in texts.into_iter().enumerate() {
                let field = if i == 0 {
                    c.text.clone()
                } else {
                    let text =
                        NSTextView::initWithFrame(self.mtm().alloc(), rect(0.0, 0.0, width, 1.0));
                    text.setRichText(false);
                    text.setEditable(false);
                    text.setSelectable(true);
                    text.setTextColor(Some(&native_theme::color(self.ivars().palette.muted)));
                    text.setHorizontallyResizable(false);
                    text.setVerticallyResizable(false);
                    c.detail.addSubview(&text);
                    c.extra_text.borrow_mut().push(text.clone());
                    text
                };
                field.setDrawsBackground(false);
                field.setTextContainerInset(if licensed {
                    NSSize::new(16.0, 12.0)
                } else {
                    NSSize::new(0.0, 0.0)
                });
                let font = if licensed {
                    NSFont::monospacedSystemFontOfSize_weight(10.5, unsafe { NSFontWeightRegular })
                } else {
                    NSFont::systemFontOfSize(10.5)
                };
                field.setString(&NSString::from_str(text));
                // Replacing the plain text resets the view's typing attributes.
                // Apply the original mono font and line spacing to the new text.
                field.setFont(Some(&font));
                let paragraph = NSMutableParagraphStyle::new();
                let line_height = if licensed { 17.0625 } else { 15.225 };
                paragraph.setMinimumLineHeight(line_height);
                paragraph.setMaximumLineHeight(line_height);
                if let Some(storage) = unsafe { field.textStorage() } {
                    unsafe {
                        storage.addAttribute_value_range(
                            NSParagraphStyleAttributeName,
                            &paragraph,
                            objc2_foundation::NSRange::new(0, field.string().len_utf16()),
                        );
                    }
                }
                field.setFrameSize(NSSize::new(width, 1.0));
                let mut height = 20.0_f64;
                if let Some(container) = unsafe { field.textContainer() } {
                    container.setLineFragmentPadding(0.0);
                    container.setContainerSize(NSSize::new(
                        width - if licensed { 32.0 } else { 0.0 },
                        1_000_000.0,
                    ));
                    container.setWidthTracksTextView(true);
                    if let Some(layout) = unsafe { field.layoutManager() } {
                        layout.ensureLayoutForTextContainer(&container);
                        height = height.max(
                            layout
                                .usedRectForTextContainer(&container)
                                .size
                                .height
                                .ceil()
                                + if licensed { 32.0 } else { 0.0 },
                        );
                    }
                }
                let frame = rect(0.0, top, width, height);
                field.setFrame(frame);
                if licensed {
                    c.detail.ivars().blocks.borrow_mut().push(frame);
                }
                top += height + 12.0;
            }
            c.detail.setFrameSize(NSSize::new(
                width,
                (top + 20.0).max(c.detail_scroll.bounds().size.height),
            ));
            c.detail.setNeedsDisplay(true);
            c.detail_scroll
                .contentView()
                .scrollToPoint(NSPoint::new(0.0, 0.0));
        } else {
            self.layout_columns(c);
            c.table.reloadData();
        }
        self.setNeedsDisplay(true);
    }

    fn layout_columns(&self, c: &Contents) {
        // HTML's automatic table layout distributes width from each column's
        // longest content. Measure the current result set with the same font.
        let measure = NSTextField::labelWithString(&NSString::from_str(""), self.mtm());
        measure.setFont(Some(&NSFont::systemFontOfSize(10.5)));
        let mut natural = [32.0_f64; 3];
        for &index in self.ivars().filtered.borrow().iter() {
            let component = &inventory().components[index];
            for (i, value) in [&component.name, &component.version, &component.license]
                .into_iter()
                .enumerate()
            {
                measure.setStringValue(&NSString::from_str(value));
                natural[i] = natural[i].max(measure.intrinsicContentSize().width + 28.0);
            }
        }
        let sum: f64 = natural.iter().sum();
        let available = self.bounds().size.width - 52.0;
        let mut x = 22.0;
        for (i, width) in natural.into_iter().enumerate() {
            let column = c.table.tableColumns().objectAtIndex(i);
            column.setWidth(available * width / sum);
            c.headings[i].setFrame(rect(x + 16.0, 155.0, column.width() - 24.0, 20.0));
            x += column.width();
        }
    }
}

struct BackdropIvars {
    view: Retained<NoticesView>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoNoticesBackdrop"]
    #[thread_kind = MainThreadOnly]
    #[ivars = BackdropIvars]
    struct Backdrop;
    unsafe impl NSObjectProtocol for Backdrop {}
    impl Backdrop {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(drawRect:))]
        fn draw(&self, _: NSRect) {
            NSColor::colorWithSRGBRed_green_blue_alpha(0.0, 0.0, 0.0, 0.35).setFill();
            NSBezierPath::bezierPathWithRect(self.bounds()).fill();
            let p = self.ivars().view.ivars().palette;
            for (blur, offset, opacity) in [(40.0, 16.0, 0.18), (8.0, 2.0, 0.10)] {
                NSGraphicsContext::saveGraphicsState_class();
                let shadow = NSShadow::new();
                shadow.setShadowBlurRadius(blur); shadow.setShadowOffset(NSSize::new(0.0, -offset));
                shadow.setShadowColor(Some(&NSColor::colorWithSRGBRed_green_blue_alpha(0.13, 0.19, 0.24, opacity)));
                shadow.set();
                fill(self.ivars().view.frame(), 12.0, p.surface);
                NSGraphicsContext::restoreGraphicsState_class();
            }
        }
        // The backdrop consumes clicks just as HTML's modal dialog does.
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _: &objc2_app_kit::NSEvent) {}
        #[unsafe(method(parentChanged:))]
        fn parent_changed(&self, notification: &NSNotification) {
            let _keep_alive: Retained<Self> = self.into();
            if &*notification.name() == unsafe { objc2_app_kit::NSWindowWillCloseNotification } {
                close();
            } else if &*notification.name() == unsafe { objc2_app_kit::NSWindowDidResizeNotification } {
                self.resize_to_parent();
            } else if let Some(window) = self.window() {
                window.makeKeyWindow();
            }
        }
    }
);

impl Backdrop {
    fn resize_to_parent(&self) {
        let Some(window) = self.window() else {
            return;
        };
        let Some(parent) = window.parentWindow() else {
            return;
        };
        let frame = parent.contentRectForFrameRect(parent.frame());
        window.setFrame_display(frame, true);
        self.setFrameSize(frame.size);
        let width = (frame.size.width * 0.9).min(860.0);
        let height = (frame.size.height * 0.85).min(760.0);
        self.ivars().view.reflow(rect(
            (frame.size.width - width) * 0.5,
            (frame.size.height - height) * 0.5,
            width,
            height,
        ));
        self.setNeedsDisplay(true);
    }
}

struct Modal {
    window: Retained<NSWindow>,
    view: Retained<NoticesView>,
    backdrop: Retained<Backdrop>,
}
thread_local! { static MODAL: RefCell<Option<Modal>> = const { RefCell::new(None) }; }

pub(super) fn show(parent: &NSWindow, general: &GeneralSettings) {
    let existing = MODAL.with_borrow(|slot| slot.as_ref().map(|modal| modal.window.clone()));
    if let Some(existing) = existing {
        existing.makeKeyAndOrderFront(None);
        return;
    }
    let mtm = parent.mtm();
    let parent_frame = parent.contentRectForFrameRect(parent.frame());
    let size = parent_frame.size;
    let frame = rect(
        size.width * 0.05,
        size.height * 0.075,
        (size.width * 0.9).min(860.0),
        (size.height * 0.85).min(760.0),
    );
    let window = NoticesPanel::alloc(mtm).set_ivars(());
    let window: Retained<NoticesPanel> = unsafe {
        msg_send![super(window), initWithContentRect: parent_frame, styleMask: NSWindowStyleMask::Borderless,
            backing: NSBackingStoreType::Buffered, defer: false]
    };
    let window: Retained<NSWindow> = Retained::into_super(Retained::into_super(window));
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setHasShadow(false);
    window.setHidesOnDeactivate(false);
    window.setTitle(&NSString::from_str("第三方组件与许可"));
    let dark = native_theme::is_dark(&general.theme);
    window.setAppearance(
        NSAppearance::appearanceNamed(unsafe {
            if dark {
                NSAppearanceNameDarkAqua
            } else {
                NSAppearanceNameAqua
            }
        })
        .as_deref(),
    );
    let view = NoticesView::new(
        mtm,
        frame,
        native_theme::palette(&general.interface_theme, dark),
    );
    let backdrop = Backdrop::alloc(mtm).set_ivars(BackdropIvars { view: view.clone() });
    let backdrop: Retained<Backdrop> = unsafe {
        msg_send![super(backdrop), initWithFrame: rect(0.0, 0.0, size.width, size.height)]
    };
    backdrop.addSubview(&view);
    window.setContentView(Some(&backdrop));
    window.setDelegate(Some(ProtocolObject::from_ref(&*view)));
    unsafe {
        parent.addChildWindow_ordered(&window, NSWindowOrderingMode::Above);
        for name in [
            objc2_app_kit::NSWindowDidResizeNotification,
            objc2_app_kit::NSWindowDidBecomeKeyNotification,
            objc2_app_kit::NSWindowWillCloseNotification,
        ] {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                &backdrop,
                sel!(parentChanged:),
                Some(name),
                Some(parent),
            );
        }
    }
    backdrop.resize_to_parent();
    window.makeKeyAndOrderFront(None);
    let search = view
        .ivars()
        .contents
        .borrow()
        .as_ref()
        .unwrap()
        .search
        .clone();
    window.makeFirstResponder(Some(&search));
    MODAL.with_borrow_mut(|slot| {
        *slot = Some(Modal {
            window,
            view,
            backdrop,
        })
    });
}

pub(super) fn close() {
    let modal = MODAL.with_borrow_mut(Option::take);
    if let Some(modal) = modal {
        unsafe {
            NSNotificationCenter::defaultCenter().removeObserver(&modal.backdrop);
        }
        if let Some(parent) = modal.window.parentWindow() {
            parent.removeChildWindow(&modal.window);
            if modal.window.isKeyWindow() && parent.isVisible() {
                parent.makeKeyWindow();
            }
        }
        modal.window.orderOut(None);
    }
}

pub(super) fn close_for(parent: &NSWindow) {
    let owns_modal = MODAL.with_borrow(|slot| {
        slot.as_ref()
            .and_then(|modal| modal.window.parentWindow())
            .is_some_and(|owner| &*owner == parent)
    });
    if owns_modal {
        close();
    }
}

#[cfg(test)]
fn artifact_variant(artifact: &std::path::Path, variant: &str) -> std::path::PathBuf {
    artifact.with_file_name(format!(
        "{}-{variant}.png",
        artifact.file_stem().unwrap().to_string_lossy()
    ))
}

#[cfg(test)]
pub(crate) fn test_workflow(
    parent: &NSWindow,
    general: &GeneralSettings,
    artifact: &std::path::Path,
) {
    show(parent, general);
    let view = MODAL.with_borrow(|slot| slot.as_ref().unwrap().view.clone());
    assert_eq!(
        view.ivars().filtered.borrow().len(),
        inventory().components.len()
    );
    std::fs::write(
        artifact_variant(artifact, "all"),
        super::super::native_view_snapshot::snapshot(&view),
    )
    .unwrap();
    let search = view
        .ivars()
        .contents
        .borrow()
        .as_ref()
        .unwrap()
        .search
        .clone();
    search.setStringValue(&NSString::from_str("objc2"));
    let notification = unsafe {
        NSNotification::notificationWithName_object(
            objc2_app_kit::NSControlTextDidChangeNotification,
            Some(&search),
        )
    };
    let delegate = search.delegate().unwrap();
    unsafe {
        let _: () = msg_send![&delegate, controlTextDidChange: &*notification];
    }
    if let Some(editor) = search.currentEditor() {
        editor.setSelectedRange(objc2_foundation::NSRange::new(5, 0));
    }
    assert!(view.ivars().filtered.borrow().len() > 1);
    assert!(view
        .ivars()
        .filtered
        .borrow()
        .iter()
        .all(|&i| inventory().components[i].name.contains("objc2")));
    std::fs::write(
        artifact_variant(artifact, "list"),
        super::super::native_view_snapshot::snapshot(&view),
    )
    .unwrap();
    let index = *view.ivars().filtered.borrow().first().unwrap();
    let column = view
        .ivars()
        .contents
        .borrow()
        .as_ref()
        .unwrap()
        .table
        .tableColumns()
        .objectAtIndex(0);
    let cell = view.table_cell(Some(&column), 0).unwrap();
    let component_button = cell
        .subviews()
        .objectAtIndex(0)
        .downcast::<NSButton>()
        .unwrap();
    unsafe {
        component_button.performClick(None);
    }
    assert_eq!(view.ivars().selected.get(), Some(index));
    let text = view
        .ivars()
        .contents
        .borrow()
        .as_ref()
        .unwrap()
        .text
        .string()
        .to_string();
    assert_eq!(text, notice_text(&inventory().components[index]));
    std::fs::write(
        artifact,
        super::super::native_view_snapshot::snapshot(&view),
    )
    .unwrap();
    let overlay = MODAL.with_borrow(|slot| slot.as_ref().unwrap().backdrop.clone());
    let overlay_png = super::super::native_view_snapshot::snapshot(&overlay);
    let overlay_image = image::load_from_memory(&overlay_png).unwrap().into_rgba8();
    assert!(
        (overlay_image.get_pixel(0, 0)[3] as i16 - 89).abs() <= 1,
        "Modal backdrop must preserve the original35% black opacity"
    );
    std::fs::write(artifact_variant(artifact, "overlay"), overlay_png).unwrap();
    let back = view
        .ivars()
        .contents
        .borrow()
        .as_ref()
        .unwrap()
        .back
        .clone();
    unsafe {
        back.performClick(None);
    }
    assert!(view.ivars().selected.get().is_none());
    let multi = inventory()
        .components
        .iter()
        .position(|component| component.notices.len() > 1)
        .unwrap();
    view.select(multi);
    {
        let contents = view.ivars().contents.borrow();
        let c = contents.as_ref().unwrap();
        let component = &inventory().components[multi];
        assert_eq!(
            c.text.string().to_string(),
            inventory().texts[&component.notices[0]]
        );
        assert_eq!(c.extra_text.borrow().len() + 1, component.notices.len());
        assert!(
            c.text.font().unwrap().isFixedPitch(),
            "License text must retain the original monospace font"
        );
        assert_eq!(
            c.detail.ivars().blocks.borrow().len(),
            component.notices.len()
        );
        for (field, id) in c
            .extra_text
            .borrow()
            .iter()
            .zip(component.notices.iter().skip(1))
        {
            assert_eq!(field.string().to_string(), inventory().texts[id]);
        }
    }
    std::fs::write(
        artifact_variant(artifact, "licenses"),
        super::super::native_view_snapshot::snapshot(&view),
    )
    .unwrap();
    let clear = view
        .ivars()
        .contents
        .borrow()
        .as_ref()
        .unwrap()
        .clear_search
        .clone();
    unsafe {
        clear.performClick(None);
    }
    assert!(view.ivars().selected.get().is_none());
    assert_eq!(
        view.ivars().filtered.borrow().len(),
        inventory().components.len()
    );
    view.search("definitely-no-bundled-component-7e0fb");
    assert!(view.ivars().filtered.borrow().is_empty());
    assert!(!view
        .ivars()
        .contents
        .borrow()
        .as_ref()
        .unwrap()
        .empty
        .isHidden());
    let window = MODAL.with_borrow(|slot| slot.as_ref().unwrap().window.clone());
    let event = objc2_app_kit::NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        objc2_app_kit::NSEventType::KeyDown, NSPoint::new(0.0, 0.0), NSEventModifierFlags::empty(),
        0.0, window.windowNumber(), None, &NSString::from_str("\u{1b}"), &NSString::from_str("\u{1b}"), false, 53,
    ).unwrap();
    window.sendEvent(&event);
    assert!(MODAL.with_borrow(|slot| slot.is_none()));
    show(parent, general);
    let original_size = parent.contentView().unwrap().bounds().size;
    let resized = NSSize::new(original_size.width + 80.0, original_size.height + 40.0);
    parent.setContentSize(resized);
    let modal_window = MODAL.with_borrow(|slot| slot.as_ref().unwrap().window.clone());
    let modal_size = modal_window.contentView().unwrap().bounds().size;
    assert_eq!(modal_size, resized);
    let dialog_frame = MODAL.with_borrow(|slot| slot.as_ref().unwrap().view.frame());
    assert!((dialog_frame.origin.x * 2.0 + dialog_frame.size.width - resized.width).abs() < 0.01);
    assert!((dialog_frame.origin.y * 2.0 + dialog_frame.size.height - resized.height).abs() < 0.01);
    let other = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            parent.mtm().alloc(),
            rect(0.0, 0.0, 200.0, 200.0),
            NSWindowStyleMask::Titled,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe {
        other.setReleasedWhenClosed(false);
    }
    close_for(&other);
    assert!(MODAL.with_borrow(|slot| slot.is_some()));
    close_for(parent);
    assert!(MODAL.with_borrow(|slot| slot.is_none()));
    assert!(!parent
        .childWindows()
        .is_some_and(|windows| windows.iter().any(|window| &*window == &*modal_window)));
    parent.setContentSize(original_size);
    other.close();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_components_keep_all_notice_texts_and_license_search() {
        let data = inventory();
        assert!(!data.components.is_empty());
        for component in &data.components {
            for id in &component.notices {
                assert!(!data.texts[id].is_empty());
            }
            let text = notice_text(component);
            assert!(!text.is_empty());
            for id in &component.notices {
                assert!(text.contains(&data.texts[id]));
            }
        }
        let matches = filtered("aPaChE");
        assert!(!matches.is_empty());
        assert!(matches.into_iter().all(|i| format!(
            "{} {}",
            data.components[i].name, data.components[i].license
        )
        .to_lowercase()
        .contains("apache")));
    }
}
