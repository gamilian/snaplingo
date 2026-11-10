mod tokens;
pub(crate) mod view;
use objc2_foundation::NSUserDefaults;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::integrations::ui_thread;

use self::view as native;
use super::controller::{ResultWindowController, ResultWindowEffects, Services};
use snaplingo_core::result::ResultWindowNotifierPort;
use snaplingo_core::result::ResultWindowPresentationPort;
use snaplingo_core::result::ResultWindowRequestId;
use snaplingo_core::result::ResultWindowSnapshot;
use snaplingo_core::result::ResultWindowWindowPort;

pub(crate) struct NativeEffects;

#[async_trait::async_trait]
impl ResultWindowEffects for NativeEffects {
    async fn pick_image(&self) -> crate::Result<Option<PathBuf>> {
        crate::integrations::system::dialogs::select_native_result_image_file().await
    }

    fn copy_text(&self, text: String) -> crate::Result<()> {
        snaplingo_platform::clipboard_text::copy_text(text).map_err(Into::into)
    }
}

#[derive(Default)]
pub(crate) struct NativeResultWindowHost {
    controller: RwLock<Option<ResultWindowController>>,
}

impl NativeResultWindowHost {
    pub(crate) fn bind(&self, services: Services) {
        *self.controller.write().unwrap() = Some(ResultWindowController::new(services));
    }

    fn controller(&self) -> Result<ResultWindowController, String> {
        self.controller
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| "Native result window services are not initialized".into())
    }

    pub(crate) fn settings_changed(&self) {
        let Ok(controller) = self.controller() else {
            return;
        };
        ui_thread::dispatch(move || controller.settings_changed());
    }

    #[cfg(test)]
    pub(super) fn show_on_main_thread(&self) -> Result<(), String> {
        show(self.controller()?)
    }

    #[cfg(test)]
    pub(super) fn receive_on_main_thread(
        &self,
        request_id: ResultWindowRequestId,
    ) -> Result<(), String> {
        self.controller()?.receive_payload(request_id)
    }
}

#[async_trait::async_trait]
impl ResultWindowWindowPort for NativeResultWindowHost {
    async fn show_or_create(&self) -> crate::Result<()> {
        let controller = self.controller()?;
        ui_thread::run("show result window", move || show(controller)).await
    }
}

#[async_trait::async_trait]
impl ResultWindowNotifierPort for NativeResultWindowHost {
    async fn notify_payload_ready(&self, request_id: ResultWindowRequestId) -> crate::Result<()> {
        let controller = self.controller()?;
        ui_thread::run("notify result window payload readiness", move || {
            controller.receive_payload(request_id)
        })
        .await
    }
}

pub(crate) struct NativeResultWindowPresentation;

impl ResultWindowPresentationPort for NativeResultWindowPresentation {
    fn publish(&self, view: ResultWindowSnapshot) {
        if objc2::MainThreadMarker::new().is_some() {
            native::update(view);
        } else {
            ui_thread::dispatch(move || native::update(view));
        }
    }
}

fn show(controller: ResultWindowController) -> Result<(), String> {
    native::open(
        controller.snapshot(),
        NSUserDefaults::standardUserDefaults(),
        move |action| controller.dispatch(action),
    )
}
