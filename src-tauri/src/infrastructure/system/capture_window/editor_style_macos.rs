#![cfg_attr(test, allow(dead_code))]

use std::cell::{Cell, OnceCell, RefCell};

use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSBezelStyle, NSButton, NSButtonType, NSColor,
    NSColorPanel, NSColorSpace, NSColorWell, NSEvent, NSFont, NSPopUpButton, NSSlider, NSTextField,
    NSView,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::application::capture::{
    CaptureEditorStyleChange as Change, CaptureEditorStylePresentation,
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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StyleEvent<'a> {
    session_id: &'a str,
    action: &'static str,
    #[serde(flatten)]
    change: Change,
    commit: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PresetEvent<'a> {
    session_id: &'a str,
    action: &'static str,
    colors: Vec<[u8; 4]>,
}

struct StyleIvars {
    app: AppHandle,
    session_id: String,
    state: RefCell<CaptureEditorStylePresentation>,
    busy: Cell<bool>,
    preset_index: Cell<Option<usize>>,
    preset_pending: Cell<bool>,
    preset_buttons: OnceCell<Vec<Retained<NSButton>>>,
}
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
        let _ = self.ivars().app.emit_to(
            super::backend::CAPTURE_WINDOW_LABEL,
            "capture-editor-toolbar-action",
            StyleEvent {
                session_id: &self.ivars().session_id,
                action: "style",
                change,
                commit,
            },
        );
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
        let _ = self.ivars().app.emit_to(
            super::backend::CAPTURE_WINDOW_LABEL,
            "capture-editor-toolbar-action",
            PresetEvent {
                session_id: &self.ivars().session_id,
                action: "presets",
                colors,
            },
        );
    }
}

pub(crate) struct StyleControls {
    view: Retained<StyleView>,
    color: Retained<ColorWell>,
    presets: Retained<NSPopUpButton>,
    palette_buttons: Vec<Retained<NSButton>>,
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
        app: &AppHandle,
        id: &str,
        parent: &NSView,
        state: &CaptureEditorStylePresentation,
    ) -> Self {
        let mtm = parent.mtm();
        let view = StyleView::alloc(mtm).set_ivars(StyleIvars {
            app: app.clone(),
            session_id: id.into(),
            state: RefCell::new(state.clone()),
            busy: Cell::new(false),
            preset_index: Cell::new(None),
            preset_pending: Cell::new(false),
            preset_buttons: OnceCell::new(),
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
                button.setButtonType(NSButtonType::Switch);
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
        let presets = popup(1);
        presets.setToolTip(Some(&NSString::from_str("颜色预设")));
        let palette_buttons = vec![
            button(5, "+", "保存当前颜色为预设", false),
            button(6, "↺", "替换选中的颜色预设", false),
            button(7, "−", "删除选中的颜色预设", false),
        ];
        let _ = view.ivars().preset_buttons.set(palette_buttons.clone());
        let font = popup(0);
        font.setToolTip(Some(&NSString::from_str("字体")));
        let font_buttons = vec![
            button(1, "粗", "粗体", true),
            button(2, "斜", "斜体", true),
            button(3, "−", "减小字号", false),
            button(4, "+", "增大字号", false),
        ];
        Self {
            color,
            presets,
            palette_buttons,
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
        self.color.close_panel();
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
    pub(crate) fn test_color_active(&self) -> bool {
        let panel = NSColorPanel::sharedColorPanel(self.view.mtm());
        self.color.isActive() && panel.isVisible() && panel.showsAlpha()
    }

    pub(crate) fn update(&self, state: &CaptureEditorStylePresentation, busy: bool, scale: f64) {
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
        }
        for button in self
            .palette_buttons
            .iter()
            .chain(&self.font_buttons)
            .chain(std::iter::once(&self.filled))
        {
            button.setFont(Some(&NSFont::systemFontOfSize(11.0 * scale)));
        }
        self.presets
            .setFont(Some(&NSFont::systemFontOfSize(11.0 * scale)));
        self.font
            .setFont(Some(&NSFont::systemFontOfSize(11.0 * scale)));
        *self.view.ivars().state.borrow_mut() = state.clone();
        self.layout(state, scale);
    }

    fn layout(&self, state: &CaptureEditorStylePresentation, scale: f64) {
        let width = self.view.bounds().size.width / scale;
        let wrapped = width < 560.0;
        let place = |view: &NSView, x: f64, y: f64, width: f64| {
            view.setFrame(rect(
                x * scale,
                (y + 4.0) * scale,
                width * scale,
                28.0 * scale,
            ));
        };
        place(&self.color, 0.0, 0.0, 28.0);
        place(
            &self.presets,
            36.0,
            0.0,
            (width - 130.0).min(144.0).max(60.0),
        );
        let palette_x = 36.0 + (width - 130.0).min(144.0).max(60.0) + 4.0;
        for (index, button) in self.palette_buttons.iter().enumerate() {
            place(button, palette_x + index as f64 * 28.0, 0.0, 24.0);
        }
        let x = if wrapped { 0.0 } else { 276.0 };
        let y = if wrapped { 36.0 } else { 0.0 };
        place(&self.size_title, x, y, 40.0);
        let slider_width = (width - x - 144.0).min(120.0).max(32.0);
        place(&self.size, x + 44.0, y, slider_width);
        place(&self.size_value, x + 48.0 + slider_width, y, 28.0);
        place(&self.filled, x + 80.0 + slider_width, y, 60.0);
        let mut y = if wrapped { 72.0 } else { 36.0 };
        if state.text_mode {
            let family_width = (width - 176.0).min(240.0).max(60.0);
            place(&self.font, 0.0, y, family_width);
            place(&self.font_buttons[0], family_width + 4.0, y, 40.0);
            place(&self.font_buttons[1], family_width + 48.0, y, 40.0);
            place(&self.font_buttons[2], family_width + 92.0, y, 24.0);
            place(&self.font_value, family_width + 120.0, y, 28.0);
            place(&self.font_buttons[3], family_width + 152.0, y, 24.0);
            y += 36.0;
        }
        if state.round_enabled {
            place(&self.corner_title, 0.0, y, 40.0);
            place(&self.corner, 44.0, y, (width - 100.0).min(240.0).max(32.0));
            place(
                &self.corner_value,
                48.0 + (width - 100.0).min(240.0).max(32.0),
                y,
                36.0,
            );
            y += 36.0;
        }
        if state.error.is_some() {
            place(&self.error, 0.0, y, width);
        }
    }
}
