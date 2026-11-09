//! AppKit supplies only input and pixels; shared Rust owns every page layout.
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use objc2::rc::Retained;
use objc2::{
    define_class, msg_send,
    runtime::{AnyObject, Sel},
    sel, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSBitmapFormat, NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace, NSEvent,
    NSEventModifierFlags, NSGraphicsContext, NSImage, NSImageInterpolation, NSTextInputClient,
    NSTrackingArea, NSTrackingAreaOptions, NSView, NSWindowDidBecomeKeyNotification,
    NSWindowDidResignKeyNotification,
};
use objc2_foundation::{
    NSArray, NSAttributedString, NSAttributedStringKey, NSNotification, NSNotificationCenter,
    NSObjectProtocol, NSPoint, NSRange, NSRect, NSSize,
};
use snaplingo_render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{
    Event, Key, Modifiers, Page, Point, Scene, Section, SettingsUi, Size, Snapshot, WindowControls,
    WindowSnapshot, WindowUi,
};

use super::shared_page_macos::SharedFeaturePage;

pub(crate) struct ViewState {
    page: RefCell<Weak<SharedFeaturePage>>,
    pub(super) ui: RefCell<WindowUi>,
    controls: RefCell<WindowControls>,
    renderer: RefCell<Renderer>,
    metrics: RefCell<RendererTextMetrics>,
    pub(super) scene: RefCell<Option<Scene>>,
    pub(super) key_modifiers: Cell<Modifiers>,
    pub(super) ime_committed: Cell<bool>,
    pub(super) ime_was_composing: Cell<bool>,
    image: RefCell<Option<Retained<NSImage>>>,
    raster_scale: Cell<f32>,
    shell_width: Cell<f64>,
    viewport_width: Cell<f64>,
    drawing: Cell<bool>,
    full_window: Cell<bool>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name="SnapLingoSharedSettingsCanvas"]
    #[thread_kind=MainThreadOnly]
    #[ivars=ViewState]
    pub(crate) struct SharedFeatureView;
    unsafe impl NSObjectProtocol for SharedFeatureView {}
    unsafe impl NSTextInputClient for SharedFeatureView {
        #[unsafe(method(insertText:replacementRange:))]
        unsafe fn insert_text(&self, text:&AnyObject, replacement:NSRange){self.ime_insert(text,replacement);}
        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        unsafe fn marked_text(&self,text:&AnyObject,selection:NSRange,replacement:NSRange){self.ime_mark(text,selection,replacement);}
        #[unsafe(method(unmarkText))] fn unmark(&self){self.ime_unmark();}
        #[unsafe(method(hasMarkedText))] fn has_marked(&self)->bool{self.ime_marked_range().length>0}
        #[unsafe(method(markedRange))] fn marked_range(&self)->NSRange{self.ime_marked_range()}
        #[unsafe(method(selectedRange))] fn selected_range(&self)->NSRange{self.ime_selected_range()}
        #[unsafe(method_id(validAttributesForMarkedText))] fn marked_attributes(&self)->Retained<NSArray<NSAttributedStringKey>>{NSArray::new()}
        #[unsafe(method_id(attributedSubstringForProposedRange:actualRange:))]
        unsafe fn substring(&self,range:NSRange,actual:*mut NSRange)->Option<Retained<NSAttributedString>>{unsafe{self.ime_substring(range,actual)}}
        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        unsafe fn first_rect(&self,range:NSRange,actual:*mut NSRange)->NSRect{unsafe{self.ime_first_rect(range,actual)}}
        #[unsafe(method(characterIndexForPoint:))] fn character_index(&self,_:NSPoint)->usize{objc2_foundation::NSNotFound as usize}
        #[unsafe(method(doCommandBySelector:))] unsafe fn command(&self,command:Sel){self.ime_command(command);}
    }
    impl SharedFeatureView {
        #[unsafe(method(isFlipped))] fn flipped(&self)->bool{true}
        #[unsafe(method(acceptsFirstResponder))] fn accepts(&self)->bool{true}
        #[unsafe(method(viewDidMoveToWindow))] fn moved_to_window(&self){self.redraw();}
        #[unsafe(method(viewDidChangeBackingProperties))] fn backing_changed(&self){self.redraw();}
        #[unsafe(method(sharedWindowFocusChanged:))] fn window_focus(&self,event:&NSNotification){
            if event.object().is_some_and(|object|self.window().is_some_and(|window|std::ptr::eq(Retained::as_ptr(&object),Retained::as_ptr(&window).cast()))){
                if let Some(page)=self.ivars().page.borrow().upgrade(){page.focus_changed();}
            }
        }
        #[unsafe(method(setFrameSize:))] fn resized(&self,size:NSSize){unsafe{let _:()=msg_send![super(self),setFrameSize:size];}self.redraw();}
        #[unsafe(method(drawRect:))] fn draw(&self,_:NSRect){
            if self.ivars().image.borrow().is_none(){self.redraw();}
            if let Some(image)=self.ivars().image.borrow().as_ref(){
                if let Some(context)=NSGraphicsContext::currentContext(){context.setImageInterpolation(NSImageInterpolation::None);}
                unsafe{image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(self.bounds(),NSRect::ZERO,NSCompositingOperation::SourceOver,1.,true,None);}
            }
        }
        #[unsafe(method(mouseDown:))] fn mouse_down(&self,event:&NSEvent){self.cancel_composition();if let Some(window)=self.window(){window.makeFirstResponder(Some(self));}self.input(Event::PointerDown(self.point(event)));}
        #[unsafe(method(mouseDragged:))] fn dragged(&self,event:&NSEvent){self.input(Event::PointerMove(self.point(event)));}
        #[unsafe(method(mouseMoved:))] fn moved(&self,event:&NSEvent){self.input(Event::PointerMove(self.point(event)));}
        #[unsafe(method(mouseUp:))] fn mouse_up(&self,event:&NSEvent){self.input(Event::PointerUp(self.point(event)));}
        #[unsafe(method(scrollWheel:))] fn wheel(&self,event:&NSEvent){let factor=if event.hasPreciseScrollingDeltas(){1.}else{32.};self.input(Event::Scroll{position:self.point(event),delta_y:(-event.scrollingDeltaY()*factor/self.zoom())as f32});}
        #[unsafe(method(keyDown:))] fn down(&self,event:&NSEvent){self.text_key_down(event);}
        #[unsafe(method(performKeyEquivalent:))] fn equivalent(&self,event:&NSEvent)->bool{self.text_key_equivalent(event)}
        #[unsafe(method(keyUp:))] fn up(&self,event:&NSEvent){self.key(event,false);}
        #[unsafe(method(resignFirstResponder))] fn resigned(&self)->bool{self.cancel_composition();self.input(Event::FocusLost);true}
        #[unsafe(method(copy:))] fn copy(&self,_:Option<&AnyObject>){self.text_shortcut('C');}
        #[unsafe(method(cut:))] fn cut(&self,_:Option<&AnyObject>){self.text_shortcut('X');}
        #[unsafe(method(paste:))] fn paste(&self,_:Option<&AnyObject>){self.text_shortcut('V');}
        #[unsafe(method(selectAll:))] fn select_all(&self,_:Option<&AnyObject>){self.text_shortcut('A');}
    }
);
impl SharedFeatureView {
    pub(crate) fn new(mtm: MainThreadMarker, page: Page, snapshot: Snapshot) -> Retained<Self> {
        let mut ui = SettingsUi::new(snapshot);
        ui.update(Event::Activate(snaplingo_ui::Target::Navigate(page)));
        ui.set_embedded(true);
        let view = Self::alloc(mtm).set_ivars(ViewState {
            page: RefCell::new(Weak::new()),
            ui: RefCell::new(WindowUi::Settings(ui)),
            controls: RefCell::new(WindowControls::Settings),
            renderer: RefCell::new(Renderer::default()),
            metrics: RefCell::new(RendererTextMetrics::default()),
            scene: RefCell::new(None),
            key_modifiers: Cell::new(Modifiers::default()),
            ime_committed: Cell::new(false),
            ime_was_composing: Cell::new(false),
            image: RefCell::new(None),
            raster_scale: Cell::new(0.),
            shell_width: Cell::new(900.),
            viewport_width: Cell::new(900.),
            drawing: Cell::new(false),
            full_window: Cell::new(false),
        });
        let view: Retained<Self> = unsafe { msg_send![super(view),initWithFrame:NSRect::ZERO] };
        let tracking = unsafe {
            NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                NSRect::ZERO,
                NSTrackingAreaOptions::MouseMoved
                    | NSTrackingAreaOptions::ActiveInKeyWindow
                    | NSTrackingAreaOptions::InVisibleRect,
                Some(&view),
                None,
            )
        };
        view.addTrackingArea(&tracking);
        unsafe {
            let notifications = NSNotificationCenter::defaultCenter();
            for name in [
                NSWindowDidResignKeyNotification,
                NSWindowDidBecomeKeyNotification,
            ] {
                notifications.addObserver_selector_name_object(
                    &view,
                    sel!(sharedWindowFocusChanged:),
                    Some(name),
                    None,
                );
            }
        }
        view
    }
    pub(crate) fn bind(&self, page: Weak<SharedFeaturePage>) {
        *self.ivars().page.borrow_mut() = page;
    }
    pub(crate) fn set_full_window(&self, full: bool) {
        self.ivars().full_window.set(full);
        if let WindowUi::Settings(ui) = &mut *self.ivars().ui.borrow_mut() {
            ui.set_embedded(!full);
        }
        self.redraw();
    }
    pub(crate) fn presentation_region(&self, scene: &Scene) -> snaplingo_ui::Rect {
        if self.ivars().full_window.get() {
            snaplingo_ui::Rect {
                x: 0.,
                y: 0.,
                width: scene.size.width,
                height: scene.size.height,
            }
        } else {
            scene.content
        }
    }
    pub(crate) fn set_layout_widths(&self, shell: f64, viewport: f64) {
        self.ivars().shell_width.set(shell);
        self.ivars().viewport_width.set(viewport);
        self.redraw();
    }
    pub(crate) fn set_layout(&self, shell: f64, viewport: f64, size: NSSize) {
        self.ivars().shell_width.set(shell);
        self.ivars().viewport_width.set(viewport);
        self.setFrameSize(size);
        self.redraw();
    }
    pub(crate) fn refresh(&self, snapshot: Snapshot) {
        self.refresh_window(WindowSnapshot::Settings(snapshot));
    }
    pub(crate) fn refresh_window(&self, snapshot: WindowSnapshot) {
        self.ivars().ui.borrow_mut().snapshot(snapshot);
        self.redraw();
    }
    pub(crate) fn input(&self, event: Event) {
        self.window_input(snaplingo_ui::WindowInput::Event(event));
    }
    pub(crate) fn window_input(&self, event: snaplingo_ui::WindowInput) {
        let (page, actions) = {
            let mut ui = self.ivars().ui.borrow_mut();
            let page = ui.page();
            (page, ui.input(event))
        };
        if let Some(owner) = self
            .ivars()
            .page
            .borrow()
            .upgrade()
            .filter(|_| !actions.is_empty())
        {
            owner.perform_window(page, actions);
            return;
        }
        self.redraw();
    }
    pub(crate) fn jump(&self, id: &str) -> Result<(), String> {
        let section = match id {
            "hotkeys" => Section::Hotkeys,
            "recognition" => Section::Recognition,
            "language" => Section::Language,
            "window" => Section::Window,
            "behavior" => Section::Behavior,
            "output" => Section::Output,
            "capture-interface" => Section::CaptureInterface,
            "interface" => Section::Interface,
            "permissions" => Section::Permissions,
            "network" => Section::Network,
            "maintenance" => Section::Maintenance,
            "experimental" => Section::Experimental,
            "about" => Section::About,
            _ => return Err("Unknown shared Settings section".into()),
        };
        self.input(Event::Activate(snaplingo_ui::Target::Section(section)));
        Ok(())
    }
    fn zoom(&self) -> f64 {
        (self.ivars().viewport_width.get() / self.ivars().shell_width.get()).max(0.01)
    }
    fn point(&self, event: &NSEvent) -> Point {
        let p = self.convertPoint_fromView(event.locationInWindow(), None);
        let origin = self
            .ivars()
            .scene
            .borrow()
            .as_ref()
            .map(|s| self.presentation_region(s))
            .unwrap_or_default();
        Point {
            x: p.x as f32 + origin.x,
            y: p.y as f32 + origin.y,
        }
    }
    pub(crate) fn key(&self, event: &NSEvent, down: bool) {
        if let Some(key) = key(event.keyCode()) {
            self.input(if down {
                Event::KeyDown {
                    key,
                    modifiers: modifiers(event.modifierFlags()),
                    repeat: event.isARepeat(),
                }
            } else {
                Event::KeyUp { key }
            });
        }
    }
    pub(crate) fn redraw(&self) {
        if self.ivars().drawing.replace(true) {
            return;
        }
        let result = (|| {
            let size = self.bounds().size;
            if size.width < 1. || size.height < 1. {
                return Ok(());
            }
            let shell_width = self.ivars().shell_width.get();
            // The shared layout owns the top navigation height. The embedded
            // host supplies only the remaining content height.
            let first = self.ivars().ui.borrow_mut().frame(
                Size {
                    width: shell_width as f32,
                    height: size.height as f32,
                },
                1.,
                &mut *self.ivars().metrics.borrow_mut(),
            );
            let frame = self.ivars().ui.borrow_mut().frame(
                Size {
                    width: shell_width as f32,
                    height: size.height as f32
                        + if self.ivars().full_window.get() {
                            0.
                        } else {
                            first.scene.content.y
                        },
                },
                1.,
                &mut *self.ivars().metrics.borrow_mut(),
            );
            let scene = frame.scene;
            *self.ivars().controls.borrow_mut() = frame.controls;
            let scale = self.window().map(|w| w.backingScaleFactor()).unwrap_or(1.) * self.zoom();
            // Focus, unchanged settings notifications and the host's paired
            // width/frame updates can produce the same scene. Keep its pixels;
            // input and accessibility still receive the current scene below.
            if self.ivars().raster_scale.get() == scale as f32
                && self.ivars().scene.borrow().as_ref() == Some(&scene)
                && self.ivars().image.borrow().is_some()
            {
                if let Some(page) = self.ivars().page.borrow().upgrade() {
                    page.accessibility_changed();
                }
                return Ok(());
            }
            let frame = self.ivars().renderer.borrow_mut().render_region(
                &scene,
                self.presentation_region(&scene),
                scale as f32,
            )?;
            let image = frame_image(&frame)?;
            *self.ivars().scene.borrow_mut() = Some(scene);
            *self.ivars().image.borrow_mut() = Some(image);
            self.ivars().raster_scale.set(scale as f32);
            if let Some(context) = self.inputContext() {
                context.invalidateCharacterCoordinates();
            }
            self.setNeedsDisplay(true);
            if let Some(page) = self.ivars().page.borrow().upgrade() {
                page.accessibility_changed();
            }
            Ok::<_, String>(())
        })();
        self.ivars().drawing.set(false);
        if let Err(error) = result {
            log::warn!("Failed to render shared Settings page: {error}");
        }
    }
    pub(crate) fn with_scene<T>(
        &self,
        read: impl FnOnce(&Scene, &SettingsUi, f64) -> T,
    ) -> Option<T> {
        let scene = self.ivars().scene.borrow();
        scene.as_ref().and_then(|scene| {
            let ui = self.ivars().ui.borrow();
            let preferences = ui.preferences()?;
            Some(read(
                scene,
                preferences,
                // AccessKit converts view coordinates through AppKit, which
                // already applies the enclosing host's UI zoom.
                self.window().map(|w| w.backingScaleFactor()).unwrap_or(1.),
            ))
        })
    }
    pub(crate) fn with_window<T>(
        &self,
        read: impl FnOnce(&snaplingo_ui::WindowFrame, &WindowUi, f64) -> T,
    ) -> Option<T> {
        let scene = self.ivars().scene.borrow();
        scene.as_ref().map(|scene| {
            read(
                &snaplingo_ui::WindowFrame {
                    scene: scene.clone(),
                    controls: self.ivars().controls.borrow().clone(),
                },
                &self.ivars().ui.borrow(),
                self.window().map(|w| w.backingScaleFactor()).unwrap_or(1.),
            )
        })
    }
    pub(crate) fn page(&self) -> Page {
        self.ivars().ui.borrow().page()
    }
    pub(crate) fn recording_index(&self) -> Option<usize> {
        self.ivars()
            .ui
            .borrow()
            .preferences()
            .and_then(SettingsUi::recording_index)
    }
    #[cfg(test)]
    pub(crate) fn test_snapshot(&self) -> Snapshot {
        self.ivars()
            .ui
            .borrow()
            .preferences()
            .unwrap()
            .snapshot()
            .clone()
    }
    #[cfg(test)]
    pub(crate) fn test_kind(&self) -> super::FeatureKind {
        if self.page() == Page::Ocr {
            super::FeatureKind::Ocr
        } else {
            super::FeatureKind::Translation
        }
    }
    #[cfg(test)]
    pub(crate) fn test_recording_id(&self) -> Option<String> {
        self.ivars()
            .page
            .borrow()
            .upgrade()
            .and_then(|page| page.test_recording_id())
    }
    #[cfg(test)]
    pub(crate) fn test_geometry(&self) -> serde_json::Value {
        self.with_window(|frame,_,_| { let scene=&frame.scene; serde_json::json!({"shellWidth":self.ivars().shell_width.get(),"viewportWidth":self.ivars().viewport_width.get(),"content":{"x":scene.content.x,"y":scene.content.y,"width":scene.content.width,"height":scene.content.height},"controls":scene.geometry.len(),"pixelWidth":self.ivars().image.borrow().as_ref().map(|image|image.size().width),"pixelHeight":self.ivars().image.borrow().as_ref().map(|image|image.size().height),"rasterIdentity":self.ivars().image.borrow().as_ref().map(|image|Retained::as_ptr(image) as usize)}) }).unwrap_or_default()
    }
    #[cfg(test)]
    pub(crate) fn test_choose(&self, field: super::FeatureField, value: &str) {
        let target = match field {
            super::FeatureField::Ocr(f) => snaplingo_ui::Target::Ocr(
                snaplingo_ui::OcrField::ALL
                    .into_iter()
                    .find(|field| snaplingo_settings::ocr_field(*field) == f)
                    .unwrap(),
            ),
            super::FeatureField::Translation(f) => snaplingo_ui::Target::Translation(
                snaplingo_ui::TranslationField::ALL
                    .into_iter()
                    .find(|field| snaplingo_settings::translation_field(*field) == f)
                    .unwrap(),
            ),
        };
        self.input(Event::SetValue(
            target,
            snaplingo_ui::Value::Text(value.into()),
        ));
    }
}
impl Drop for SharedFeatureView {
    fn drop(&mut self) {
        unsafe {
            NSNotificationCenter::defaultCenter().removeObserver(self);
        }
    }
}
pub(crate) fn frame_image(frame: &snaplingo_render::Frame) -> Result<Retained<NSImage>, String> {
    let bitmap=unsafe{NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(NSBitmapImageRep::alloc(),std::ptr::null_mut(),frame.width()as isize,frame.height()as isize,8,4,true,false,NSDeviceRGBColorSpace,NSBitmapFormat::empty(),frame.width()as isize*4,32)}.ok_or("Failed to allocate shared Settings pixels")?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            frame.rgba().as_ptr(),
            bitmap.bitmapData(),
            frame.rgba().len(),
        );
    }
    let image = NSImage::initWithSize(
        NSImage::alloc(),
        NSSize::new(frame.width() as f64, frame.height() as f64),
    );
    image.addRepresentation(&bitmap);
    Ok(image)
}
fn modifiers(flags: NSEventModifierFlags) -> Modifiers {
    Modifiers {
        shift: flags.contains(NSEventModifierFlags::Shift),
        alt: flags.contains(NSEventModifierFlags::Option),
        control: flags.contains(NSEventModifierFlags::Control),
        meta: flags.contains(NSEventModifierFlags::Command),
    }
}
fn key(code: u16) -> Option<Key> {
    Some(match code {
        53 => Key::Escape,
        48 => Key::Tab,
        36 | 76 => Key::Enter,
        49 => Key::Space,
        51 => Key::Backspace,
        117 => Key::Delete,
        123 => Key::ArrowLeft,
        124 => Key::ArrowRight,
        125 => Key::ArrowDown,
        126 => Key::ArrowUp,
        115 => Key::Home,
        119 => Key::End,
        116 => Key::PageUp,
        121 => Key::PageDown,
        0 => Key::Character('A'),
        1 => Key::Character('S'),
        2 => Key::Character('D'),
        3 => Key::Character('F'),
        4 => Key::Character('H'),
        5 => Key::Character('G'),
        6 => Key::Character('Z'),
        7 => Key::Character('X'),
        8 => Key::Character('C'),
        9 => Key::Character('V'),
        11 => Key::Character('B'),
        12 => Key::Character('Q'),
        13 => Key::Character('W'),
        14 => Key::Character('E'),
        15 => Key::Character('R'),
        16 => Key::Character('Y'),
        17 => Key::Character('T'),
        18 => Key::Character('1'),
        19 => Key::Character('2'),
        20 => Key::Character('3'),
        21 => Key::Character('4'),
        22 => Key::Character('6'),
        23 => Key::Character('5'),
        25 => Key::Character('9'),
        26 => Key::Character('7'),
        28 => Key::Character('8'),
        29 => Key::Character('0'),
        31 => Key::Character('O'),
        32 => Key::Character('U'),
        34 => Key::Character('I'),
        35 => Key::Character('P'),
        37 => Key::Character('L'),
        38 => Key::Character('J'),
        40 => Key::Character('K'),
        45 => Key::Character('N'),
        46 => Key::Character('M'),
        122 => Key::Function(1),
        120 => Key::Function(2),
        99 => Key::Function(3),
        118 => Key::Function(4),
        96 => Key::Function(5),
        97 => Key::Function(6),
        98 => Key::Function(7),
        100 => Key::Function(8),
        101 => Key::Function(9),
        109 => Key::Function(10),
        103 => Key::Function(11),
        111 => Key::Function(12),
        105 => Key::Function(13),
        107 => Key::Function(14),
        113 => Key::Function(15),
        106 => Key::Function(16),
        64 => Key::Function(17),
        79 => Key::Function(18),
        80 => Key::Function(19),
        90 => Key::Function(20),
        _ => return None,
    })
}
