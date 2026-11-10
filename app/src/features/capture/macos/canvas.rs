use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::{ProtocolObject, Sel};
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBezierPath, NSBitmapFormat, NSBitmapImageRep, NSColor, NSCompositingOperation,
    NSDeviceRGBColorSpace, NSEvent, NSEventModifierFlags, NSFont, NSFontManager, NSFontTraitMask,
    NSImage, NSLineCapStyle, NSLineJoinStyle, NSTextDelegate, NSTextInputClient, NSTextView,
    NSTextViewDelegate, NSView,
};
use objc2_foundation::{NSNotification, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

use snaplingo_core::capture::model::AnnotationCommand;
use snaplingo_core::capture::model::LogicalPoint;
use snaplingo_core::capture::model::LogicalRect;
use snaplingo_core::capture::AnnotationHandle;
use snaplingo_core::capture::AnnotationTool;
use snaplingo_core::capture::CaptureEditor;
use snaplingo_core::capture::CaptureEditorControlAction;
use snaplingo_core::capture::CaptureEditorStyleChange;
use snaplingo_core::capture::CaptureEditorToolbarPresentation;

#[derive(Clone, Debug)]
pub(crate) enum Request {
    SelectionInput {
        point: LogicalPoint,
        phase: &'static str,
        constrained: bool,
    },
    SelectionNudge(f64, f64),
    Render {
        rect: LogicalRect,
        annotations: Vec<AnnotationCommand>,
        revision: u64,
        excluded: Option<usize>,
        include_cursor: bool,
    },
    Output {
        command: &'static str,
        rect: LogicalRect,
        annotations: Vec<AnnotationCommand>,
        include_cursor: bool,
    },
    Control(CaptureEditorControlAction),
    Fonts,
}

pub(crate) struct EditorIvars {
    id: String,
    model: RefCell<CaptureEditor>,
    frozen: Retained<NSImage>,
    rendered: RefCell<Option<(u64, Option<usize>, Retained<NSImage>)>>,
    requested: Cell<Option<(u64, Option<usize>)>>,
    dispatch: Box<dyn Fn(Request)>,
    text: OnceCell<Retained<NSTextView>>,
    editing_text: Cell<bool>,
    fonts_requested: Cell<bool>,
    busy: Cell<bool>,
    ui_scale: f64,
    dark: bool,
    interface_theme: String,
    include_cursor: bool,
    shortcuts: HashMap<String, String>,
    selection_dragging: Cell<bool>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoAnnotationCanvas"]
    #[thread_kind = MainThreadOnly]
    #[ivars = EditorIvars]
    pub(crate) struct EditorView;
    unsafe impl NSObjectProtocol for EditorView {}
    unsafe impl NSTextDelegate for EditorView {
        #[unsafe(method(textDidChange:))]
        fn text_changed(&self, _notification: &NSNotification) {
            self.ivars().model.borrow_mut().update_text(self.ivars().text.get().unwrap().string().to_string());
            self.layout_text();
        }
    }
    unsafe impl NSTextViewDelegate for EditorView {
        #[unsafe(method(textView:doCommandBySelector:))]
        fn text_command(&self, input: &NSTextView, command: Sel) -> bool {
            if input.hasMarkedText() { return false.into(); }
            if command == sel!(insertNewline:) {
                let modifiers = self.window().and_then(|window| window.currentEvent()).map(|event| event.modifierFlags()).unwrap_or_else(NSEventModifierFlags::empty);
                if modifiers.intersects(NSEventModifierFlags::Shift | NSEventModifierFlags::Command | NSEventModifierFlags::Control) { return false.into(); }
                self.ivars().model.borrow_mut().commit_text(); self.refresh(); true
            } else if command == sel!(cancelOperation:) {
                self.ivars().model.borrow_mut().escape(); self.refresh(); true
            } else { false }
        }
    }
    impl EditorView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(isOpaque))]
        fn is_opaque(&self) -> bool { true }
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool { true }
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool { true }
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) { self.draw(); }
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            if self.ivars().busy.get() { return; }
            let point = self.point(event);
            let edge = {
                let model = self.ivars().model.borrow();
                let annotation_handle = model.selected_handles().iter().any(|(_, handle)| (point.x - handle.x).hypot(point.y - handle.y) <= 6.0);
                !annotation_handle && point.x.min(point.y).min(model.selection.width - point.x).min(model.selection.height - point.y) <= 6.0
            };
            if edge {
                self.ivars().selection_dragging.set(true);
                self.selection_input(event, "down");
                return;
            }
            self.ivars().model.borrow_mut().pointer_down(point, event.clickCount().max(1) as u32,
                event.modifierFlags().intersects(NSEventModifierFlags::Command | NSEventModifierFlags::Control));
            let move_selection = {
                let model = self.ivars().model.borrow();
                model.tool.is_none() && model.selected.is_none()
            };
            if move_selection {
                self.ivars().selection_dragging.set(true);
                self.selection_input(event, "down");
            }
            self.refresh();
        }
        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            if self.ivars().busy.get() { return; }
            if self.ivars().selection_dragging.get() { self.selection_input(event, "move"); return; }
            self.ivars().model.borrow_mut().pointer_move(self.point(event), event.modifierFlags().contains(NSEventModifierFlags::Shift));
            self.request_render();
            self.setNeedsDisplay(true);
        }
        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            if self.ivars().busy.get() { return; }
            if self.ivars().selection_dragging.replace(false) { self.selection_input(event, "up"); return; }
            self.ivars().model.borrow_mut().pointer_up(self.point(event), event.modifierFlags().contains(NSEventModifierFlags::Shift));
            self.refresh();
        }
        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, _event: &NSEvent) {
            if super::view::editor_cancel_selection(&self.ivars().id) { return; }
            self.ivars().model.borrow_mut().escape(); self.refresh();
        }
        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            if self.ivars().busy.get() || self.ivars().editing_text.get()
                || event.modifierFlags().contains(NSEventModifierFlags::Option)
                || event.scrollingDeltaY() == 0.0 { return; }
            let step = if event.modifierFlags().intersects(NSEventModifierFlags::Command | NSEventModifierFlags::Control) { 3 } else { 1 };
            self.adjust_size(if event.scrollingDeltaY() > 0.0 { step } else { -step });
        }
        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) { self.key(event); }
        #[unsafe(method(performKeyEquivalent:))]
        fn perform_key_equivalent(&self, event: &NSEvent) -> bool { self.key(event) }
    }
);

fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}
fn rgba(color: [u8; 4]) -> Retained<NSColor> {
    let [r, g, b, a] = color.map(|value| value as f64 / 255.0);
    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a)
}
fn image(pixels: image::RgbaImage) -> Result<Retained<NSImage>, String> {
    let (width, height) = pixels.dimensions();
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(), std::ptr::null_mut(), width as isize, height as isize, 8, 4, true, false,
            NSDeviceRGBColorSpace, NSBitmapFormat::AlphaNonpremultiplied, width as isize * 4, 32,
        )
    }.ok_or("Could not allocate the annotation preview")?;
    // AppKit owns this packed RGBA8 allocation. The source has the same dimensions and row stride.
    unsafe {
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bitmap.bitmapData(), pixels.len());
    }
    let image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(width as f64, height as f64));
    image.addRepresentation(&bitmap);
    Ok(image)
}

impl EditorView {
    pub(crate) fn new(
        id: &str,
        parent: &NSView,
        model: CaptureEditor,
        pixels: image::RgbaImage,
        ui_scale: f64,
        dark: bool,
        interface_theme: String,
        include_cursor: bool,
        shortcuts: HashMap<String, String>,
        dispatch: impl Fn(Request) + 'static,
    ) -> Result<Retained<Self>, String> {
        let frame = rect(0.0, 0.0, model.selection.width, model.selection.height);
        let view = Self::alloc(parent.mtm()).set_ivars(EditorIvars {
            id: id.into(),
            model: RefCell::new(model),
            frozen: image(pixels)?,
            rendered: RefCell::new(None),
            requested: Cell::new(None),
            dispatch: Box::new(dispatch),
            text: OnceCell::new(),
            editing_text: Cell::new(false),
            fonts_requested: Cell::new(false),
            busy: Cell::new(false),
            ui_scale,
            dark,
            interface_theme,
            include_cursor,
            shortcuts,
            selection_dragging: Cell::new(false),
        });
        let view: Retained<Self> = unsafe { msg_send![super(view), initWithFrame: frame] };
        view.setWantsLayer(true);
        let input =
            NSTextView::initWithFrame(NSTextView::alloc(parent.mtm()), rect(0.0, 0.0, 1.0, 1.0));
        input.setRichText(false);
        input.setImportsGraphics(false);
        input.setEditable(true);
        input.setSelectable(true);
        input.setDrawsBackground(false);
        input.setAllowsUndo(true);
        input.setVerticallyResizable(false);
        input.setHorizontallyResizable(false);
        input.setTextContainerInset(NSSize::new(0.0, 0.0));
        input.setDelegate(Some(ProtocolObject::from_ref(&*view)));
        input.setHidden(true);
        view.addSubview(&input);
        let _ = view.ivars().text.set(input);
        parent.addSubview(&view);
        Ok(view)
    }
    fn point(&self, event: &NSEvent) -> LogicalPoint {
        let point = self.convertPoint_fromView(event.locationInWindow(), None);
        LogicalPoint {
            x: point.x,
            y: point.y,
        }
    }
    fn selection_input(&self, event: &NSEvent, phase: &'static str) {
        let point = self.point(event);
        let selection = self.ivars().model.borrow().selection.clone();
        (self.ivars().dispatch)(Request::SelectionInput {
            point: LogicalPoint {
                x: point.x + selection.x,
                y: point.y + selection.y,
            },
            phase,
            constrained: event.modifierFlags().contains(NSEventModifierFlags::Shift),
        });
    }
    pub(crate) fn selection_changed(&self, selection: LogicalRect, frame: NSRect) {
        if self.ivars().model.borrow_mut().selection_changed(selection) {
            self.setFrame(frame);
            self.refresh();
        }
    }
    pub(crate) fn cancel_selection_tracking(&self) {
        self.ivars().selection_dragging.set(false);
    }
    fn excluded(&self) -> Option<usize> {
        let model = self.ivars().model.borrow();
        model
            .moving_index()
            .or_else(|| model.text.as_ref().and_then(|text| text.replacing))
    }
    fn request_render(&self) {
        let excluded = self.excluded();
        let model = self.ivars().model.borrow();
        let key = (model.revision, excluded);
        if self.ivars().requested.get() == Some(key) {
            return;
        }
        let draft = model.draft_annotation();
        let mut annotations: Vec<_> = model
            .annotations()
            .enumerate()
            .filter_map(|(index, annotation)| {
                if Some(index) == excluded {
                    draft.clone()
                } else {
                    Some(annotation.clone())
                }
            })
            .collect();
        if excluded.is_none() {
            annotations.extend(draft);
        }
        let request = Request::Render {
            rect: model.selection.clone(),
            annotations,
            revision: key.0,
            excluded,
            include_cursor: self.ivars().include_cursor,
        };
        drop(model);
        self.ivars().requested.set(Some(key));
        (self.ivars().dispatch)(request);
    }
    pub(crate) fn rendered(
        &self,
        revision: u64,
        excluded: Option<usize>,
        pixels: image::RgbaImage,
    ) -> Result<(), String> {
        if self.ivars().requested.get() != Some((revision, excluded)) {
            return Ok(());
        }
        *self.ivars().rendered.borrow_mut() = Some((revision, excluded, image(pixels)?));
        self.setNeedsDisplay(true);
        Ok(())
    }
    pub(crate) fn render_failed(&self, revision: u64, excluded: Option<usize>, error: &str) {
        if self.ivars().requested.get() == Some((revision, excluded)) {
            self.ivars().requested.set(None);
            self.report_error(error);
        }
    }
    pub(crate) fn fonts(&self, families: Vec<String>) {
        self.ivars().model.borrow_mut().styling.font_families = families;
        self.refresh();
    }
    pub(crate) fn presets(&self, colors: Vec<[u8; 4]>) {
        self.ivars().model.borrow_mut().styling.color_presets = colors;
        self.refresh();
    }
    pub(crate) fn status(&self, busy: bool, error: Option<String>) {
        self.ivars().busy.set(busy);
        self.ivars().model.borrow_mut().styling.error = error;
        self.refresh();
    }
    pub(crate) fn report_error(&self, error: &str) {
        self.ivars().model.borrow_mut().styling.error = Some(error.to_owned());
        self.refresh_toolbar();
    }
    pub(crate) fn control(&self, action: CaptureEditorControlAction) {
        if self.ivars().busy.get() {
            return;
        }
        if matches!(&action, CaptureEditorControlAction::Command(_)) {
            super::toolbar::dismiss_palette(&self.ivars().id);
        }
        let consumed = self.ivars().model.borrow_mut().control(&action);
        if !consumed {
            match action {
                CaptureEditorControlAction::Command(command) => {
                    if matches!(command, "cancel" | "refresh") {
                        (self.ivars().dispatch)(Request::Control(
                            CaptureEditorControlAction::Command(command),
                        ));
                        return;
                    }
                    self.ivars().model.borrow_mut().commit_text();
                    let model = self.ivars().model.borrow();
                    let request = Request::Output {
                        command,
                        rect: model.selection.clone(),
                        annotations: model.export_annotations(),
                        include_cursor: self.ivars().include_cursor,
                    };
                    drop(model);
                    self.ivars().busy.set(true);
                    (self.ivars().dispatch)(request);
                }
                action => (self.ivars().dispatch)(Request::Control(action)),
            }
        } else if matches!(
            action,
            CaptureEditorControlAction::Style { commit: true, .. }
                | CaptureEditorControlAction::Command(_)
        ) {
            (self.ivars().dispatch)(Request::Control(action));
        }
        self.refresh();
    }

    pub(crate) fn activate(&self) {
        self.refresh();
    }
    fn refresh(&self) {
        self.layout_text();
        self.request_render();
        self.refresh_toolbar();
        self.setNeedsDisplay(true);
        let needs_fonts = self.ivars().model.borrow().styling.text_mode
            && self.ivars().model.borrow().styling.font_families.is_empty();
        if needs_fonts && !self.ivars().fonts_requested.replace(true) {
            (self.ivars().dispatch)(Request::Fonts);
        }
    }
    fn refresh_toolbar(&self) {
        let Some(parent) = self.window() else {
            return;
        };
        let model = self.ivars().model.borrow();
        let scale = self.ivars().ui_scale;
        let parent_size = parent.frame().size;
        let size =
            super::toolbar::preferred_size(&model.styling, parent_size.width - 16.0 * scale, scale);
        let selected = self.frame();
        let frame = snaplingo_ui::capture::presentation::toolbar_frame(
            snaplingo_ui::Rect {
                x: selected.origin.x as f32,
                y: selected.origin.y as f32,
                width: selected.size.width as f32,
                height: selected.size.height as f32,
            },
            snaplingo_ui::Size {
                width: parent_size.width as f32,
                height: parent_size.height as f32,
            },
            snaplingo_ui::Size {
                width: size.width as f32,
                height: size.height as f32,
            },
            scale as f32,
        );
        let presentation = CaptureEditorToolbarPresentation {
            frame: LogicalRect {
                x: frame.x as f64,
                y: frame.y as f64,
                width: frame.width as f64,
                height: frame.height as f64,
            },
            ui_scale: scale,
            dark: self.ivars().dark,
            interface_theme: self.ivars().interface_theme.clone(),
            visible: true,
            active_tool: model.tool.map(|tool| tool.name().to_owned()),
            can_undo: model.can_undo(),
            can_redo: model.can_redo(),
            busy: self.ivars().busy.get(),
            styling: model.styling.clone(),
        };
        drop(model);
        let id = self.ivars().id.clone();
        let dispatch = Rc::new(move |action| super::view::editor_control(&id, action));
        let _ = super::toolbar::update(&self.ivars().id, parent, Some(&presentation), dispatch);
    }
    fn layout_text(&self) {
        let (draft, styling) = {
            let model = self.ivars().model.borrow();
            (model.text.clone(), model.styling.clone())
        };
        let input = self.ivars().text.get().unwrap();
        let Some(draft) = draft else {
            if self.ivars().editing_text.replace(false) {
                input.setHidden(true);
                if let Some(window) = self.window() {
                    window.makeFirstResponder(Some(self));
                }
            }
            return;
        };
        if !input.hasMarkedText() && input.string().to_string() != draft.text {
            input.setString(&NSString::from_str(&draft.text));
        }
        let font_size = styling.font_size as f64;
        let mut traits = NSFontTraitMask::empty();
        if styling.style.bold {
            traits |= NSFontTraitMask::BoldFontMask;
        }
        if styling.style.italic {
            traits |= NSFontTraitMask::ItalicFontMask;
        }
        let manager = NSFontManager::sharedFontManager(self.mtm());
        let font = manager
            .fontWithFamily_traits_weight_size(
                &NSString::from_str(&styling.style.font_family),
                traits,
                5,
                font_size,
            )
            .unwrap_or_else(|| {
                manager.convertFont_toHaveTrait(&NSFont::systemFontOfSize(font_size), traits)
            });
        input.setFont(Some(&font));
        input.setTextColor(Some(&rgba(styling.style.color)));
        let lines: Vec<_> = draft.text.split('\n').collect();
        let width = (lines
            .iter()
            .map(|line| line.encode_utf16().count())
            .max()
            .unwrap_or(1) as f64
            * font_size
            * 0.62
            + 12.0)
            .max(40.0)
            .min((self.bounds().size.width - draft.position.x).max(40.0));
        input.setFrame(rect(
            draft.position.x,
            draft.position.y - font_size,
            width,
            font_size * 1.35 * lines.len() as f64,
        ));
        input.setHidden(false);
        if !self.ivars().editing_text.replace(true) {
            if let Some(window) = self.window() {
                window.makeFirstResponder(Some(input));
            }
        }
    }
    pub(crate) fn key(&self, event: &NSEvent) -> bool {
        if self.ivars().busy.get() {
            return true;
        }
        let modifiers = event.modifierFlags();
        let command =
            modifiers.intersects(NSEventModifierFlags::Command | NSEventModifierFlags::Control);
        let shift = modifiers.contains(NSEventModifierFlags::Shift);
        let alt = modifiers.contains(NSEventModifierFlags::Option);
        let key = event
            .charactersIgnoringModifiers()
            .map(|text| text.to_string().to_lowercase())
            .unwrap_or_default();
        if self.ivars().editing_text.get()
            && (!command || matches!(key.as_str(), "a" | "c" | "v" | "x" | "z" | "y"))
        {
            return false;
        }
        if event.keyCode() == 53 && super::toolbar::dismiss_palette(&self.ivars().id) {
            return true;
        }
        if !command
            && !alt
            && (matches!(key.as_str(), "[" | "]")
                || matches!(key.as_str(), "1" | "2")
                    && !self
                        .ivars()
                        .shortcuts
                        .values()
                        .any(|shortcut| shortcut == &key))
        {
            self.adjust_size(if matches!(key.as_str(), "]" | "2") {
                1
            } else {
                -1
            });
            return true;
        }
        if !command && !alt && key == "f" {
            let filled = {
                let model = self.ivars().model.borrow();
                model
                    .styling
                    .fill_enabled
                    .then_some(!model.styling.style.filled)
            };
            if let Some(filled) = filled {
                self.control(CaptureEditorControlAction::Style {
                    change: CaptureEditorStyleChange::Filled(filled),
                    commit: false,
                });
                return true;
            }
        }
        if !command && !alt && !shift && event.keyCode() == 48 {
            let tool = self.ivars().model.borrow().tool;
            let next = match tool {
                Some(AnnotationTool::Arrow) => Some("line"),
                Some(AnnotationTool::Line) => Some("arrow"),
                _ => None,
            };
            if let Some(next) = next {
                self.control(CaptureEditorControlAction::Command(next));
                return true;
            }
        }
        if !command && !alt && matches!(event.keyCode(), 123..=126) {
            let step = if shift { 10.0 } else { 1.0 };
            let (dx, dy) = match event.keyCode() {
                123 => (-step, 0.0),
                124 => (step, 0.0),
                125 => (0.0, step),
                _ => (0.0, -step),
            };
            let moved = self
                .ivars()
                .model
                .borrow_mut()
                .nudge_selected(LogicalPoint { x: dx, y: dy });
            if !moved {
                (self.ivars().dispatch)(Request::SelectionNudge(dx, dy));
            }
            self.refresh();
            return true;
        }
        if event.keyCode() == 53 && self.ivars().selection_dragging.replace(false) {
            (self.ivars().dispatch)(Request::SelectionInput {
                point: LogicalPoint { x: 0.0, y: 0.0 },
                phase: "cancel",
                constrained: false,
            });
            return true;
        }
        let action = if event.keyCode() == 53 {
            "cancel"
        } else if event.keyCode() == 96 {
            "refresh"
        } else if event.keyCode() == 99 {
            "pin"
        } else if command {
            if alt && event.keyCode() == 51 && !shift {
                self.control(CaptureEditorControlAction::Command("clear"));
                return true;
            }
            if alt {
                return false;
            }
            match key.as_str() {
                "z" if shift => "redo",
                "z" => "undo",
                "y" => "redo",
                "c" => "copy",
                "s" if shift => "quick-save",
                "s" => "save",
                "t" => "pin",
                "p" => "print",
                _ => return false,
            }
        } else if matches!(event.keyCode(), 51 | 117) {
            "delete"
        } else if key == "\r" {
            "copy"
        } else {
            if alt {
                return false;
            }
            let key = match key.as_str() {
                value
                    if value
                        .chars()
                        .next()
                        .is_some_and(|value| ('\u{f704}'..='\u{f71b}').contains(&value)) =>
                {
                    format!("f{}", key.chars().next().unwrap() as u32 - 0xf704 + 1)
                }
                _ => key,
            };
            let tool = self
                .ivars()
                .shortcuts
                .iter()
                .find(|(_, shortcut)| shortcut.to_lowercase() == key);
            if let Some((tool, _)) = tool {
                if tool == "scrolling" {
                    self.control(CaptureEditorControlAction::Command("scrolling"));
                    return true;
                }
                if let Some(tool) = AnnotationTool::parse(&tool) {
                    let selected = self.ivars().model.borrow().tool == Some(tool);
                    self.control(CaptureEditorControlAction::Command(if selected {
                        "select"
                    } else {
                        tool.name()
                    }));
                    return true;
                }
            }
            if let Ok(index) = key.parse::<usize>() {
                let color = self
                    .ivars()
                    .model
                    .borrow()
                    .styling
                    .color_presets
                    .get(index.wrapping_sub(1))
                    .copied();
                if let Some(color) = color {
                    self.control(CaptureEditorControlAction::Style {
                        change: CaptureEditorStyleChange::Color(color),
                        commit: false,
                    });
                    return true;
                }
            }
            return false;
        };
        self.control(CaptureEditorControlAction::Command(action));
        true
    }
    fn adjust_size(&self, delta: i32) {
        let tool = self.ivars().model.borrow().tool;
        if matches!(
            tool,
            Some(AnnotationTool::Spotlight | AnnotationTool::RegionHighlight)
        ) {
            return;
        }
        let styling = self.ivars().model.borrow().styling.clone();
        let change = if styling.text_mode {
            CaptureEditorStyleChange::FontSize(
                (styling.font_size as i32 + delta).clamp(10, 96) as u32
            )
        } else if styling.blur_mode {
            CaptureEditorStyleChange::BlurRadius(
                (styling.style.blur_radius + delta as f64).clamp(4.0, 64.0),
            )
        } else {
            CaptureEditorStyleChange::StrokeWidth(
                (styling.style.stroke_width as i32 + delta).clamp(1, 32) as u32,
            )
        };
        self.control(CaptureEditorControlAction::Style {
            change,
            commit: false,
        });
    }
    fn draw(&self) {
        let model = self.ivars().model.borrow();
        let excluded = model
            .moving_index()
            .or_else(|| model.text.as_ref().and_then(|text| text.replacing));
        let rendered = self.ivars().rendered.borrow();
        let current = rendered
            .as_ref()
            .filter(|(revision, hidden, _)| *revision == model.revision && *hidden == excluded)
            .map(|(_, _, image)| image);
        let image = current.unwrap_or(&self.ivars().frozen);
        unsafe {
            image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                self.bounds(),
                rect(0.0, 0.0, image.size().width, image.size().height),
                NSCompositingOperation::Copy,
                1.0,
                true,
                None,
            );
        }
        if current.is_none() {
            for (index, annotation) in model.annotations().enumerate() {
                if Some(index) != excluded {
                    paint(annotation, false);
                }
            }
        }
        if current.is_none() {
            if let Some(draft) = model.draft_annotation() {
                paint(&draft, true);
            }
        }
        if model.moving_index().is_none() {
            if let Some(bounds) = model.selected_bounds() {
                NSColor::controlAccentColor().setStroke();
                let path = NSBezierPath::bezierPathWithRect(rect(
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                ));
                path.setLineWidth(1.0);
                path.stroke();
            }
        }
        for (handle, point) in model.selected_handles() {
            let path = NSBezierPath::bezierPathWithOvalInRect(rect(
                point.x - 4.0,
                point.y - 4.0,
                8.0,
                8.0,
            ));
            NSColor::whiteColor().setFill();
            path.fill();
            if matches!(
                handle,
                AnnotationHandle::Control1 | AnnotationHandle::Control2 | AnnotationHandle::Radius
            ) {
                NSColor::systemOrangeColor().setStroke();
            } else {
                NSColor::controlAccentColor().setStroke();
            }
            path.setLineWidth(1.0);
            path.stroke();
        }
        let size = self.bounds().size;
        for (x, y) in [
            (0.0, 0.0),
            (size.width / 2.0, 0.0),
            (size.width, 0.0),
            (size.width, size.height / 2.0),
            (size.width, size.height),
            (size.width / 2.0, size.height),
            (0.0, size.height),
            (0.0, size.height / 2.0),
        ] {
            let handle = NSBezierPath::bezierPathWithRect(rect(x - 3.0, y - 3.0, 6.0, 6.0));
            NSColor::controlAccentColor().setFill();
            handle.fill();
        }
    }
}

fn ns_point(point: &LogicalPoint) -> NSPoint {
    NSPoint::new(point.x, point.y)
}

#[cfg(feature = "test-support")]
#[derive(Debug)]
pub(crate) struct TestState {
    pub annotations: Vec<AnnotationCommand>,
    pub selection: LogicalRect,
    pub text: Option<String>,
    pub styling: snaplingo_core::capture::CaptureEditorStylePresentation,
    pub ready: bool,
    pub busy: bool,
}

#[cfg(feature = "test-support")]
impl EditorView {
    pub(crate) fn test_state(&self) -> TestState {
        let model = self.ivars().model.borrow();
        TestState {
            annotations: model.export_annotations(),
            selection: model.selection.clone(),
            text: model.text.as_ref().map(|draft| draft.text.clone()),
            styling: model.styling.clone(),
            ready: self.ivars().rendered.borrow().as_ref().is_some_and(
                |(revision, excluded, _)| {
                    self.ivars().requested.get() == Some((*revision, *excluded))
                },
            ),
            busy: self.ivars().busy.get(),
        }
    }
    pub(crate) fn test_text(&self, text: &str) {
        self.ivars()
            .text
            .get()
            .unwrap()
            .setString(&NSString::from_str(text));
        self.ivars().model.borrow_mut().update_text(text.into());
        self.layout_text();
    }
    pub(crate) fn test_commit_text(&self) -> bool {
        unsafe {
            msg_send![self, textView: &**self.ivars().text.get().unwrap(), doCommandBySelector: sel!(insertNewline:)]
        }
    }
}
fn paint(annotation: &AnnotationCommand, draft: bool) {
    let path = NSBezierPath::bezierPath();
    path.setLineCapStyle(NSLineCapStyle::Round);
    path.setLineJoinStyle(NSLineJoinStyle::Round);
    let (color, width, filled) = match annotation {
        AnnotationCommand::Rectangle {
            rect: bounds,
            color,
            stroke_width,
            filled,
            corner_radius,
        } => {
            let radius = corner_radius.min(bounds.width.min(bounds.height) / 2.0);
            path.appendBezierPathWithRoundedRect_xRadius_yRadius(
                rect(bounds.x, bounds.y, bounds.width, bounds.height),
                radius,
                radius,
            );
            (*color, *stroke_width as f64, *filled)
        }
        AnnotationCommand::Ellipse {
            rect: bounds,
            color,
            stroke_width,
            filled,
        } => {
            path.appendBezierPathWithOvalInRect(rect(
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
            ));
            (*color, *stroke_width as f64, *filled)
        }
        AnnotationCommand::Arrow {
            start,
            end,
            control1,
            control2,
            color,
            stroke_width,
        } => {
            path.moveToPoint(ns_point(start));
            let between = |t: f64| {
                NSPoint::new(
                    start.x + (end.x - start.x) * t,
                    start.y + (end.y - start.y) * t,
                )
            };
            let first = control1
                .as_ref()
                .map(ns_point)
                .unwrap_or_else(|| between(1.0 / 3.0));
            let second = control2
                .as_ref()
                .map(ns_point)
                .unwrap_or_else(|| between(2.0 / 3.0));
            path.curveToPoint_controlPoint1_controlPoint2(ns_point(end), first, second);
            let angle = (end.y - second.y).atan2(end.x - second.x);
            let head = (*stroke_width as f64 * 4.0).max(10.0);
            for delta in [-std::f64::consts::PI / 6.0, std::f64::consts::PI / 6.0] {
                path.moveToPoint(ns_point(end));
                path.lineToPoint(NSPoint::new(
                    end.x - (angle + delta).cos() * head,
                    end.y - (angle + delta).sin() * head,
                ));
            }
            (*color, *stroke_width as f64, false)
        }
        AnnotationCommand::Line {
            start,
            end,
            color,
            stroke_width,
        } => {
            path.moveToPoint(ns_point(start));
            path.lineToPoint(ns_point(end));
            (*color, *stroke_width as f64, false)
        }
        AnnotationCommand::Freehand {
            points,
            color,
            stroke_width,
        }
        | AnnotationCommand::Highlight {
            points,
            color,
            stroke_width,
        } => {
            if let Some(first) = points.first() {
                path.moveToPoint(ns_point(first));
                for point in points.iter().skip(1) {
                    path.lineToPoint(ns_point(point));
                }
            }
            (*color, *stroke_width as f64, false)
        }
        AnnotationCommand::RegionHighlight {
            rect: bounds,
            color,
            corner_radius,
        } => {
            path.appendBezierPathWithRoundedRect_xRadius_yRadius(
                rect(bounds.x, bounds.y, bounds.width, bounds.height),
                *corner_radius,
                *corner_radius,
            );
            (*color, 0.0, true)
        }
        AnnotationCommand::Blur { rect: bounds, .. }
        | AnnotationCommand::Spotlight { rect: bounds, .. }
            if draft =>
        {
            path.appendBezierPathWithRect(rect(bounds.x, bounds.y, bounds.width, bounds.height));
            ([24, 144, 255, 160], 1.0, false)
        }
        AnnotationCommand::Mosaic {
            points,
            stroke_width,
            ..
        }
        | AnnotationCommand::Eraser {
            points,
            stroke_width,
        } if draft => {
            if let Some(first) = points.first() {
                path.moveToPoint(ns_point(first));
                for point in points.iter().skip(1) {
                    path.lineToPoint(ns_point(point));
                }
            }
            ([24, 144, 255, 96], *stroke_width as f64, false)
        }
        _ => return,
    };
    let color = rgba(color);
    color.setFill();
    color.setStroke();
    path.setLineWidth(width);
    if filled {
        path.fill();
    } else {
        path.stroke();
    }
}
