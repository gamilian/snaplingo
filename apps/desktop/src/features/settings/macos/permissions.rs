//! Self-drawn first-use guide, confined to the standalone Settings window.
use crate::features::settings::presenter::permission_prompt::{
    PermissionPromptSession, Projection,
};
use accesskit::{
    Action, ActionHandler, ActionRequest, ActivationHandler, Node, NodeId, Role, TreeId, TreeInfo,
    TreeUpdate,
};
use accesskit_macos::SubclassingAdapter;
use objc2::{
    define_class, msg_send, rc::Retained, runtime::ProtocolObject, DefinedClass, MainThreadMarker,
    MainThreadOnly, Message,
};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSCompositingOperation, NSEvent, NSEventModifierFlags, NSImage,
    NSResponder, NSView, NSWindow, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{NSNotification, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use snaplingo_ui::render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{permission_prompt::*, Key, Modifiers, Point};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
define_class!(
    #[unsafe(super(NSWindow))]
    #[name="SnapLingoNativePermissionPanel"]
    #[thread_kind=MainThreadOnly]
    #[ivars=()]
    struct Panel;
    unsafe impl NSObjectProtocol for Panel{}
    impl Panel{
        #[unsafe(method(canBecomeKeyWindow))]fn can_become_key(&self)->bool{true}
        #[unsafe(method(canBecomeMainWindow))]fn can_become_main(&self)->bool{false}
    }
);

static NEXT: AtomicU64 = AtomicU64::new(1);
thread_local! {static SHEET:RefCell<Option<Rc<Sheet>>>=const{RefCell::new(None)};}
struct CanvasState {
    owner: RefCell<Weak<Sheet>>,
    ui: RefCell<PermissionPromptUi>,
    renderer: RefCell<Renderer>,
    metrics: RefCell<RendererTextMetrics>,
    image: RefCell<Option<Retained<NSImage>>>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name="SnapLingoNativePermissionGuide"]
    #[thread_kind=MainThreadOnly]
    #[ivars=CanvasState]
    struct Canvas;
    unsafe impl NSObjectProtocol for Canvas{}
    unsafe impl NSWindowDelegate for Canvas{
        #[unsafe(method(windowDidBecomeKey:))]fn became_key(&self,_:&NSNotification){let owner=self.ivars().owner.borrow().upgrade();if let Some(owner)=owner{owner.refresh();}}
    }
    impl Canvas{
        #[unsafe(method(isFlipped))]fn flipped(&self)->bool{true}
        #[unsafe(method(acceptsFirstResponder))]fn accepts(&self)->bool{true}
        #[unsafe(method(viewDidChangeBackingProperties))]fn backing_changed(&self){if let Some(owner)=self.ivars().owner.borrow().upgrade(){owner.redraw();}}
        #[unsafe(method(drawRect:))]fn draw(&self,_:NSRect){if let Some(image)=self.ivars().image.borrow().as_ref(){unsafe{image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(self.bounds(),NSRect::ZERO,NSCompositingOperation::SourceOver,1.,true,None);}}}
        #[unsafe(method(mouseDown:))]fn mouse(&self,event:&NSEvent){let point=self.convertPoint_fromView(event.locationInWindow(),None);self.input(PermissionPromptInput::Pointer(Point{x:point.x as f32,y:point.y as f32}));}
        #[unsafe(method(keyDown:))]fn key(&self,event:&NSEvent){let flags=event.modifierFlags();let key=match event.keyCode(){53=>Some(Key::Escape),48=>Some(Key::Tab),36|76=>Some(Key::Enter),49=>Some(Key::Space),_=>None};if let Some(key)=key{self.input(PermissionPromptInput::Key(key,Modifiers{meta:flags.contains(NSEventModifierFlags::Command),control:flags.contains(NSEventModifierFlags::Control),alt:flags.contains(NSEventModifierFlags::Option),shift:flags.contains(NSEventModifierFlags::Shift)}));}}
    }
);
impl Canvas {
    fn input(&self, input: PermissionPromptInput) {
        let intent = self.ivars().ui.borrow_mut().update(input);
        let owner = self.ivars().owner.borrow().upgrade();
        if let Some(owner) = owner {
            if let Some(intent) = intent {
                owner.perform(intent);
            } else {
                owner.redraw();
            }
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
            nodes: self.nodes.iter().map(|(id, n)| (*id, n.clone())).collect(),
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
struct Actions(u64);
impl ActionHandler for Actions {
    fn do_action(&mut self, request: ActionRequest) {
        let epoch = self.0;
        crate::integrations::ui_thread::dispatch(move || {
            if request.target_tree != TreeId::ROOT {
                return;
            }
            let sheet = SHEET.with_borrow(Clone::clone);
            let Some(sheet) = sheet.filter(|s| s.id == epoch) else {
                return;
            };
            let target = match request.target_node.0.checked_sub(10 + epoch * 8) {
                Some(0) => PermissionPromptTarget::Later,
                Some(1) => PermissionPromptTarget::Authorize,
                Some(2) => PermissionPromptTarget::Help,
                _ => return,
            };
            let input = match request.action {
                Action::Click => PermissionPromptInput::Activate(target),
                Action::Focus => PermissionPromptInput::Focus(target),
                _ => return,
            };
            sheet.view.input(input);
        });
    }
}
struct Sheet {
    previous: Option<Retained<NSResponder>>,
    parent: Retained<NSWindow>,
    window: Retained<NSWindow>,
    view: Retained<Canvas>,
    session: Arc<PermissionPromptSession>,
    state: crate::features::settings::Services,
    open_settings: Rc<dyn Fn() -> Result<(), String>>,
    id: u64,
    zoom: f64,
    alive: Arc<AtomicBool>,
    adapter: RefCell<Option<SubclassingAdapter>>,
    semantics: Arc<Mutex<Option<Semantics>>>,
}
impl Drop for Sheet {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Release);
    }
}

fn model(
    state: &crate::features::settings::Services,
    projection: Projection,
) -> Result<PermissionPromptSnapshot, String> {
    let saved = state.configuration.snapshot().map_err(|e| e.to_string())?;
    let mut shell = snaplingo_ui::Snapshot::default();
    shell.theme = match saved.general.interface_theme.as_str() {
        "studio" => snaplingo_ui::Theme::Studio,
        "paper" => snaplingo_ui::Theme::Paper,
        _ => snaplingo_ui::Theme::Native,
    };
    shell.dark = crate::integrations::system::native_theme::is_dark(&saved.general.theme);
    Ok(PermissionPromptSnapshot {
        shell,
        needs_installation: projection.needs_installation,
        busy: projection.busy,
        attempted: projection.attempted,
        error: projection.error,
    })
}
pub(crate) fn refresh(
    parent: &NSWindow,
    state: crate::features::settings::Services,
    session: Arc<PermissionPromptSession>,
    open_settings: Rc<dyn Fn() -> Result<(), String>>,
) -> Result<(), String> {
    let projection = session.projection();
    if !projection.visible {
        close_for(parent);
        return Ok(());
    }
    let existing = SHEET.with_borrow(Clone::clone);
    if let Some(sheet) = existing.filter(|sheet| &*sheet.parent == parent) {
        sheet.refresh();
        return Ok(());
    }
    let mtm = MainThreadMarker::new().ok_or("Permission guide requires the UI thread")?;
    let saved = state.configuration.snapshot().map_err(|e| e.to_string())?;
    let zoom = snaplingo_core::capture::interface_scale(&saved.screenshot.capture_ui_scale, None);
    let mut ui = PermissionPromptUi::new(model(&state, projection)?);
    let mut metrics = RendererTextMetrics::default();
    let frame = ui.frame(1., &mut metrics);
    let size = frame.scene.size;
    let panel = Panel::alloc(mtm).set_ivars(());
    let window: Retained<Panel> = unsafe {
        msg_send![super(panel),initWithContentRect:NSRect::new(NSPoint::ZERO,NSSize::new(size.width as f64*zoom,size.height as f64*zoom)),styleMask:NSWindowStyleMask::Borderless,backing:NSBackingStoreType::Buffered,defer:false]
    };
    let window = window.into_super();
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    window.setTitle(&NSString::from_str(ui.snapshot().title()));
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setHasShadow(true);
    let view = Canvas::alloc(mtm).set_ivars(CanvasState {
        owner: RefCell::new(Weak::new()),
        ui: RefCell::new(ui),
        renderer: RefCell::new(Renderer::default()),
        metrics: RefCell::new(metrics),
        image: RefCell::new(None),
    });
    let view: Retained<Canvas> = unsafe {
        msg_send![super(view),initWithFrame:NSRect::new(NSPoint::ZERO,NSSize::new(size.width as f64,size.height as f64))]
    };
    window.setContentView(Some(&view));
    window.setDelegate(Some(ProtocolObject::from_ref(&*view)));
    view.setFrameSize(NSSize::new(
        size.width as f64 * zoom,
        size.height as f64 * zoom,
    ));
    view.setBoundsSize(NSSize::new(size.width as f64, size.height as f64));
    let sheet = Rc::new(Sheet {
        previous: parent.firstResponder(),
        parent: parent.retain(),
        window,
        view,
        session,
        state,
        open_settings,
        id: NEXT.fetch_add(1, Ordering::Relaxed),
        zoom,
        alive: Arc::new(AtomicBool::new(true)),
        adapter: RefCell::new(None),
        semantics: Arc::new(Mutex::new(None)),
    });
    *sheet.view.ivars().owner.borrow_mut() = Rc::downgrade(&sheet);
    *sheet.adapter.borrow_mut() = Some(unsafe {
        SubclassingAdapter::new(
            Retained::as_ptr(&sheet.view).cast_mut().cast(),
            Initial(sheet.semantics.clone()),
            Actions(sheet.id),
        )
    });
    SHEET.with_borrow_mut(|slot| *slot = Some(sheet.clone()));
    sheet.redraw();
    parent.beginSheet_completionHandler(&sheet.window, None);
    sheet.window.makeFirstResponder(Some(&sheet.view));
    let alive = sheet.alive.clone();
    let epoch = sheet.id;
    crate::integrations::runtime::spawn(async move {
        while alive.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(750)).await;
            if !alive.load(Ordering::Acquire) {
                break;
            }
            crate::integrations::ui_thread::dispatch(move || {
                let sheet = SHEET.with_borrow(Clone::clone);
                if let Some(sheet) = sheet.filter(|s| s.id == epoch) {
                    sheet.refresh();
                }
            });
        }
    });
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
        sheet.alive.store(false, Ordering::Release);
        sheet.window.setDelegate(None);
        sheet.parent.endSheet(&sheet.window);
        sheet.window.orderOut(None);
        sheet.adapter.borrow_mut().take();
        sheet.parent.makeFirstResponder(sheet.previous.as_deref());
    }
}
impl Sheet {
    fn refresh(&self) {
        let projection = self.session.projection();
        if !projection.visible {
            close_for(&self.parent);
            return;
        }
        match model(&self.state, projection) {
            Ok(model) => {
                self.view
                    .ivars()
                    .ui
                    .borrow_mut()
                    .update(PermissionPromptInput::Snapshot(model));
                self.redraw();
            }
            Err(error) => log::warn!("Failed to refresh permission guide: {error}"),
        }
    }
    fn perform(&self, intent: PermissionPromptIntent) {
        match intent {
            PermissionPromptIntent::Dismiss => {
                self.session.dismiss();
                close_for(&self.parent);
            }
            PermissionPromptIntent::OpenSettings => {
                self.session.dismiss();
                close_for(&self.parent);
                if let Err(e) = (self.open_settings)() {
                    log::warn!("Failed to open permission recovery: {e}");
                }
            }
            PermissionPromptIntent::Authorize => {
                if let Some(ticket) = self.session.begin_authorize() {
                    self.refresh();
                    let session = self.session.clone();
                    let id = self.id;
                    crate::integrations::runtime::spawn(async move {
                        session.authorize(ticket);
                        crate::integrations::ui_thread::dispatch(move || {
                            let sheet = SHEET.with_borrow(Clone::clone);
                            if let Some(sheet) = sheet.filter(|s| s.id == id) {
                                sheet.refresh();
                            }
                        });
                    });
                }
            }
        }
    }
    fn redraw(&self) {
        let frame = self
            .view
            .ivars()
            .ui
            .borrow_mut()
            .frame(1., &mut *self.view.ivars().metrics.borrow_mut());
        let size = frame.scene.size;
        let content = NSSize::new(
            size.width as f64 * self.zoom,
            size.height as f64 * self.zoom,
        );
        if self
            .window
            .contentView()
            .is_some_and(|v| v.frame().size != content)
        {
            self.window.setContentSize(content);
            self.view.setFrameSize(content);
            self.view
                .setBoundsSize(NSSize::new(size.width as f64, size.height as f64));
        }
        let scale = self.window.backingScaleFactor() * self.zoom;
        match self
            .view
            .ivars()
            .renderer
            .borrow_mut()
            .render(&frame.scene, scale as f32)
            .and_then(|pixels| crate::features::settings::macos::frame_image(&pixels))
        {
            Ok(image) => *self.view.ivars().image.borrow_mut() = Some(image),
            Err(error) => log::warn!("Failed to draw permission guide: {error}"),
        };
        self.view.setNeedsDisplay(true);
        let ui = self.view.ivars().ui.borrow();
        let snapshot = ui.snapshot();
        let mut root = Node::new(Role::Dialog);
        root.set_modal();
        root.set_label(snapshot.title());
        root.set_description(if snapshot.needs_installation {
            snapshot.description().into()
        } else {
            format!("{} {GRANT_EXPLANATION}", snapshot.description())
        });
        let mut nodes = BTreeMap::new();
        let mut children = vec![];
        let mut focus = NodeId(1);
        for c in frame.controls {
            let id = NodeId(10 + self.id * 8 + c.target as u64);
            let mut node = Node::new(Role::Button);
            node.set_label(c.label);
            node.set_bounds(accesskit::Rect::new(
                c.rect.x as f64 * scale,
                c.rect.y as f64 * scale,
                (c.rect.x + c.rect.width) as f64 * scale,
                (c.rect.y + c.rect.height) as f64 * scale,
            ));
            if c.enabled {
                node.add_action(Action::Click);
                node.add_action(Action::Focus);
            } else {
                node.set_disabled();
            }
            if ui.focused() == Some(c.target) {
                focus = id;
            }
            nodes.insert(id, node);
            children.push(id);
        }
        if let Some(error) = &snapshot.error {
            let mut node = Node::new(Role::Alert);
            node.set_label(error.clone());
            node.set_live(accesskit::Live::Assertive);
            nodes.insert(NodeId(2), node);
            children.push(NodeId(2));
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
#[cfg(test)]
pub(crate) fn test_is_open() -> bool {
    SHEET.with_borrow(Option::is_some)
}
#[cfg(test)]
pub(crate) fn test_input(input: PermissionPromptInput) -> bool {
    let sheet = SHEET.with_borrow(Clone::clone);
    if let Some(sheet) = sheet {
        sheet.view.input(input);
        true
    } else {
        false
    }
}
#[cfg(test)]
pub(crate) fn test_snapshot() -> Option<PermissionPromptSnapshot> {
    SHEET.with_borrow(|slot| {
        slot.as_ref()
            .map(|s| s.view.ivars().ui.borrow().snapshot().clone())
    })
}
