//! A self-drawn AppKit sheet for the existing history/favorite capacity draft.
use accesskit::{
    Action, ActionData, ActionHandler, ActionRequest, ActivationHandler, Node, NodeId, Role,
    TreeId, TreeInfo, TreeUpdate,
};
use accesskit_macos::SubclassingAdapter;
use objc2::{
    define_class, msg_send, rc::Retained, DefinedClass, MainThreadMarker, MainThreadOnly, Message,
};
use objc2_app_kit::{
    NSBackingStoreType, NSCompositingOperation, NSEvent, NSEventModifierFlags, NSImage, NSView,
    NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize};
use snaplingo_render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{capacity::*, Key, Modifiers, Point};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

struct CanvasState {
    owner: RefCell<Weak<Sheet>>,
    ui: RefCell<CapacityUi>,
    renderer: RefCell<Renderer>,
    metrics: RefCell<RendererTextMetrics>,
    image: RefCell<Option<Retained<NSImage>>>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name="SnapLingoNativeCapacityCanvas"]
    #[thread_kind=MainThreadOnly]
    #[ivars=CanvasState]
    struct Canvas;
    unsafe impl NSObjectProtocol for Canvas {}
    impl Canvas {
        #[unsafe(method(isFlipped))]fn flipped(&self)->bool{true}
        #[unsafe(method(acceptsFirstResponder))]fn accepts(&self)->bool{true}
        #[unsafe(method(viewDidChangeBackingProperties))]fn backing_changed(&self){if let Some(owner)=self.ivars().owner.borrow().upgrade(){owner.redraw();}}
        #[unsafe(method(drawRect:))]fn draw(&self,_:NSRect){
            if let Some(image)=self.ivars().image.borrow().as_ref(){unsafe{image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(self.bounds(),NSRect::ZERO,NSCompositingOperation::SourceOver,1.,true,None);}}
        }
        #[unsafe(method(mouseDown:))]fn mouse(&self,event:&NSEvent){
            if let Some(window)=self.window(){window.makeFirstResponder(Some(self));}
            let point=self.convertPoint_fromView(event.locationInWindow(),None);
            self.input(CapacityInput::Pointer(Point{x:point.x as f32,y:point.y as f32}));
        }
        #[unsafe(method(keyDown:))]fn key(&self,event:&NSEvent){
            let flags=event.modifierFlags();let modifiers=Modifiers{meta:flags.contains(NSEventModifierFlags::Command),control:flags.contains(NSEventModifierFlags::Control),alt:flags.contains(NSEventModifierFlags::Option),shift:flags.contains(NSEventModifierFlags::Shift)};
            let key=match event.keyCode(){53=>Some(Key::Escape),48=>Some(Key::Tab),36|76=>Some(Key::Enter),51=>Some(Key::Backspace),117=>Some(Key::Delete),123=>Some(Key::ArrowLeft),124=>Some(Key::ArrowRight),125=>Some(Key::ArrowDown),126=>Some(Key::ArrowUp),115=>Some(Key::Home),119=>Some(Key::End),0 if modifiers.meta=>Some(Key::Character('A')),_=>None};
            if let Some(key)=key{self.input(CapacityInput::Key(key,modifiers));}
            else if !modifiers.meta&&!modifiers.control{if let Some(text)=event.characters(){self.input(CapacityInput::Text(text.to_string()));}}
        }
    }
);
impl Canvas {
    fn input(&self, input: CapacityInput) {
        let intent = self.ivars().ui.borrow_mut().update(input);
        if let Some(owner) = self.ivars().owner.borrow().upgrade() {
            if let Some(intent) = intent {
                owner.perform(intent);
                return;
            }
            owner.redraw();
        }
    }
}
#[derive(Clone)]
struct Semantics {
    nodes: BTreeMap<NodeId, Node>,
    focus: NodeId,
}
impl Semantics {
    fn update(&self) -> TreeUpdate {
        TreeUpdate {
            nodes: self
                .nodes
                .iter()
                .map(|(id, node)| (*id, node.clone()))
                .collect(),
            tree: Some(TreeInfo::new(NodeId(1))),
            tree_id: TreeId::ROOT,
            focus: self.focus,
        }
    }
}
struct Initial(Arc<Mutex<Option<Semantics>>>);
impl ActivationHandler for Initial {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.0.lock().unwrap().as_ref().map(Semantics::update)
    }
}
struct Actions;
impl ActionHandler for Actions {
    fn do_action(&mut self, request: ActionRequest) {
        crate::infrastructure::ui_thread::dispatch(move || {
            if request.target_tree != TreeId::ROOT {
                return;
            }
            let target = match request.target_node.0.checked_sub(10) {
                Some(0) => CapacityTarget::Close,
                Some(1) => CapacityTarget::Cleanup,
                Some(2) => CapacityTarget::Days,
                Some(3) => CapacityTarget::History,
                Some(4) => CapacityTarget::Favorites,
                Some(5) => CapacityTarget::Cancel,
                Some(6) => CapacityTarget::Save,
                _ => return,
            };
            SHEET.with_borrow(|slot| {
                if let Some(sheet) = slot {
                    let input = match request.action {
                        Action::Click => CapacityInput::Activate(target),
                        Action::Focus => CapacityInput::Focus(target),
                        Action::SetValue => match request.data {
                            Some(ActionData::Value(value)) => {
                                CapacityInput::SetValue(target, value.into())
                            }
                            Some(ActionData::NumericValue(value)) if value.is_finite() => {
                                CapacityInput::SetValue(target, format!("{value:.0}"))
                            }
                            _ => return,
                        },
                        Action::Increment | Action::Decrement => {
                            sheet.view.input(CapacityInput::Focus(target));
                            CapacityInput::Key(
                                if request.action == Action::Increment {
                                    Key::ArrowUp
                                } else {
                                    Key::ArrowDown
                                },
                                Modifiers::default(),
                            )
                        }
                        _ => return,
                    };
                    sheet.view.input(input);
                }
            });
        });
    }
}
struct Sheet {
    parent: Retained<NSWindow>,
    window: Retained<NSWindow>,
    view: Retained<Canvas>,
    state: Arc<crate::AppState>,
    adapter: RefCell<Option<SubclassingAdapter>>,
    semantics: Arc<Mutex<Option<Semantics>>>,
}
thread_local! {static SHEET:RefCell<Option<Rc<Sheet>>>=const{RefCell::new(None)};}

pub(crate) fn show(parent: &NSWindow, state: Arc<crate::AppState>) -> Result<(), String> {
    close_for(parent);
    let mtm = MainThreadMarker::new().ok_or("Capacity sheet requires the UI thread")?;
    let saved = state
        .settings
        .configuration
        .snapshot()
        .map_err(|e| e.to_string())?;
    let mut shell = snaplingo_settings::project_snapshot(
        &saved,
        &state
            .settings
            .hotkeys
            .snapshot()
            .map_err(|e| e.to_string())?,
        &state.settings.hotkeys.default_snapshot(),
    );
    shell.dark = crate::infrastructure::system::native_theme::is_dark(&saved.general.theme);
    let value = CapacityValue {
        automatic_cleanup: saved.history.auto_cleanup_enabled,
        retention_days: saved.history.retention_days,
        history_limit: saved.history.maximum_records,
        favorites_limit: saved.history.maximum_favorites,
    };
    let interface_scale =
        crate::application::capture::interface_scale(&saved.screenshot.capture_ui_scale, None);
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            NSRect::new(
                NSPoint::ZERO,
                NSSize::new(440. * interface_scale, 510. * interface_scale),
            ),
            NSWindowStyleMask::Titled,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    let view = Canvas::alloc(mtm).set_ivars(CanvasState {
        owner: RefCell::new(Weak::new()),
        ui: RefCell::new(CapacityUi::new(shell, value)),
        renderer: RefCell::new(Renderer::default()),
        metrics: RefCell::new(RendererTextMetrics::default()),
        image: RefCell::new(None),
    });
    let view: Retained<Canvas> = unsafe {
        msg_send![super(view),initWithFrame:NSRect::new(NSPoint::ZERO,NSSize::new(440.,510.))]
    };
    window.setContentView(Some(&view));
    view.setFrameSize(NSSize::new(440. * interface_scale, 510. * interface_scale));
    view.setBoundsSize(NSSize::new(440., 510.));
    let sheet = Rc::new(Sheet {
        parent: parent.retain(),
        window,
        view,
        state,
        adapter: RefCell::new(None),
        semantics: Arc::new(Mutex::new(None)),
    });
    *sheet.view.ivars().owner.borrow_mut() = Rc::downgrade(&sheet);
    *sheet.adapter.borrow_mut() = Some(unsafe {
        SubclassingAdapter::new(
            Retained::as_ptr(&sheet.view).cast_mut().cast(),
            Initial(sheet.semantics.clone()),
            Actions,
        )
    });
    SHEET.with_borrow_mut(|slot| *slot = Some(sheet.clone()));
    sheet.redraw();
    parent.beginSheet_completionHandler(&sheet.window, None);
    sheet.window.makeFirstResponder(Some(&sheet.view));
    Ok(())
}
pub(crate) fn close_for(parent: &NSWindow) {
    let sheet = SHEET.with_borrow_mut(|slot| {
        if slot.as_ref().is_some_and(|sheet| &*sheet.parent == parent) {
            slot.take()
        } else {
            None
        }
    });
    if let Some(sheet) = sheet {
        sheet.parent.endSheet(&sheet.window);
        sheet.window.orderOut(None);
        sheet.adapter.borrow_mut().take();
    }
}
#[cfg(test)]
pub(crate) fn test_input(input: CapacityInput) -> bool {
    let sheet = SHEET.with_borrow(Clone::clone);
    if let Some(sheet) = sheet {
        sheet.view.input(input);
        true
    } else {
        false
    }
}
#[cfg(test)]
pub(crate) fn test_is_open() -> bool {
    SHEET.with_borrow(Option::is_some)
}
impl Sheet {
    fn perform(&self, intent: CapacityIntent) {
        if let CapacityIntent::Save(value) = intent {
            let saved =
                self.state
                    .settings
                    .configuration
                    .update_history(crate::domain::HistorySettings {
                        auto_cleanup_enabled: value.automatic_cleanup,
                        retention_days: value.retention_days,
                        maximum_records: value.history_limit,
                        maximum_favorites: value.favorites_limit,
                    });
            if let Err(error) = saved {
                self.view.ivars().ui.borrow_mut().error(error.to_string());
                self.redraw();
                return;
            }
        }
        close_for(&self.parent);
    }
    fn redraw(&self) {
        let scale = self.window.backingScaleFactor();
        let zoom = self.view.frame().size.width / 440.;
        let frame = self
            .view
            .ivars()
            .ui
            .borrow_mut()
            .frame(1., &mut *self.view.ivars().metrics.borrow_mut());
        match self
            .view
            .ivars()
            .renderer
            .borrow_mut()
            .render(&frame.scene, (scale * zoom) as f32)
            .and_then(|frame| crate::infrastructure::system::settings_window::frame_image(&frame))
        {
            Ok(image) => *self.view.ivars().image.borrow_mut() = Some(image),
            Err(error) => log::warn!("Failed to draw capacity sheet: {error}"),
        }
        self.view.setNeedsDisplay(true);
        let mut nodes = BTreeMap::new();
        let mut root = Node::new(Role::Dialog);
        root.set_label("历史与收藏");
        root.set_modal();
        let mut children = vec![];
        let mut focus = NodeId(1);
        for control in frame.controls {
            let id = NodeId(10 + control.target as u64);
            children.push(id);
            let mut node = Node::new(if control.target == CapacityTarget::Cleanup {
                Role::Switch
            } else if control.target.range().is_some() {
                Role::SpinButton
            } else {
                Role::Button
            });
            node.set_label(control.target.label());
            let r = control.rect;
            node.set_bounds(accesskit::Rect::new(
                r.x as f64 * scale,
                r.y as f64 * scale,
                (r.x + r.width) as f64 * scale,
                (r.y + r.height) as f64 * scale,
            ));
            node.add_action(Action::Click);
            node.add_action(Action::Focus);
            if let Some(value) = control.checked {
                node.set_toggled(value.into());
            }
            if let Some(value) = control.value {
                node.set_value(&value);
            }
            if let Some((min, max, step)) = control.target.range() {
                node.set_min_numeric_value(min as f64);
                node.set_max_numeric_value(max as f64);
                node.set_numeric_value_step(step as f64);
                node.add_action(Action::SetValue);
                node.add_action(Action::Increment);
                node.add_action(Action::Decrement);
            }
            if self.view.ivars().ui.borrow().focused() == Some(control.target) {
                focus = id;
            }
            nodes.insert(id, node);
        }
        root.set_children(children);
        nodes.insert(NodeId(1), root);
        let semantics = Semantics { nodes, focus };
        *self.semantics.lock().unwrap() = Some(semantics.clone());
        if let Some(adapter) = self.adapter.borrow_mut().as_mut() {
            if let Some(events) = adapter.update_if_active(|| semantics.update()) {
                events.raise();
            }
        }
    }
}
