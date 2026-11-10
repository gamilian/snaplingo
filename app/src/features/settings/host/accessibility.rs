//! winit native adapter for the shared AccessKit semantic projection.
use crate::features::settings::host::HostEvent;
use accesskit::{ActionRequest, ActivationHandler, TreeUpdate};
use accesskit_winit::Adapter;
use snaplingo_ui::accessibility::{window_project, WindowProjection};
use snaplingo_ui::{WindowFrame, WindowInput, WindowUi};
use std::sync::{Arc, Mutex};
use winit::{
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::Window,
};
struct CachedTree(Arc<Mutex<WindowProjection>>);
impl ActivationHandler for CachedTree {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(self.0.lock().unwrap().update(None))
    }
}

/// Own this beside the native Window. Construct it before showing the window.
pub(crate) struct Accessibility {
    adapter: Adapter,
    current: Arc<Mutex<WindowProjection>>,
    sent: Option<WindowProjection>,
}
impl Accessibility {
    pub(crate) fn new(
        event_loop: &ActiveEventLoop,
        window: &Window,
        proxy: EventLoopProxy<HostEvent>,
        frame: &WindowFrame,
        ui: &WindowUi,
        device_scale: f64,
    ) -> Self {
        let current = Arc::new(Mutex::new(window_project(frame, ui, device_scale)));
        let adapter =
            Adapter::with_mixed_handlers(event_loop, window, CachedTree(current.clone()), proxy);
        Self {
            adapter,
            current,
            sent: None,
        }
    }
    /// AccessKit requires this before the application's handling of every event.
    pub(crate) fn process_event(&mut self, window: &Window, event: &WindowEvent) {
        self.adapter.process_event(window, event);
    }
    pub(crate) fn update(&mut self, frame: &WindowFrame, ui: &WindowUi, device_scale: f64) {
        let next = window_project(frame, ui, device_scale);
        *self.current.lock().unwrap() = next.clone();
        if self.sent.as_ref() == Some(&next) {
            return;
        }
        let previous = self.sent.as_ref();
        let mut delivered = false;
        self.adapter.update_if_active(|| {
            delivered = true;
            next.update(previous)
        });
        if delivered {
            self.sent = Some(next);
        }
    }
    pub(crate) fn action(&self, request: &ActionRequest) -> Vec<WindowInput> {
        self.current.lock().unwrap().actions(request)
    }
    pub(crate) fn deactivated(&mut self) {
        self.sent = None;
    }
}
