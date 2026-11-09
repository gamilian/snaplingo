use std::cell::{Cell, Ref, RefCell};
use std::collections::BTreeMap;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2::{
    define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSAlert, NSBezierPath, NSButton, NSEvent, NSEventMask, NSEventModifierFlags, NSEventType,
    NSFont, NSFontWeightSemibold, NSLineCapStyle, NSLineJoinStyle, NSTextField, NSTrackingArea,
    NSTrackingAreaOptions, NSView, NSWindowDidResignKeyNotification,
};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use super::super::native_theme::{self, Palette};
use crate::application::hotkeys::{HotkeyRecording, HotkeyRuntime};
use crate::domain::hotkey_config::{hotkey_category, HotkeySettingsSnapshot, HOTKEY_UNSET};
use crate::domain::GeneralSettings;
use crate::infrastructure::desktop_events::{DesktopEvent, DesktopEvents, DesktopSubscription};
use crate::infrastructure::{runtime, ui_thread};

#[derive(Clone)]
pub(crate) struct HotkeyViewServices {
    pub runtime: Arc<HotkeyRuntime>,
    pub recording: HotkeyRecording,
    pub events: Arc<DesktopEvents>,
    pub window_label: String,
}

const TRANSLATION: &[(&str, &str)] = &[
    ("selection-translate", "划词翻译"),
    ("screenshot-translate", "截图翻译"),
    ("show-window", "显示翻译窗口"),
];
const OCR: &[(&str, &str)] = &[
    ("screenshot-ocr", "截图 OCR"),
    ("silent-screenshot-ocr", "静默截图 OCR"),
    ("file-ocr", "上传图片 OCR"),
];
static NEXT_VIEW: AtomicU64 = AtomicU64::new(1);
thread_local! { static VIEWS: RefCell<BTreeMap<u64, Weak<HotkeyView>>> = const { RefCell::new(BTreeMap::new()) }; }

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

#[derive(Clone, Copy)]
enum ButtonKind {
    Record,
    Reset,
    Clear,
    RestoreCategory,
    Conflicts,
}
struct ButtonIvars {
    kind: ButtonKind,
    palette: Cell<Palette>,
    recording: Cell<bool>,
    unset: Cell<bool>,
    hovered: Cell<bool>,
}
define_class!(
    #[unsafe(super(NSButton))]
    #[name = "SnapLingoHotkeyButton"]
    #[thread_kind = MainThreadOnly]
    #[ivars = ButtonIvars]
    struct HotkeyButton;
    unsafe impl NSObjectProtocol for HotkeyButton {}
    impl HotkeyButton {
        #[unsafe(method(isFlipped))] fn flipped(&self) -> bool { true }
        #[unsafe(method(mouseEntered:))]
        fn entered(&self, _: &NSEvent) { self.ivars().hovered.set(true); NSView::setNeedsDisplay(self, true); }
        #[unsafe(method(mouseExited:))]
        fn exited(&self, _: &NSEvent) { self.ivars().hovered.set(false); NSView::setNeedsDisplay(self, true); }
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> *mut NSView {
            let hit: *mut NSView = unsafe { msg_send![super(self), hitTest: point] };
            if hit.is_null() { hit } else { (self as *const Self).cast_mut().cast() }
        }
        #[unsafe(method(drawRect:))]
        fn draw(&self, _: NSRect) {
            let p = self.ivars().palette.get(); let bounds = self.bounds();
            let hovered = self.ivars().hovered.get() && self.isEnabled();
            match self.ivars().kind {
                ButtonKind::Record | ButtonKind::Conflicts => {
                    fill(bounds, 10.0, p.surface);
                    let recording = self.ivars().recording.get();
                    let unset = self.ivars().unset.get();
                    native_theme::color(if recording { p.accent } else if unset || hovered { p.line_strong } else { p.line }).setStroke();
                    let border = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect(0.5,0.5,bounds.size.width-1.0,bounds.size.height-1.0),10.0,10.0);
                    border.setLineWidth(if recording || unset {2.0} else {1.0});
                    if unset && !recording { unsafe { border.setLineDash_count_phase([6.0,6.0].as_ptr(),2,0.0); } }
                    border.stroke();
                }
                ButtonKind::Reset | ButtonKind::Clear => {
                    if hovered { fill(bounds, 5.0, p.accent_soft); }
                    native_theme::color(if hovered { if matches!(self.ivars().kind,ButtonKind::Clear) { p.danger } else { p.accent } } else {p.faint}).setStroke();
                    let path = NSBezierPath::bezierPath(); path.setLineWidth(4.0/3.0);
                    path.setLineCapStyle(NSLineCapStyle::Round);
                    path.setLineJoinStyle(NSLineJoinStyle::Round);
                    if matches!(self.ivars().kind, ButtonKind::Clear) {
                        path.moveToPoint(NSPoint::new(10.0,10.0)); path.lineToPoint(NSPoint::new(18.0,18.0));
                        path.moveToPoint(NSPoint::new(18.0,10.0)); path.lineToPoint(NSPoint::new(10.0,18.0));
                    } else {
                        path.moveToPoint(NSPoint::new(19.33,13.33)); path.curveToPoint_controlPoint1_controlPoint2(NSPoint::new(9.05,12.0),NSPoint::new(18.69,8.17),NSPoint::new(11.79,6.75));
                        path.moveToPoint(NSPoint::new(8.67,8.67)); path.lineToPoint(NSPoint::new(8.67,12.0)); path.lineToPoint(NSPoint::new(12.0,12.0));
                        path.moveToPoint(NSPoint::new(8.67,14.67)); path.curveToPoint_controlPoint1_controlPoint2(NSPoint::new(18.95,16.0),NSPoint::new(9.31,19.83),NSPoint::new(16.21,21.25));
                        path.moveToPoint(NSPoint::new(19.33,19.33)); path.lineToPoint(NSPoint::new(19.33,16.0)); path.lineToPoint(NSPoint::new(16.0,16.0));
                    }
                    path.stroke();
                }
                ButtonKind::RestoreCategory => { if hovered { fill(bounds,10.0,p.surface); } }
            }
        }
    }
);

struct Row {
    label: Retained<NSTextField>,
    record: Retained<HotkeyButton>,
    modifiers: Vec<Retained<NSTextField>>,
    key: Retained<NSTextField>,
    reset: Retained<HotkeyButton>,
    clear: Retained<HotkeyButton>,
}
struct Contents {
    rows: Vec<Row>,
    toolbar: Retained<NSView>,
    restore: Retained<HotkeyButton>,
    conflicts: Retained<HotkeyButton>,
}
struct Recording {
    row: usize,
    id: String,
    pending: Option<(u16, String)>,
}
pub(crate) struct HotkeyIvars {
    id: u64,
    category: &'static str,
    actions: &'static [(&'static str, &'static str)],
    services: RefCell<Option<HotkeyViewServices>>,
    general: RefCell<GeneralSettings>,
    contents: RefCell<Option<Contents>>,
    recording: RefCell<Option<Recording>>,
    saving: Cell<bool>,
    disposed: Cell<bool>,
    error: RefCell<Option<String>>,
    monitor: RefCell<Option<Retained<AnyObject>>>,
    subscription: RefCell<Option<DesktopSubscription>>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoFeatureHotkeysView"]
    #[thread_kind = MainThreadOnly]
    #[ivars = HotkeyIvars]
    pub(crate) struct HotkeyView;
    unsafe impl NSObjectProtocol for HotkeyView {}
    impl HotkeyView {
        #[unsafe(method(isFlipped))] fn flipped(&self) -> bool { true }
        #[unsafe(method(setFrameSize:))]
        fn resized(&self, size: NSSize) { unsafe { let _: () = msg_send![super(self), setFrameSize: size]; } self.layout(); }
        #[unsafe(method(drawRect:))]
        fn draw(&self, _: NSRect) {
            let p = self.palette(); native_theme::color(p.line).setFill();
            let inset = self.content_inset();
            for index in 1..self.ivars().actions.len() { NSBezierPath::bezierPathWithRect(rect(inset,index as f64 * 59.0-1.0,self.bounds().size.width-inset*2.0,1.0)).fill(); }
        }
        #[unsafe(method(hotkeyAction:))]
        fn action(&self, sender: &NSButton) { self.perform(sender.tag()); }
        #[unsafe(method(recordingWindowLostFocus:))]
        fn lost_focus(&self, event: &NSNotification) {
            if event.object().is_some_and(|object| self.window().is_some_and(|window| std::ptr::eq(Retained::as_ptr(&window).cast::<AnyObject>(),Retained::as_ptr(&object)))) { self.cancel_recording(); }
        }
    }
);

fn text(parent: &NSView, value: &str, size: f64, color: [u8; 3]) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&NSString::from_str(value), parent.mtm());
    field.setFont(Some(&NSFont::systemFontOfSize(size)));
    field.setTextColor(Some(&native_theme::color(color)));
    field.setSelectable(false);
    parent.addSubview(&field);
    field
}
fn button(
    parent: &NSView,
    owner: &HotkeyView,
    kind: ButtonKind,
    tag: isize,
    title: &str,
) -> Retained<HotkeyButton> {
    let b = HotkeyButton::alloc(parent.mtm()).set_ivars(ButtonIvars {
        kind,
        palette: Cell::new(owner.palette()),
        recording: Cell::new(false),
        unset: Cell::new(false),
        hovered: Cell::new(false),
    });
    let b: Retained<HotkeyButton> = unsafe { msg_send![super(b), initWithFrame: NSRect::ZERO] };
    b.setBordered(false);
    b.setTitle(&NSString::new());
    b.setTag(tag);
    b.setToolTip(Some(&NSString::from_str(title)));
    let tracking = unsafe {
        NSTrackingArea::initWithRect_options_owner_userInfo(
            NSTrackingArea::alloc(),
            NSRect::ZERO,
            NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::ActiveInKeyWindow
                | NSTrackingAreaOptions::InVisibleRect,
            Some(&b),
            None,
        )
    };
    b.addTrackingArea(&tracking);
    unsafe {
        b.setTarget(Some(owner));
        b.setAction(Some(sel!(hotkeyAction:)));
    }
    if matches!(kind, ButtonKind::RestoreCategory | ButtonKind::Conflicts) {
        let label = text(&b, title, 10.5, owner.palette().muted);
        label.setFrame(rect(10.0, 7.0, 70.0, 18.0));
    }
    parent.addSubview(&b);
    b
}

impl HotkeyView {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        category: &'static str,
        services: HotkeyViewServices,
        general: GeneralSettings,
    ) -> Result<Retained<Self>, String> {
        let actions = match category {
            "ocr" => OCR,
            "translation" => TRANSLATION,
            _ => return Err("Unsupported native hotkey category".into()),
        };
        services.runtime.snapshot().map_err(|e| e.to_string())?;
        let id = NEXT_VIEW.fetch_add(1, Ordering::Relaxed);
        let view = Self::alloc(mtm).set_ivars(HotkeyIvars {
            id,
            category,
            actions,
            services: RefCell::new(Some(services)),
            general: RefCell::new(general),
            contents: RefCell::new(None),
            recording: RefCell::new(None),
            saving: Cell::new(false),
            disposed: Cell::new(false),
            error: RefCell::new(None),
            monitor: RefCell::new(None),
            subscription: RefCell::new(None),
        });
        let view: Retained<Self> =
            unsafe { msg_send![super(view), initWithFrame: rect(0.0,0.0,650.0,182.0)] };
        VIEWS.with_borrow_mut(|views| {
            views.insert(id, Weak::from_retained(&view));
        });
        view.build();
        let weak = Weak::from_retained(&view);
        let monitor = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
            if weak
                .load()
                .is_some_and(|view| view.receive_event(unsafe { event.as_ref() }))
            {
                std::ptr::null_mut()
            } else {
                event.as_ptr()
            }
        });
        *view.ivars().monitor.borrow_mut() = unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(
                NSEventMask::KeyDown
                    | NSEventMask::KeyUp
                    | NSEventMask::LeftMouseDown
                    | NSEventMask::RightMouseDown,
                &monitor,
            )
        };
        let subscription = view.services().events.subscribe(move |event| {
            if matches!(
                event,
                DesktopEvent::HotkeysChanged | DesktopEvent::HotkeyRecorded { .. }
            ) {
                let event = event.clone();
                ui_thread::dispatch(move || {
                    if let Some(view) = lookup(id) {
                        match event {
                            DesktopEvent::HotkeyRecorded {
                                window_label,
                                recording_id,
                                hotkey,
                            } => {
                                if window_label == view.services().window_label
                                    && view.window().is_some_and(|window| {
                                        window.isKeyWindow() && window.isVisible()
                                    })
                                    && view.services().recording.session().is_some_and(|session| {
                                        session.id == recording_id
                                            && session.window_label == window_label
                                    })
                                    && view
                                        .ivars()
                                        .recording
                                        .borrow()
                                        .as_ref()
                                        .is_some_and(|session| session.id == recording_id)
                                {
                                    view.finish_recording(hotkey);
                                }
                            }
                            _ => view.sync(),
                        }
                    }
                });
            }
        });
        *view.ivars().subscription.borrow_mut() = Some(subscription);
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                &view,
                sel!(recordingWindowLostFocus:),
                Some(NSWindowDidResignKeyNotification),
                None,
            );
        }
        view.sync();
        Ok(view)
    }

    fn palette(&self) -> Palette {
        let general = self.ivars().general.borrow();
        native_theme::palette(
            &general.interface_theme,
            native_theme::is_dark(&general.theme),
        )
    }
    fn services(&self) -> Ref<'_, HotkeyViewServices> {
        Ref::map(self.ivars().services.borrow(), |services| {
            services.as_ref().unwrap()
        })
    }
    fn build(&self) {
        let toolbar = NSView::initWithFrame(self.mtm().alloc(), rect(0.0, 0.0, 200.0, 32.0));
        let restore = button(
            &toolbar,
            self,
            ButtonKind::RestoreCategory,
            1000,
            "恢复默认值",
        );
        let conflicts = button(&toolbar, self, ButtonKind::Conflicts, 1001, "检测冲突");
        let mut rows = Vec::new();
        let p = self.palette();
        for (index, (_, title)) in self.ivars().actions.iter().enumerate() {
            let label = text(self, title, 13.0, p.ink);
            label.setFont(Some(&NSFont::systemFontOfSize_weight(13.0, unsafe {
                NSFontWeightSemibold
            })));
            let record = button(
                self,
                self,
                ButtonKind::Record,
                index as isize * 3,
                "点击修改快捷键",
            );
            let modifiers = ["⇧", "⌥", "⌘", "⌃"]
                .map(|m| text(&record, m, 13.0, p.faint))
                .into_iter()
                .collect();
            let key = text(&record, "", 13.0, p.accent);
            key.setFont(Some(&NSFont::systemFontOfSize_weight(13.0, unsafe {
                NSFontWeightSemibold
            })));
            let reset = button(
                self,
                self,
                ButtonKind::Reset,
                index as isize * 3 + 1,
                "恢复默认值",
            );
            let clear = button(
                self,
                self,
                ButtonKind::Clear,
                index as isize * 3 + 2,
                "清除快捷键",
            );
            rows.push(Row {
                label,
                record,
                modifiers,
                key,
                reset,
                clear,
            });
        }
        *self.ivars().contents.borrow_mut() = Some(Contents {
            rows,
            toolbar,
            restore,
            conflicts,
        });
        self.layout();
    }
    pub(crate) fn toolbar_view(&self) -> &NSView {
        // The toolbar has the same lifetime as the owner, and never changes.
        let pointer = Retained::as_ptr(&self.ivars().contents.borrow().as_ref().unwrap().toolbar);
        unsafe { &*pointer }
    }
    pub(crate) fn preferred_height(&self) -> f64 {
        self.ivars().actions.len() as f64 * 58.0 + (self.ivars().actions.len() - 1) as f64 + 8.0
    }
    pub(crate) fn refresh(&self, general: GeneralSettings) {
        *self.ivars().general.borrow_mut() = general;
        self.sync();
    }
    fn sync(&self) {
        if self.ivars().disposed.get() {
            return;
        }
        let snapshot = match self.services().runtime.snapshot() {
            Ok(value) => value,
            Err(error) => {
                *self.ivars().error.borrow_mut() = Some(error.to_string());
                return;
            }
        };
        let defaults = self.services().runtime.default_snapshot();
        let saved = hotkey_category(&snapshot, self.ivars().category).unwrap();
        let defaults = hotkey_category(&defaults, self.ivars().category).unwrap();
        let p = self.palette();
        let contents = self.ivars().contents.borrow();
        let Some(c) = contents.as_ref() else {
            return;
        };
        for (index, row) in c.rows.iter().enumerate() {
            let action = self.ivars().actions[index].0;
            let value = saved
                .get(action)
                .map(String::as_str)
                .unwrap_or(HOTKEY_UNSET);
            let recording = self
                .ivars()
                .recording
                .borrow()
                .as_ref()
                .is_some_and(|session| session.row == index);
            row.label.setTextColor(Some(&native_theme::color(p.ink)));
            row.record.ivars().recording.set(recording);
            row.record.ivars().unset.set(value == HOTKEY_UNSET);
            for button in [&row.record, &row.reset, &row.clear] {
                button.ivars().palette.set(p);
                NSView::setNeedsDisplay(button, true);
            }
            for (label, glyph) in row.modifiers.iter().zip(['⇧', '⌥', '⌘', '⌃']) {
                label.setTextColor(Some(&native_theme::color(
                    if !recording && value.contains(glyph) {
                        p.ink
                    } else {
                        p.faint
                    },
                )));
            }
            row.key.setStringValue(&NSString::from_str(if recording {
                "按下快捷键..."
            } else if value == HOTKEY_UNSET {
                HOTKEY_UNSET
            } else {
                value.trim_start_matches(['⇧', '⌥', '⌘', '⌃', '⊞'])
            }));
            row.key.setFont(Some(&NSFont::systemFontOfSize_weight(
                if recording || value == HOTKEY_UNSET {
                    10.5
                } else {
                    13.0
                },
                if recording || value == HOTKEY_UNSET {
                    0.0
                } else {
                    unsafe { NSFontWeightSemibold }
                },
            )));
            row.key.setTextColor(Some(&native_theme::color(
                if recording || value == HOTKEY_UNSET {
                    p.faint
                } else {
                    p.accent
                },
            )));
            row.record.setEnabled(!self.ivars().saving.get());
            row.reset.setEnabled(
                !self.ivars().saving.get()
                    && defaults.get(action).is_some_and(|default| default != value),
            );
            row.clear
                .setEnabled(!self.ivars().saving.get() && value != HOTKEY_UNSET);
        }
        for button in [&c.restore, &c.conflicts] {
            button.ivars().palette.set(p);
            NSView::setNeedsDisplay(button, true);
            button.setEnabled(!self.ivars().saving.get());
            for child in button.subviews() {
                if let Some(label) = child.downcast_ref::<NSTextField>() {
                    label.setTextColor(Some(&native_theme::color(p.muted)));
                }
            }
        }
        drop(contents);
        self.layout();
        self.setNeedsDisplay(true);
    }
    fn layout(&self) {
        let contents = self.ivars().contents.borrow();
        let Some(c) = contents.as_ref() else {
            return;
        };
        let width = self.bounds().size.width;
        let inset = self.content_inset();
        c.restore.setFrame(rect(0.0, 0.0, 80.0, 32.0));
        c.conflicts.setFrame(rect(88.0, 0.0, 68.0, 32.0));
        for (index, row) in c.rows.iter().enumerate() {
            let y = index as f64 * 59.0;
            let recording = row.record.ivars().recording.get();
            let key_width = row.key.intrinsicContentSize().width;
            let modifier_widths: Vec<_> = row
                .modifiers
                .iter()
                .map(|label| label.intrinsicContentSize().width)
                .collect();
            let content_width = modifier_widths.iter().sum::<f64>() + 16.0 + key_width;
            let record_width = (24.0 + content_width).max(156.0);
            let x = width - inset - 28.0 * 2.0 - 4.0 - 12.0 - record_width;
            row.label
                .setFrame(rect(inset, y + 19.0, (x - inset - 16.0).max(0.0), 22.0));
            row.record.setFrame(rect(x, y + 11.0, record_width, 36.0));
            let mut glyph_x = (record_width - content_width) * 0.5;
            for (label, glyph_width) in row.modifiers.iter().zip(modifier_widths) {
                label.setFrame(rect(glyph_x, 8.0, glyph_width, 20.0));
                glyph_x += glyph_width + 4.0;
            }
            row.key.setFrame(rect(
                glyph_x,
                if recording { 9.0 } else { 8.0 },
                key_width,
                22.0,
            ));
            row.reset
                .setFrame(rect(x + record_width + 12.0, y + 15.0, 28.0, 28.0));
            row.clear
                .setFrame(rect(x + record_width + 44.0, y + 15.0, 28.0, 28.0));
        }
    }
    fn content_inset(&self) -> f64 {
        if self.ivars().general.borrow().interface_theme == "paper" {
            0.0
        } else {
            23.0
        }
    }
    fn perform(&self, tag: isize) {
        if self.ivars().disposed.get() || self.ivars().saving.get() {
            return;
        }
        if tag == 1000 {
            self.cancel_recording();
            let alert = NSAlert::new(self.mtm());
            alert.setMessageText(&NSString::from_str("确定要恢复这一组快捷键到默认值吗？"));
            alert.addButtonWithTitle(&NSString::from_str("确认"));
            alert.addButtonWithTitle(&NSString::from_str("取消"));
            if let Some(window) = self.window() {
                let id = self.ivars().id;
                let completed = RcBlock::new(move |response| {
                    if response == 1000 {
                        if let Some(view) = lookup(id) {
                            view.mutate(Mutation::ResetCategory);
                        }
                    }
                });
                alert.beginSheetModalForWindow_completionHandler(&window, Some(&completed));
            }
            return;
        }
        if tag == 1001 {
            self.cancel_recording();
            let snapshot = self.services().runtime.snapshot();
            match snapshot {
                Ok(snapshot) => self.message(&conflict_message(&snapshot, self.ivars().category)),
                Err(error) => self.message(&format!("检测冲突失败：{error}")),
            };
            return;
        }
        let row = (tag / 3) as usize;
        if row >= self.ivars().actions.len() {
            return;
        }
        match tag % 3 {
            0 => self.begin_recording(row),
            1 => {
                self.cancel_recording();
                self.mutate(Mutation::Reset(row));
            }
            _ => {
                self.cancel_recording();
                self.mutate(Mutation::Save(row, HOTKEY_UNSET.into()));
            }
        }
    }
    fn begin_recording(&self, row: usize) {
        let same = self
            .ivars()
            .recording
            .borrow()
            .as_ref()
            .is_some_and(|session| session.row == row);
        self.cancel_recording();
        if same {
            return;
        }
        if !self
            .window()
            .is_some_and(|window| window.isKeyWindow() && window.isVisible())
        {
            self.message("开始录制快捷键失败：需要当前设置窗口获得焦点");
            return;
        }
        let id = format!(
            "native-hotkey-{}-{}",
            self.ivars().id,
            NEXT_VIEW.fetch_add(1, Ordering::Relaxed)
        );
        self.services()
            .recording
            .begin(&self.services().window_label, id.clone());
        *self.ivars().recording.borrow_mut() = Some(Recording {
            row,
            id,
            pending: None,
        });
        self.sync();
    }
    pub(crate) fn cancel_recording(&self) {
        if let Some(session) = self.ivars().recording.borrow_mut().take() {
            self.services()
                .recording
                .end(&self.services().window_label, &session.id);
        }
        if !self.ivars().disposed.get() {
            self.sync();
        }
    }
    fn finish_recording(&self, hotkey: String) {
        if self.ivars().saving.get() {
            return;
        }
        let Some(row) = self
            .ivars()
            .recording
            .borrow()
            .as_ref()
            .map(|session| session.row)
        else {
            return;
        };
        self.mutate(Mutation::Save(row, hotkey));
    }
    fn receive_event(&self, event: &NSEvent) -> bool {
        if self.ivars().disposed.get() || self.ivars().recording.borrow().is_none() {
            return false;
        }
        let session_id = self.ivars().recording.borrow().as_ref().unwrap().id.clone();
        if !self.services().recording.session().is_some_and(|session| {
            session.id == session_id && session.window_label == self.services().window_label
        }) {
            self.cancel_recording();
            return false;
        }
        if matches!(
            event.r#type(),
            NSEventType::LeftMouseDown | NSEventType::RightMouseDown
        ) {
            let inside = self.ivars().contents.borrow().as_ref().is_some_and(|c| {
                let row = self.ivars().recording.borrow().as_ref().unwrap().row;
                let button = &c.rows[row].record;
                let point = button.convertPoint_fromView(event.locationInWindow(), None);
                let b = button.bounds();
                self.window()
                    .is_some_and(|window| event.windowNumber() == window.windowNumber())
                    && point.x >= 0.0
                    && point.y >= 0.0
                    && point.x < b.size.width
                    && point.y < b.size.height
            });
            if !inside {
                self.cancel_recording();
            }
            return false;
        }
        if !self.window().is_some_and(|window| {
            window.isKeyWindow() && window.windowNumber() == event.windowNumber()
        }) {
            self.cancel_recording();
            return false;
        }
        if event.r#type() == NSEventType::KeyDown {
            if event.keyCode() == 53 {
                self.cancel_recording();
                return true;
            }
            if !event.isARepeat() {
                if let Some(hotkey) = local_chord(event.keyCode(), event.modifierFlags()) {
                    self.ivars()
                        .recording
                        .borrow_mut()
                        .as_mut()
                        .unwrap()
                        .pending = Some((event.keyCode(), hotkey));
                }
            }
            return true;
        }
        if event.r#type() == NSEventType::KeyUp {
            let hotkey = self
                .ivars()
                .recording
                .borrow_mut()
                .as_mut()
                .and_then(|session| {
                    if session
                        .pending
                        .as_ref()
                        .is_some_and(|(key, _)| *key == event.keyCode())
                    {
                        session.pending.take().map(|(_, hotkey)| hotkey)
                    } else {
                        None
                    }
                });
            if let Some(hotkey) = hotkey {
                self.finish_recording(hotkey);
            }
            return true;
        }
        false
    }
    fn mutate(&self, mutation: Mutation) {
        if self.ivars().saving.replace(true) {
            return;
        }
        self.ivars().error.borrow_mut().take();
        self.sync();
        let runtime = self.services().runtime.clone();
        let category = self.ivars().category;
        let id = self.ivars().id;
        let action = match &mutation {
            Mutation::Save(row, _) | Mutation::Reset(row) => {
                Some(self.ivars().actions[*row].0.to_string())
            }
            _ => None,
        };
        runtime::spawn_blocking(move || {
            let result = match mutation {
                Mutation::Save(_, ref hotkey) => runtime
                    .update_hotkey(category.into(), action.unwrap(), hotkey.clone())
                    .map(|_| ())
                    .map_err(|error| format!("快捷键 {hotkey} 注册失败：{error}")),
                Mutation::Reset(_) => runtime
                    .reset_hotkey(category.into(), action.unwrap())
                    .map(|_| ())
                    .map_err(|error| format!("恢复快捷键失败：{error}")),
                Mutation::ResetCategory => runtime
                    .reset_category(category.into())
                    .map(|_| ())
                    .map_err(|error| format!("恢复快捷键组失败：{error}")),
            };
            ui_thread::dispatch(move || {
                if let Some(view) = lookup(id) {
                    view.ivars().saving.set(false);
                    view.cancel_recording();
                    view.sync();
                    if let Err(error) = result {
                        *view.ivars().error.borrow_mut() = Some(error.clone());
                        view.message(&error);
                    }
                }
            });
        });
    }
    fn message(&self, message: &str) {
        let alert = NSAlert::new(self.mtm());
        alert.setMessageText(&NSString::from_str(message));
        alert.addButtonWithTitle(&NSString::from_str("确认"));
        if let Some(window) = self.window() {
            // Worker completions arrive on the main queue. A blocking modal
            // loop here prevents that queue from delivering other UI work.
            alert.beginSheetModalForWindow_completionHandler(&window, None);
        }
    }
    pub(crate) fn dispose(&self) {
        if self.ivars().disposed.replace(true) {
            return;
        }
        self.cancel_recording();
        self.ivars().services.borrow_mut().take();
        self.ivars().subscription.borrow_mut().take();
        if let Some(monitor) = self.ivars().monitor.borrow_mut().take() {
            unsafe {
                NSEvent::removeMonitor(&monitor);
            }
        }
        unsafe {
            NSNotificationCenter::defaultCenter().removeObserver(self);
        }
        VIEWS.with_borrow_mut(|views| {
            views.remove(&self.ivars().id);
        });
    }
    #[cfg(test)]
    pub(crate) fn test_record(&self, row: usize) {
        self.perform(row as isize * 3);
    }
    #[cfg(test)]
    pub(crate) fn test_clear(&self, row: usize) {
        self.perform(row as isize * 3 + 2);
    }
    #[cfg(test)]
    pub(crate) fn test_reset(&self, row: usize) {
        self.perform(row as isize * 3 + 1);
    }
    #[cfg(test)]
    pub(crate) fn test_recording_id(&self) -> Option<String> {
        self.ivars()
            .recording
            .borrow()
            .as_ref()
            .map(|s| s.id.clone())
    }
    #[cfg(test)]
    pub(crate) fn test_busy(&self) -> bool {
        self.ivars().saving.get()
    }
    #[cfg(test)]
    pub(crate) fn test_error(&self) -> Option<String> {
        self.ivars().error.borrow().clone()
    }
    #[cfg(test)]
    pub(crate) fn test_buttons(&self, row: usize) -> (bool, bool) {
        let contents = self.ivars().contents.borrow();
        let row = &contents.as_ref().unwrap().rows[row];
        (row.reset.isEnabled(), row.clear.isEnabled())
    }
    #[cfg(test)]
    pub(crate) fn test_key(&self, event: &NSEvent) -> bool {
        self.receive_event(event)
    }
    #[cfg(test)]
    pub(crate) fn test_geometry(&self, root: &NSView) -> Vec<serde_json::Value> {
        let frame = |view: &NSView| {
            let r = root.convertRect_fromView(view.bounds(), Some(view));
            serde_json::json!({"x":r.origin.x,"y":r.origin.y,"width":r.size.width,"height":r.size.height})
        };
        self.ivars()
            .contents
            .borrow()
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "controls":[frame(&row.record),frame(&row.reset),frame(&row.clear)],
                    "label":frame(&row.label),
                "text":row.label.stringValue().to_string(),
                })
            })
            .collect()
    }
}

impl Drop for HotkeyView {
    fn drop(&mut self) {
        self.dispose();
    }
}
enum Mutation {
    Save(usize, String),
    Reset(usize),
    ResetCategory,
}
fn lookup(id: u64) -> Option<Retained<HotkeyView>> {
    VIEWS.with_borrow(|views| views.get(&id).and_then(Weak::load))
}

fn conflict_message(snapshot: &HotkeySettingsSnapshot, category: &str) -> String {
    let actions = if category == "ocr" { OCR } else { TRANSLATION };
    let saved = hotkey_category(snapshot, category).unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut duplicates = Vec::new();
    for (action, _) in actions {
        if let Some(hotkey) = saved
            .get(*action)
            .filter(|value| value.as_str() != HOTKEY_UNSET && !value.is_empty())
        {
            if !seen.insert(hotkey) && !duplicates.contains(&hotkey) {
                duplicates.push(hotkey);
            }
        }
    }
    if duplicates.is_empty() {
        "未发现冲突的快捷键".into()
    } else {
        format!(
            "发现冲突的快捷键：{}",
            duplicates
                .into_iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn local_chord(code: u16, flags: NSEventModifierFlags) -> Option<String> {
    let key = match code {
        0 => "A",
        1 => "S",
        2 => "D",
        3 => "F",
        4 => "H",
        5 => "G",
        6 => "Z",
        7 => "X",
        8 => "C",
        9 => "V",
        11 => "B",
        12 => "Q",
        13 => "W",
        14 => "E",
        15 => "R",
        16 => "Y",
        17 => "T",
        18 => "1",
        19 => "2",
        20 => "3",
        21 => "4",
        22 => "6",
        23 => "5",
        25 => "9",
        26 => "7",
        28 => "8",
        29 => "0",
        31 => "O",
        32 => "U",
        34 => "I",
        35 => "P",
        37 => "L",
        38 => "J",
        40 => "K",
        45 => "N",
        46 => "M",
        122 => "F1",
        120 => "F2",
        99 => "F3",
        118 => "F4",
        96 => "F5",
        97 => "F6",
        98 => "F7",
        100 => "F8",
        101 => "F9",
        109 => "F10",
        103 => "F11",
        111 => "F12",
        105 => "F13",
        107 => "F14",
        113 => "F15",
        106 => "F16",
        64 => "F17",
        79 => "F18",
        80 => "F19",
        90 => "F20",
        _ => return None,
    };
    let mut hotkey = String::new();
    for (flag, glyph) in [
        (NSEventModifierFlags::Shift, '⇧'),
        (NSEventModifierFlags::Option, '⌥'),
        (NSEventModifierFlags::Command, '⌘'),
        (NSEventModifierFlags::Control, '⌃'),
    ] {
        if flags.contains(flag) {
            hotkey.push(glyph);
        }
    }
    hotkey.push_str(key);
    Some(hotkey)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_recording_uses_existing_modifier_order_and_supported_keys() {
        assert_eq!(
            local_chord(2, NSEventModifierFlags::Option),
            Some("⌥D".into())
        );
        assert_eq!(
            local_chord(
                1,
                NSEventModifierFlags::Shift
                    | NSEventModifierFlags::Option
                    | NSEventModifierFlags::Command
                    | NSEventModifierFlags::Control
            ),
            Some("⇧⌥⌘⌃S".into())
        );
        assert_eq!(
            local_chord(122, NSEventModifierFlags::empty()),
            Some("F1".into())
        );
        assert_eq!(local_chord(53, NSEventModifierFlags::empty()), None);
        assert_eq!(local_chord(49, NSEventModifierFlags::empty()), None);
    }
    #[test]
    fn conflict_detection_matches_each_feature_without_counting_unset_keys() {
        let mut snapshot = crate::domain::hotkey_config::default_hotkey_snapshot();
        assert_eq!(conflict_message(&snapshot, "ocr"), "未发现冲突的快捷键");
        snapshot
            .translation
            .insert("screenshot-translate".into(), "⌥D".into());
        snapshot
            .translation
            .insert("show-window".into(), "⌥D".into());
        assert_eq!(
            conflict_message(&snapshot, "translation"),
            "发现冲突的快捷键：⌥D"
        );
    }
}
