#![cfg_attr(test, allow(dead_code))]

use std::cell::{Cell, OnceCell, RefCell};
use std::io::Cursor;

use super::toolbar::Dispatch;
use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadOnly, Message};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSBackingStoreType, NSBezelStyle, NSBezierPath,
    NSButton, NSButtonType, NSColor, NSColorPanel, NSColorSpace, NSColorWell, NSEvent, NSFont,
    NSImage, NSPanel, NSPopUpButton, NSScreen, NSSlider, NSTextField, NSView, NSWindowOrderingMode,
    NSWindowSharingType, NSWindowStyleMask,
};
use objc2_foundation::{NSData, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use snaplingo_core::application::capture::{
    CaptureEditorControlAction, CaptureEditorStyleChange as Change, CaptureEditorStylePresentation,
};

struct ColorPanelLease {
    level: isize,
    appearance: Option<Retained<NSAppearance>>,
}
struct ColorWellIvars {
    lease: RefCell<Option<ColorPanelLease>>,
}
define_class!(
    #[unsafe(super(NSColorWell))]
    #[name = "SnapLingoAnnotationColorWell"]
    #[thread_kind = MainThreadOnly]
    #[ivars = ColorWellIvars]
    struct ColorWell;
    unsafe impl NSObjectProtocol for ColorWell {}
    impl ColorWell {
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            self.prepare_panel();
            unsafe { let _: () = msg_send![super(self), mouseDown: event]; }
        }
    }
);
impl ColorWell {
    fn prepare_panel(&self) {
        let panel = NSColorPanel::sharedColorPanel(self.mtm());
        self.ivars()
            .lease
            .borrow_mut()
            .get_or_insert_with(|| ColorPanelLease {
                level: panel.level(),
                appearance: panel.appearance(),
            });
        panel.setShowsAlpha(true);
        if let Some(window) = self.window() {
            panel.setLevel(window.level() + 1);
            panel.setAppearance(window.appearance().as_deref());
        }
    }
    fn close_panel(&self) {
        let Some(lease) = self.ivars().lease.borrow_mut().take() else {
            return;
        };
        let panel = NSColorPanel::sharedColorPanel(self.mtm());
        let owned = self.isActive();
        self.deactivate();
        // A different active color well owns the shared picker after exclusive activation.
        if owned || !panel.isVisible() {
            panel.orderOut(None);
            panel.setLevel(lease.level);
            panel.setAppearance(lease.appearance.as_deref());
        }
    }
}

define_class!(
    #[unsafe(super(NSSlider))]
    #[name = "SnapLingoAnnotationSizeSlider"]
    #[thread_kind = MainThreadOnly]
    struct SizeSlider;
    unsafe impl NSObjectProtocol for SizeSlider {}
    impl SizeSlider {
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let previous = self.doubleValue();
            unsafe { let _: () = msg_send![super(self), mouseDown: event]; }
            self.commit_change(previous);
        }
        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            let previous = self.doubleValue();
            unsafe { let _: () = msg_send![super(self), keyDown: event]; }
            self.commit_change(previous);
        }
    }
);
impl SizeSlider {
    fn commit_change(&self, previous: f64) {
        if self.doubleValue() != previous {
            if let Some(target) = self.target() {
                unsafe {
                    let _: () = msg_send![&target, commitSize: self];
                }
            }
        }
    }
}

struct StyleIvars {
    dispatch: Dispatch,
    state: RefCell<CaptureEditorStylePresentation>,
    busy: Cell<bool>,
    preset_index: Cell<Option<usize>>,
    preset_pending: Cell<bool>,
    preset_buttons: OnceCell<Vec<Retained<NSButton>>>,
    palette_open: Cell<bool>,
    main_height: Cell<f64>,
    scale: Cell<f64>,
    palette: OnceCell<Retained<NSPanel>>,
    palette_views: OnceCell<Vec<Retained<NSView>>>,
    swatches: RefCell<Vec<Retained<NSButton>>>,
    swatch_colors: RefCell<Vec<[u8; 4]>>,
    color_well: OnceCell<Retained<ColorWell>>,
    colors: Cell<[[u8; 3]; 6]>,
}
struct PaletteIvars {
    selected: Cell<Option<usize>>,
    scale: Cell<f64>,
    colors: Cell<[[u8; 3]; 6]>,
}
define_class!(
    #[unsafe(super(NSPanel))]
    #[name = "SnapLingoAnnotationPalettePanel"]
    #[thread_kind = MainThreadOnly]
    struct PalettePanel;
    unsafe impl NSObjectProtocol for PalettePanel {}
    impl PalettePanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool { false }
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool { false }
    }
);
define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoAnnotationPaletteView"]
    #[thread_kind = MainThreadOnly]
    #[ivars = PaletteIvars]
    struct PaletteView;
    unsafe impl NSObjectProtocol for PaletteView {}
    impl PaletteView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let bounds = self.bounds();
            let path = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect(0.5, 0.5, bounds.size.width - 1.0, bounds.size.height - 1.0), 6.0, 6.0);
            let colors = self.ivars().colors.get();
            themed_color(colors[0]).setFill();
            path.fill();
            themed_color(colors[1]).setStroke(); path.setLineWidth(0.5); path.stroke();
            if let Some(index) = self.ivars().selected.get() {
                let scale = self.ivars().scale.get();
                let path = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect((10.0+(index%7) as f64*32.0)*scale,(28.0+(index/7) as f64*30.0)*scale,28.0*scale,28.0*scale),4.0*scale,4.0*scale);
                themed_color(colors[3]).setStroke(); path.setLineWidth(2.0*scale); path.stroke();
            }
        }
    }
);
define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoAnnotationStyleView"]
    #[thread_kind = MainThreadOnly]
    #[ivars = StyleIvars]
    struct StyleView;
    unsafe impl NSObjectProtocol for StyleView {}
    impl StyleView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool { true }
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> *mut NSView {
            let hit: *mut NSView = unsafe { msg_send![super(self), hitTest: point] };
            if std::ptr::eq(hit, self as *const Self as *mut NSView) { std::ptr::null_mut() } else { hit }
        }
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let state = self.ivars().state.borrow();
            let scale = self.ivars().scale.get();
            let colors = self.ivars().colors.get();
            if state.text_mode {
                for (tag, active) in [(1, state.style.bold), (2, state.style.italic)] {
                    let button = self.subviews().iter().find_map(|view| view.downcast_ref::<NSButton>().filter(|button| button.tag() == tag).map(|button| button.retain()));
                    if let Some(button) = button {
                        let path = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(button.frame(), 4.0 * scale, 4.0 * scale);
                        themed_color(if active { colors[4] } else { colors[0] }).setFill();
                        path.fill(); themed_color(if active { colors[3] } else { colors[1] }).setStroke(); path.setLineWidth(0.5); path.stroke();
                    }
                }
            }
        }
        #[unsafe(method(togglePalette:))]
        fn toggle_palette(&self, _sender: &NSButton) {
            self.ivars().palette_open.set(!self.ivars().palette_open.get());
            self.relayout();
        }
        #[unsafe(method(selectPreset:))]
        fn select_preset(&self, sender: &NSButton) {
            if self.ivars().busy.get() { return; }
            let index = sender.tag() as usize;
            let color = self.ivars().state.borrow().color_presets.get(index).copied();
            if let Some(color) = color {
                self.ivars().preset_index.set(Some(index));
                self.send(Change::Color(color), false);
            }
        }
        #[unsafe(method(openSystemPalette:))]
        fn open_system_palette(&self, _sender: &NSButton) {
            if self.ivars().busy.get() { return; }
            let color = self.ivars().color_well.get().unwrap();
            color.prepare_panel(); color.activate(true);
            NSColorPanel::sharedColorPanel(self.mtm()).makeKeyAndOrderFront(None);
        }
        #[unsafe(method(closePalette:))]
        fn close_palette(&self, _sender: &NSButton) {
            self.ivars().palette_open.set(false);
            self.relayout();
        }
        #[unsafe(method(changeColor:))]
        fn change_color(&self, sender: &NSColorWell) {
            let Some(color) = sender.color().colorUsingColorSpace(&NSColorSpace::sRGBColorSpace()) else { return; };
            let channels = [color.redComponent(), color.greenComponent(), color.blueComponent(), color.alphaComponent()]
                .map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8);
            self.send(Change::Color(channels), false);
        }
        #[unsafe(method(changeSize:))]
        fn change_size(&self, sender: &NSSlider) {
            self.send_size(sender, false);
        }
        #[unsafe(method(commitSize:))]
        fn commit_size(&self, sender: &NSSlider) {
            self.send_size(sender, true);
        }
        #[unsafe(method(selectValue:))]
        fn select_value(&self, sender: &NSPopUpButton) {
            let index = sender.indexOfSelectedItem();
            if sender.tag() == 0 {
                let font = if index == 0 { String::new() } else { sender.titleOfSelectedItem().map(|title| title.to_string()).unwrap_or_default() };
                self.send(Change::FontFamily(font), false);
            } else if index > 0 {
                let index = index as usize - 1;
                let color = self.ivars().state.borrow().color_presets.get(index).copied();
                if let Some(color) = color {
                    self.ivars().preset_index.set(Some(index));
                    self.ivars().palette_open.set(false);
                    self.send(Change::Color(color), false);
                }
            }
        }
        #[unsafe(method(performStyleAction:))]
        fn perform_action(&self, sender: &NSButton) {
            if !sender.isEnabled() || self.ivars().busy.get() { return; }
            let state = self.ivars().state.borrow();
            let change = match sender.tag() {
                0 => Some(Change::Filled(sender.state() == 1)),
                1 => Some(Change::Bold(sender.state() == 1)),
                2 => Some(Change::Italic(sender.state() == 1)),
                3 => Some(Change::FontSize(state.font_size.saturating_sub(2).clamp(10, 96))),
                4 => Some(Change::FontSize((state.font_size + 2).clamp(10, 96))),
                _ => None,
            };
            drop(state);
            if let Some(change) = change { self.send(change, false); }
            else { self.change_presets(sender.tag()); }
        }
    }
);

impl StyleView {
    fn send_size(&self, sender: &NSSlider, commit: bool) {
        let value = sender.doubleValue().round();
        let change = if sender.tag() == 1 {
            Change::CornerRadius(value)
        } else if self.ivars().state.borrow().text_mode {
            Change::FontSize(value as u32)
        } else if self.ivars().state.borrow().blur_mode {
            Change::BlurRadius(value)
        } else {
            Change::StrokeWidth(value as u32)
        };
        self.send(change, commit);
    }
    fn send(&self, change: Change, commit: bool) {
        if self.ivars().busy.get() {
            return;
        }
        (self.ivars().dispatch)(CaptureEditorControlAction::Style { change, commit });
    }
    fn change_presets(&self, tag: isize) {
        if self.ivars().preset_pending.get() {
            return;
        }
        let state = self.ivars().state.borrow();
        let mut colors = state.color_presets.clone();
        let current = state.style.color;
        match (tag, self.ivars().preset_index.get()) {
            (5, _) if !colors.contains(&current) => colors.push(current),
            (6, Some(index))
                if index < colors.len()
                    && !colors
                        .iter()
                        .enumerate()
                        .any(|(i, value)| i != index && *value == current) =>
            {
                colors[index] = current
            }
            (7, Some(index)) if index < colors.len() => {
                colors.remove(index);
            }
            _ => return,
        }
        if colors == state.color_presets {
            return;
        }
        self.ivars().preset_pending.set(true);
        for button in self.ivars().preset_buttons.get().unwrap() {
            button.setEnabled(false);
        }
        drop(state);
        (self.ivars().dispatch)(CaptureEditorControlAction::Presets(colors));
    }
    fn relayout(&self) {
        self.setNeedsDisplay(true);
        let Some(palette) = self.ivars().palette.get() else {
            return;
        };
        let Some(parent) = self.window() else {
            return;
        };
        if !self.ivars().palette_open.get() || !parent.isVisible() {
            palette.orderOut(None);
            return;
        }
        palette.setAppearance(parent.appearance().as_deref());
        palette.setLevel(parent.level());
        if palette.parentWindow().is_none() {
            unsafe {
                parent.addChildWindow_ordered(palette, NSWindowOrderingMode::Above);
            }
        }
        let scale = self.ivars().scale.get();
        let parent_frame = parent.frame();
        let width = 248.0 * scale;
        let rows = self
            .ivars()
            .state
            .borrow()
            .color_presets
            .len()
            .div_ceil(7)
            .max(1);
        let grid_bottom = 30.0 + rows as f64 * 30.0;
        let height = (grid_bottom + 76.0) * scale;
        let screen = parent
            .screen()
            .map(|screen| screen.visibleFrame())
            .unwrap_or_else(|| {
                NSScreen::mainScreen(self.mtm())
                    .map(|screen| screen.visibleFrame())
                    .unwrap_or(parent_frame)
            });
        let below = parent_frame.origin.y - height - 4.0 * scale;
        let above = parent_frame.origin.y + parent_frame.size.height + 4.0 * scale;
        let y = if below >= screen.origin.y {
            below
        } else {
            above
                .min(screen.origin.y + screen.size.height - height)
                .max(screen.origin.y)
        };
        palette.setFrame_display(
            rect(
                (parent_frame.origin.x + parent_frame.size.width - width).clamp(
                    screen.origin.x,
                    (screen.origin.x + screen.size.width - width).max(screen.origin.x),
                ),
                y,
                width,
                height,
            ),
            false,
        );
        let views = self.ivars().palette_views.get().unwrap();
        if let Some(title) = views[0].downcast_ref::<NSTextField>() {
            title.setFont(Some(&NSFont::boldSystemFontOfSize(11.0 * scale)));
        }
        views[0].setFrame(rect(12.0 * scale, 6.0 * scale, 224.0 * scale, 20.0 * scale));
        views[1].setHidden(true);
        if let Some(content) = palette
            .contentView()
            .and_then(|view| view.downcast::<PaletteView>().ok())
        {
            content
                .ivars()
                .selected
                .set(self.ivars().preset_index.get());
            content.ivars().scale.set(scale);
            content.ivars().colors.set(self.ivars().colors.get());
            content.setNeedsDisplay(true);
        }
        for (index, swatch) in self.ivars().swatches.borrow().iter().enumerate() {
            swatch.setFrame(rect(
                (12.0 + (index % 7) as f64 * 32.0) * scale,
                (30.0 + (index / 7) as f64 * 30.0) * scale,
                24.0 * scale,
                24.0 * scale,
            ));
            swatch.setState(isize::from(self.ivars().preset_index.get() == Some(index)));
        }
        for (index, view) in views[2..5].iter().enumerate() {
            view.setFrame(rect(
                (12.0 + index as f64 * 76.0) * scale,
                (grid_bottom + 42.0) * scale,
                72.0 * scale,
                26.0 * scale,
            ));
        }
        views[5].setFrame(rect(
            12.0 * scale,
            (grid_bottom + 4.0) * scale,
            224.0 * scale,
            30.0 * scale,
        ));
        views[6].setFrame(rect(220.0 * scale, 6.0 * scale, 20.0 * scale, 20.0 * scale));
        for view in views.iter() {
            if let Some(button) = view.downcast_ref::<NSButton>() {
                button.setContentTintColor(Some(&themed_color(self.ivars().colors.get()[2])));
            }
            if let Some(label) = view.downcast_ref::<NSTextField>() {
                label.setTextColor(Some(&themed_color(self.ivars().colors.get()[2])));
            }
        }
        palette.orderFront(None);
    }
}

#[derive(Clone)]
pub(crate) struct StyleControls {
    view: Retained<StyleView>,
    color: Retained<ColorWell>,
    presets: Retained<NSPopUpButton>,
    palette_buttons: Vec<Retained<NSButton>>,
    palette_toggle: Retained<NSButton>,
    size: Retained<SizeSlider>,
    size_title: Retained<NSTextField>,
    size_value: Retained<NSTextField>,
    filled: Retained<NSButton>,
    font: Retained<NSPopUpButton>,
    font_buttons: Vec<Retained<NSButton>>,
    font_value: Retained<NSTextField>,
    corner: Retained<SizeSlider>,
    corner_title: Retained<NSTextField>,
    corner_value: Retained<NSTextField>,
    error: Retained<NSTextField>,
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}
fn themed_color(channels: [u8; 3]) -> Retained<NSColor> {
    let [r, g, b] = channels.map(|value| value as f64 / 255.0);
    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0)
}
const DEFAULT_COLORS: [[u8; 3]; 6] = [
    [255, 255, 255],
    [229, 233, 240],
    [109, 120, 137],
    [57, 117, 220],
    [234, 241, 253],
    [255, 255, 255],
];

fn color_hex(color: [u8; 4]) -> String {
    let [r, g, b, a] = color;
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

impl StyleControls {
    pub(crate) fn new(
        parent: &NSView,
        state: &CaptureEditorStylePresentation,
        dispatch: Dispatch,
    ) -> Self {
        let mtm = parent.mtm();
        let view = StyleView::alloc(mtm).set_ivars(StyleIvars {
            dispatch,
            state: RefCell::new(state.clone()),
            busy: Cell::new(false),
            preset_index: Cell::new(None),
            preset_pending: Cell::new(false),
            preset_buttons: OnceCell::new(),
            palette_open: Cell::new(false),
            main_height: Cell::new(68.0),
            scale: Cell::new(1.0),
            palette: OnceCell::new(),
            palette_views: OnceCell::new(),
            swatches: RefCell::new(Vec::new()),
            swatch_colors: RefCell::new(Vec::new()),
            color_well: OnceCell::new(),
            colors: Cell::new(DEFAULT_COLORS),
        });
        let view: Retained<StyleView> =
            unsafe { msg_send![super(view), initWithFrame: rect(0.0,0.0,1.0,1.0)] };
        parent.addSubview(&view);
        let popup = |tag| {
            let popup = NSPopUpButton::initWithFrame_pullsDown(
                NSPopUpButton::alloc(mtm),
                rect(0.0, 0.0, 1.0, 1.0),
                false,
            );
            popup.setTag(tag);
            unsafe {
                popup.setTarget(Some(&view));
                popup.setAction(Some(sel!(selectValue:)));
            }
            view.addSubview(&popup);
            popup
        };
        let label = |text: &str| {
            let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
            view.addSubview(&label);
            label
        };
        let button = |tag, title: &str, hint: &str, toggle| {
            let button = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str(title),
                    Some(&view),
                    Some(sel!(performStyleAction:)),
                    mtm,
                )
            };
            button.setTag(tag);
            button.setBezelStyle(NSBezelStyle::SmallSquare);
            button.setToolTip(Some(&NSString::from_str(hint)));
            if toggle {
                button.setButtonType(if tag == 0 {
                    NSButtonType::Switch
                } else {
                    NSButtonType::PushOnPushOff
                });
            }
            view.addSubview(&button);
            button
        };
        let slider = |tag| {
            let slider = SizeSlider::alloc(mtm).set_ivars(());
            let slider: Retained<SizeSlider> =
                unsafe { msg_send![super(slider), initWithFrame: rect(0.0,0.0,1.0,1.0)] };
            slider.setTag(tag);
            slider.setContinuous(true);
            unsafe {
                slider.setTarget(Some(&view));
                slider.setAction(Some(sel!(changeSize:)));
            }
            view.addSubview(&slider);
            slider
        };
        let color = ColorWell::alloc(mtm).set_ivars(ColorWellIvars {
            lease: RefCell::new(None),
        });
        let color: Retained<ColorWell> =
            unsafe { msg_send![super(color), initWithFrame: rect(0.0,0.0,1.0,1.0)] };
        color.setToolTip(Some(&NSString::from_str("颜色 · 支持 RGB、HEX 和透明度")));
        unsafe {
            color.setTarget(Some(&view));
            color.setAction(Some(sel!(changeColor:)));
        }
        view.addSubview(&color);
        let _ = view.ivars().color_well.set(color.clone());
        let presets = popup(1);
        presets.setToolTip(Some(&NSString::from_str("颜色预设")));
        let palette_buttons = vec![
            button(5, "新增", "保存当前颜色为预设", false),
            button(6, "修改", "替换选中的颜色预设", false),
            button(7, "删除", "删除选中的颜色预设", false),
        ];
        let _ = view.ivars().preset_buttons.set(palette_buttons.clone());
        let font = popup(0);
        font.setToolTip(Some(&NSString::from_str("字体")));
        let font_buttons = vec![
            button(1, "B", "粗体", true),
            button(2, "I", "斜体", true),
            button(3, "−", "减小字号", false),
            button(4, "+", "增大字号", false),
        ];
        let palette_toggle = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str("⌄"),
                Some(&view),
                Some(sel!(togglePalette:)),
                mtm,
            )
        };
        palette_toggle.setBordered(false);
        palette_toggle.setToolTip(Some(&NSString::from_str("颜色预设")));
        view.addSubview(&palette_toggle);
        let panel = PalettePanel::alloc(mtm).set_ivars(());
        let panel: Retained<PalettePanel> = unsafe {
            msg_send![super(panel), initWithContentRect: rect(0.0, 0.0, 248.0, 92.0), styleMask: NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel, backing: NSBackingStoreType::Buffered, defer: false]
        };
        panel.setOpaque(false);
        panel.setHasShadow(true);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setSharingType(NSWindowSharingType::None);
        panel.setHidesOnDeactivate(false);
        unsafe {
            panel.setReleasedWhenClosed(false);
        }
        let content = PaletteView::alloc(mtm).set_ivars(PaletteIvars {
            selected: Cell::new(None),
            scale: Cell::new(1.0),
            colors: Cell::new(DEFAULT_COLORS),
        });
        let content: Retained<PaletteView> =
            unsafe { msg_send![super(content), initWithFrame: rect(0.0,0.0,248.0,92.0)] };
        let title = NSTextField::labelWithString(&NSString::from_str("颜色"), mtm);
        title.setFont(Some(&NSFont::boldSystemFontOfSize(11.0)));
        content.addSubview(&title);
        content.addSubview(&presets);
        for button in &palette_buttons {
            content.addSubview(button);
        }
        let system = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str("系统调色板"),
                Some(&view),
                Some(sel!(openSystemPalette:)),
                mtm,
            )
        };
        system.setBezelStyle(NSBezelStyle::SmallSquare);
        system.setToolTip(Some(&NSString::from_str(
            "系统调色板 · 支持 RGB、HEX 和透明度",
        )));
        content.addSubview(&system);
        let close = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str("×"),
                Some(&view),
                Some(sel!(closePalette:)),
                mtm,
            )
        };
        close.setBordered(false);
        close.setToolTip(Some(&NSString::from_str("关闭颜色面板")));
        content.addSubview(&close);
        panel.setContentView(Some(&content));
        panel.orderOut(None);
        let views: Vec<Retained<NSView>> = vec![
            title.clone().into_super().into_super(),
            presets.clone().into_super().into_super().into_super(),
            palette_buttons[0].clone().into_super().into_super(),
            palette_buttons[1].clone().into_super().into_super(),
            palette_buttons[2].clone().into_super().into_super(),
            system.into_super().into_super(),
            close.into_super().into_super(),
        ];
        let _ = view.ivars().palette_views.set(views);
        let _ = view.ivars().palette.set(panel.into_super());
        Self {
            color,
            presets,
            palette_buttons,
            palette_toggle,
            size: slider(0),
            size_title: label("线宽"),
            size_value: label("2"),
            filled: button(0, "填充", "填充图形", true),
            font,
            font_buttons,
            font_value: label("24"),
            corner: slider(1),
            corner_title: label("圆角"),
            corner_value: label("0"),
            error: label(""),
            view,
        }
    }
    pub(crate) fn root(&self) -> &NSView {
        &self.view
    }
    pub(crate) fn close_color_panel(&self) {
        self.view.ivars().palette_open.set(false);
        if let Some(palette) = self.view.ivars().palette.get() {
            palette.orderOut(None);
        }
        self.color.close_panel();
    }
    pub(crate) fn close_palette(&self) {
        self.close_color_panel();
        if let Some(palette) = self.view.ivars().palette.get() {
            if let Some(parent) = palette.parentWindow() {
                parent.removeChildWindow(palette);
            }
            palette.setContentView(None);
            palette.close();
        }
    }
    pub(crate) fn dismiss_palette(&self) -> bool {
        if !self.view.ivars().palette_open.get() {
            return false;
        }
        self.close_color_panel();
        true
    }
    pub(crate) fn color_panel_owns_focus(&self) -> bool {
        self.color.isActive() && NSColorPanel::sharedColorPanel(self.view.mtm()).isKeyWindow()
    }

    #[cfg(test)]
    pub(crate) fn test_state(&self) -> CaptureEditorStylePresentation {
        self.view.ivars().state.borrow().clone()
    }
    #[cfg(test)]
    pub(crate) fn test_size(&self, value: f64, commit: bool) {
        self.size.setDoubleValue(value);
        self.view.send_size(&self.size, commit);
    }
    #[cfg(test)]
    pub(crate) fn test_size_value(&self) -> (f64, f64, f64) {
        (
            self.size.doubleValue(),
            self.size.minValue(),
            self.size.maxValue(),
        )
    }
    #[cfg(test)]
    pub(crate) fn test_drag_size(&self) -> u32 {
        use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSEventType};
        let window = self.size.window().unwrap();
        let bounds = self.size.bounds();
        let point = self.size.convertPoint_toView(
            NSPoint::new(bounds.size.width * 0.75, bounds.size.height / 2.0),
            None,
        );
        let event = |kind| {
            NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                kind, point, NSEventModifierFlags::empty(), 0.0, window.windowNumber(), None, 1, 1, 0.0,
            ).unwrap()
        };
        let down = event(NSEventType::LeftMouseDown);
        let up = event(NSEventType::LeftMouseUp);
        let application = NSApplication::sharedApplication(self.view.mtm());
        application.postEvent_atStart(&up, true);
        application.sendEvent(&down);
        self.size.doubleValue().round() as u32
    }
    #[cfg(test)]
    pub(crate) fn test_corner(&self, value: f64) {
        self.corner.setDoubleValue(value);
        self.view.send_size(&self.corner, false);
    }
    #[cfg(test)]
    pub(crate) fn test_color(&self, channels: [u8; 4]) {
        let [r, g, b, a] = channels.map(|value| f64::from(value) / 255.0);
        self.color
            .setColor(&NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a));
        unsafe {
            let _: () = msg_send![&self.view, changeColor: &*self.color];
        }
    }
    #[cfg(test)]
    pub(crate) fn test_preset(&self, index: usize) {
        self.presets.selectItemAtIndex(index as isize + 1);
        unsafe {
            let _: () = msg_send![&self.view, selectValue: &*self.presets];
        }
    }
    #[cfg(test)]
    pub(crate) fn test_font(&self, family: &str) {
        self.font
            .selectItemAtIndex(self.font.indexOfItemWithTitle(&NSString::from_str(family)));
        unsafe {
            let _: () = msg_send![&self.view, selectValue: &*self.font];
        }
    }
    #[cfg(test)]
    pub(crate) fn test_button(&self, tag: isize) {
        let button = match tag {
            0 => &self.filled,
            1..=4 => &self.font_buttons[tag as usize - 1],
            5..=7 => &self.palette_buttons[tag as usize - 5],
            _ => panic!("Unknown style button"),
        };
        unsafe {
            button.performClick(None);
        }
    }
    #[cfg(test)]
    pub(crate) fn test_open_color(&self) {
        self.color.prepare_panel();
        self.color.activate(true);
        NSColorPanel::sharedColorPanel(self.view.mtm()).makeKeyAndOrderFront(None);
    }
    #[cfg(test)]
    pub(crate) fn test_open_palette(&self) {
        unsafe {
            self.palette_toggle.performClick(None);
        }
    }
    #[cfg(test)]
    pub(crate) fn test_palette_visible(&self) -> bool {
        self.view
            .ivars()
            .palette
            .get()
            .is_some_and(|palette| palette.isVisible())
    }
    #[cfg(test)]
    pub(crate) fn test_palette_snapshot(&self) -> Vec<u8> {
        let palette = self.view.ivars().palette.get().unwrap();
        crate::infrastructure::system::native_view_snapshot::snapshot(
            &palette.contentView().unwrap(),
        )
    }
    #[cfg(test)]
    pub(crate) fn test_geometry(&self) -> serde_json::Value {
        let frame = |view: &NSView| {
            let r = view.frame();
            serde_json::json!([r.origin.x, r.origin.y, r.size.width, r.size.height])
        };
        let mut controls = vec![];
        for (name, view) in [
            ("color", &self.color as &NSView),
            ("palette", &self.palette_toggle),
            ("size", &self.size),
            ("filled", &self.filled),
            ("font", &self.font),
            ("round", &self.corner),
        ] {
            if !view.isHidden() {
                controls.push(serde_json::json!({"target":name,"frame":frame(view)}));
            }
        }
        for (name, button) in ["bold", "italic", "font-decrease", "font-increase"]
            .into_iter()
            .zip(&self.font_buttons)
        {
            if !button.isHidden() {
                controls.push(serde_json::json!({"target":name,"frame":frame(button)}));
            }
        }
        let palette = self.view.ivars().palette.get().unwrap();
        let content = palette.contentView().unwrap();
        let swatches = self
            .view
            .ivars()
            .swatches
            .borrow()
            .iter()
            .map(|button| frame(button))
            .collect::<Vec<_>>();
        let buttons=self.palette_buttons.iter().map(|button|serde_json::json!({"label":button.title().to_string(),"frame":frame(button),"enabled":button.isEnabled()})).collect::<Vec<_>>();
        serde_json::json!({"controls":controls,"palette":{"size":[content.bounds().size.width,content.bounds().size.height],"swatches":swatches,"buttons":buttons},"strokeWidth":self.size.doubleValue(),"corner":self.corner.doubleValue(),"presets":self.view.ivars().state.borrow().color_presets,"color":self.view.ivars().state.borrow().style.color})
    }
    #[cfg(test)]
    pub(crate) fn test_color_active(&self) -> bool {
        let panel = NSColorPanel::sharedColorPanel(self.view.mtm());
        self.color.isActive() && panel.isVisible() && panel.showsAlpha()
    }

    pub(crate) fn update(
        &self,
        state: &CaptureEditorStylePresentation,
        busy: bool,
        scale: f64,
        main_height: f64,
        colors: [[u8; 3]; 6],
    ) {
        self.view.ivars().main_height.set(main_height);
        self.view.ivars().scale.set(scale);
        self.view.ivars().colors.set(colors);
        let old = self.view.ivars().state.borrow();
        if self.view.ivars().preset_pending.get() && old.color_presets != state.color_presets {
            if state.color_presets.len() > old.color_presets.len() {
                self.view
                    .ivars()
                    .preset_index
                    .set(Some(state.color_presets.len() - 1));
            } else if state.color_presets.len() < old.color_presets.len() {
                self.view.ivars().preset_index.set(None);
            }
        }
        if old.color_presets != state.color_presets || state.error.is_some() {
            self.view.ivars().preset_pending.set(false);
        }
        if old.color_presets != state.color_presets || self.presets.numberOfItems() == 0 {
            self.presets.removeAllItems();
            self.presets
                .addItemWithTitle(&NSString::from_str("颜色预设"));
            for (index, color) in state.color_presets.iter().enumerate() {
                self.presets.addItemWithTitle(&NSString::from_str(&format!(
                    "{} {}",
                    index + 1,
                    color_hex(*color)
                )));
            }
        }
        if old.font_families != state.font_families
            || old.style.font_family != state.style.font_family
            || self.font.numberOfItems() == 0
        {
            self.font.removeAllItems();
            self.font.addItemWithTitle(&NSString::from_str("默认字体"));
            for family in &state.font_families {
                self.font.addItemWithTitle(&NSString::from_str(family));
            }
            if !state.style.font_family.is_empty()
                && !state.font_families.contains(&state.style.font_family)
            {
                self.font
                    .addItemWithTitle(&NSString::from_str(&state.style.font_family));
            }
        }
        drop(old);
        if self
            .view
            .ivars()
            .preset_index
            .get()
            .is_some_and(|index| index >= state.color_presets.len())
        {
            self.view.ivars().preset_index.set(None);
        }
        self.presets.selectItemAtIndex(
            self.view
                .ivars()
                .preset_index
                .get()
                .map_or(0, |index| index as isize + 1),
        );
        let font_index = if state.style.font_family.is_empty() {
            0
        } else {
            self.font
                .indexOfItemWithTitle(&NSString::from_str(&state.style.font_family))
        };
        self.font.selectItemAtIndex(font_index.max(0));
        let [r, g, b, a] = state.style.color.map(|value| f64::from(value) / 255.0);
        self.color
            .setColor(&NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a));
        let (title, value, min, max) = if state.text_mode {
            (
                "字号",
                f64::from(state.font_size),
                f64::from(state.font_size.min(10)),
                f64::from(state.font_size.max(96)),
            )
        } else if state.blur_mode {
            ("模糊", state.style.blur_radius, 4.0, 64.0)
        } else {
            ("线宽", f64::from(state.style.stroke_width), 1.0, 32.0)
        };
        self.size_title.setStringValue(&NSString::from_str(title));
        self.size_value
            .setStringValue(&NSString::from_str(&format!("{value:.0}")));
        self.size.setToolTip(Some(&NSString::from_str(title)));
        self.size.setMinValue(min);
        self.size.setMaxValue(max);
        self.size.setDoubleValue(value);
        self.corner.setMinValue(0.0);
        self.corner.setMaxValue(100.0);
        self.corner.setDoubleValue(state.style.corner_radius);
        self.corner_value
            .setStringValue(&NSString::from_str(&format!(
                "{:.0}",
                state.style.corner_radius
            )));
        self.filled.setState(if state.style.filled { 1 } else { 0 });
        self.font_buttons[0].setState(if state.style.bold { 1 } else { 0 });
        self.font_buttons[1].setState(if state.style.italic { 1 } else { 0 });
        self.font_value
            .setStringValue(&NSString::from_str(&state.font_size.to_string()));
        self.error
            .setStringValue(&NSString::from_str(state.error.as_deref().unwrap_or("")));
        self.error.setToolTip(
            state
                .error
                .as_ref()
                .map(|error| NSString::from_str(error))
                .as_deref(),
        );
        self.view.ivars().busy.set(busy);
        self.color.setEnabled(!busy);
        self.palette_toggle.setEnabled(!busy);
        self.presets.setEnabled(!busy);
        self.size.setEnabled(!busy);
        self.corner.setEnabled(!busy);
        self.filled.setEnabled(!busy && state.fill_enabled);
        self.font.setEnabled(!busy);
        for button in &self.font_buttons {
            button.setEnabled(!busy);
        }
        let selected = self
            .view
            .ivars()
            .preset_index
            .get()
            .filter(|index| *index < state.color_presets.len());
        let editable = !busy && !self.view.ivars().preset_pending.get();
        self.palette_buttons[0]
            .setEnabled(editable && !state.color_presets.contains(&state.style.color));
        self.palette_buttons[1].setEnabled(
            editable
                && selected.is_some_and(|index| {
                    state.color_presets[index] != state.style.color
                        && !state
                            .color_presets
                            .iter()
                            .enumerate()
                            .any(|(i, color)| i != index && *color == state.style.color)
                }),
        );
        self.palette_buttons[2].setEnabled(editable && selected.is_some());
        self.font.setHidden(!state.text_mode);
        self.font_value.setHidden(!state.text_mode);
        for button in &self.font_buttons {
            button.setHidden(!state.text_mode);
        }
        for view in [
            &self.corner as &NSView,
            &self.corner_title,
            &self.corner_value,
        ] {
            view.setHidden(!state.round_enabled);
        }
        self.error.setHidden(state.error.is_none());
        for field in [
            &self.size_title,
            &self.size_value,
            &self.font_value,
            &self.corner_title,
            &self.corner_value,
            &self.error,
        ] {
            field.setFont(Some(&NSFont::systemFontOfSize(11.0 * scale)));
            field.setTextColor(Some(&themed_color(colors[2])));
        }
        for button in self
            .palette_buttons
            .iter()
            .chain(&self.font_buttons)
            .chain(std::iter::once(&self.filled))
        {
            button.setFont(Some(&NSFont::systemFontOfSize(11.0 * scale)));
            button.setContentTintColor(Some(&themed_color(if button.state() == 1 {
                colors[3]
            } else {
                colors[2]
            })));
        }
        self.presets
            .setFont(Some(&NSFont::systemFontOfSize(11.0 * scale)));
        self.font
            .setFont(Some(&NSFont::systemFontOfSize(11.0 * scale)));
        *self.view.ivars().state.borrow_mut() = state.clone();
        self.update_swatches(state);
        self.layout(state, scale);
        self.view.relayout();
    }

    fn update_swatches(&self, state: &CaptureEditorStylePresentation) {
        let mut swatches = self.view.ivars().swatches.borrow_mut();
        let content = self
            .view
            .ivars()
            .palette
            .get()
            .unwrap()
            .contentView()
            .unwrap();
        while swatches.len() < state.color_presets.len() {
            let button = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::new(),
                    Some(&self.view),
                    Some(sel!(selectPreset:)),
                    self.view.mtm(),
                )
            };
            button.setTag(swatches.len() as isize);
            button.setBordered(false);
            content.addSubview(&button);
            swatches.push(button);
        }
        while swatches.len() > state.color_presets.len() {
            swatches.pop().unwrap().removeFromSuperview();
        }
        let changed = *self.view.ivars().swatch_colors.borrow() != state.color_presets;
        for (button, color) in swatches.iter().zip(&state.color_presets) {
            let color_text = NSString::from_str(&color_hex(*color));
            button.setToolTip(Some(&color_text));
            button.setEnabled(!self.view.ivars().busy.get());
            if changed {
                let mut bytes = Cursor::new(Vec::new());
                let mut pixels = image::RgbaImage::from_pixel(48, 48, image::Rgba([0, 0, 0, 0]));
                for (x, y, pixel) in pixels.enumerate_pixels_mut() {
                    let cx = if x < 8 {
                        8.0
                    } else if x > 39 {
                        39.0
                    } else {
                        x as f64
                    };
                    let cy = if y < 8 {
                        8.0
                    } else if y > 39 {
                        39.0
                    } else {
                        y as f64
                    };
                    if (x as f64 - cx).hypot(y as f64 - cy) > 7.0 {
                        continue;
                    }
                    let background = if (x / 8 + y / 8) % 2 == 0 {
                        245.0
                    } else {
                        210.0
                    };
                    let alpha = color[3] as f64 / 255.0;
                    *pixel = image::Rgba([
                        ((color[0] as f64 * alpha) + background * (1.0 - alpha)).round() as u8,
                        ((color[1] as f64 * alpha) + background * (1.0 - alpha)).round() as u8,
                        ((color[2] as f64 * alpha) + background * (1.0 - alpha)).round() as u8,
                        255,
                    ]);
                }
                if pixels.write_to(&mut bytes, image::ImageFormat::Png).is_ok() {
                    if let Some(image) = NSImage::initWithData(
                        NSImage::alloc(),
                        &NSData::with_bytes(bytes.get_ref()),
                    ) {
                        image.setSize(NSSize::new(24.0, 24.0));
                        button.setImage(Some(&image));
                    }
                }
            }
        }
        if changed {
            *self.view.ivars().swatch_colors.borrow_mut() = state.color_presets.clone();
        }
    }

    fn layout(&self, state: &CaptureEditorStylePresentation, scale: f64) {
        let width = self.view.bounds().size.width / scale;
        let place = |view: &NSView, x: f64, y: f64, width: f64| {
            view.setFrame(rect(x * scale, y * scale, width * scale, 28.0 * scale));
        };
        place(&self.color, width - 40.0, 4.0, 28.0);
        place(&self.palette_toggle, width - 16.0, 4.0, 12.0);
        self.size_title.setHidden(true);
        place(&self.size, 12.0, 36.0, 86.0);
        place(&self.size_value, 110.0, 36.0, 28.0);
        place(&self.filled, 148.0, 36.0, 32.0);
        self.filled.setTitle(&NSString::new());
        let mut y = self.view.ivars().main_height.get() + 12.0;
        if state.text_mode {
            let family_width = (width - 190.0).min(220.0).max(60.0);
            place(&self.font, 12.0, y, family_width);
            place(&self.font_buttons[0], family_width + 24.0, y, 28.0);
            place(&self.font_buttons[1], family_width + 60.0, y, 28.0);
            place(&self.font_buttons[2], family_width + 100.0, y, 28.0);
            place(&self.font_value, family_width + 136.0, y, 28.0);
            place(&self.font_buttons[3], family_width + 172.0, y, 28.0);
            for button in &self.font_buttons {
                button.setBordered(false);
            }
            y += 36.0;
        }
        if state.round_enabled {
            place(&self.corner_title, 12.0, y, 36.0);
            place(&self.corner, 56.0, y, (width - 112.0).min(160.0).max(32.0));
            place(
                &self.corner_value,
                64.0 + (width - 112.0).min(160.0).max(32.0),
                y,
                36.0,
            );
            y += 36.0;
        }
        if state.error.is_some() {
            place(&self.error, 12.0, y, width - 24.0);
        }
    }
}
