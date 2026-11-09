use std::sync::{Arc, Mutex};

use image::ImageEncoder;

use crate::application::capture::{CaptureImageComposer, CaptureOutput};

use super::{PinnedImageRuntime, PinnedImageRuntimeHost, PinnedImageState};
use crate::domain::capture::PinnedImageView;

#[derive(Clone, Debug, PartialEq)]
enum HostCall {
    Open(String),
    ShowOrOpen(String),
    Hide(String),
    ToggleAll,
    ApplyGroupSwitch {
        hide_image_ids: Vec<String>,
        show_image_ids: Vec<String>,
    },
    HideGroup(Vec<String>),
    CloseGroup(Vec<String>),
}

struct RecordingPinnedImageHost {
    calls: Arc<Mutex<Vec<HostCall>>>,
    open_error: Option<String>,
    placements: Mutex<Vec<super::PinnedImagePlacement>>,
}

impl RecordingPinnedImageHost {
    fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            open_error: None,
            placements: Mutex::new(Vec::new()),
        }
    }

    fn with_open_error(message: &str) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            open_error: Some(message.to_string()),
            placements: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<HostCall> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl PinnedImageRuntimeHost for RecordingPinnedImageHost {
    async fn open(
        &self,
        image: PinnedImageView,
        placement: super::PinnedImagePlacement,
    ) -> crate::Result<()> {
        self.placements.lock().unwrap().push(placement);
        self.calls.lock().unwrap().push(HostCall::Open(image.id));
        match &self.open_error {
            Some(error) => Err(crate::AppError::Other(error.clone())),
            None => Ok(()),
        }
    }

    async fn show_or_open(&self, image: PinnedImageView) -> crate::Result<()> {
        self.calls
            .lock()
            .unwrap()
            .push(HostCall::ShowOrOpen(image.id));
        Ok(())
    }

    async fn hide(&self, image_id: String) -> crate::Result<()> {
        self.calls.lock().unwrap().push(HostCall::Hide(image_id));
        Ok(())
    }

    async fn toggle_all(&self) -> crate::Result<Option<bool>> {
        self.calls.lock().unwrap().push(HostCall::ToggleAll);
        Ok(Some(false))
    }

    async fn apply_group_switch(
        &self,
        hide_image_ids: Vec<String>,
        show_image_ids: Vec<String>,
    ) -> crate::Result<()> {
        self.calls.lock().unwrap().push(HostCall::ApplyGroupSwitch {
            hide_image_ids,
            show_image_ids,
        });
        Ok(())
    }

    async fn hide_group(&self, image_ids: Vec<String>) -> crate::Result<()> {
        self.calls
            .lock()
            .unwrap()
            .push(HostCall::HideGroup(image_ids));
        Ok(())
    }

    async fn close_group(&self, image_ids: Vec<String>) -> crate::Result<()> {
        self.calls
            .lock()
            .unwrap()
            .push(HostCall::CloseGroup(image_ids));
        Ok(())
    }
}

fn make_runtime() -> (
    PinnedImageRuntime,
    Arc<PinnedImageState>,
    Arc<RecordingPinnedImageHost>,
) {
    let state = Arc::new(PinnedImageState::new());
    let host = Arc::new(RecordingPinnedImageHost::new());
    let runtime = PinnedImageRuntime::new(
        state.clone(),
        Arc::new(CaptureImageComposer::new()),
        Arc::new(CaptureOutput::new()),
        host.clone(),
    );
    (runtime, state, host)
}

fn make_test_png(width: u32, height: u32) -> Vec<u8> {
    let pixels = vec![255; (width * height * 4) as usize];
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&pixels, width, height, image::ExtendedColorType::Rgba8)
        .unwrap();
    png
}

fn opened_image_id(host: &RecordingPinnedImageHost, index: usize) -> String {
    match &host.calls()[index] {
        HostCall::Open(image_id) => image_id.clone(),
        call => panic!("expected open call, got {call:?}"),
    }
}

#[tokio::test]
async fn pin_png_stores_image_before_opening_window() {
    let (runtime, state, host) = make_runtime();

    runtime.pin_png_and_open(make_test_png(3, 2)).await.unwrap();

    let image_id = opened_image_id(&host, 0);
    assert_eq!(state.get_pinned_image(&image_id).unwrap().width, 3);
}

#[tokio::test]
async fn capture_pins_use_the_latest_position_setting_and_preserve_the_source_coordinate_system() {
    use crate::application::capture::CapturePinOutput;
    use crate::application::settings::SettingsConfiguration;
    use crate::domain::capture::{DesktopPosition, LogicalPoint, PhysicalPoint};
    use crate::domain::ScreenshotSettings;
    use crate::infrastructure::storage::SqliteConfigStore;

    let (runtime, _, host) = make_runtime();
    let settings = Arc::new(SettingsConfiguration::new(Arc::new(
        SqliteConfigStore::new_in_memory(),
    )));
    let runtime = runtime.with_settings(settings.clone());
    for (position, centered) in [
        (
            Some(DesktopPosition::Logical(LogicalPoint {
                x: -240.0,
                y: 30.0,
            })),
            false,
        ),
        (
            Some(DesktopPosition::Physical(PhysicalPoint { x: -360, y: 45 })),
            true,
        ),
        (None, true),
    ] {
        settings
            .update_screenshot(ScreenshotSettings {
                pin_position: if centered { "center" } else { "original" }.into(),
                ..ScreenshotSettings::default()
            })
            .unwrap();
        CapturePinOutput::pin_png(&runtime, make_test_png(3, 2), position.clone())
            .await
            .unwrap();
        assert_eq!(
            host.placements.lock().unwrap().last(),
            Some(&super::PinnedImagePlacement {
                origin: position,
                centered
            })
        );
    }
}

#[tokio::test]
async fn pin_png_keeps_state_when_window_open_fails() {
    let state = Arc::new(PinnedImageState::new());
    let host = Arc::new(RecordingPinnedImageHost::with_open_error("open failed"));
    let runtime = PinnedImageRuntime::new(
        state.clone(),
        Arc::new(CaptureImageComposer::new()),
        Arc::new(CaptureOutput::new()),
        host.clone(),
    );

    let error =
        crate::application::capture::CapturePinOutput::pin_png(&runtime, make_test_png(3, 2), None)
            .await
            .unwrap_err();

    let image_id = opened_image_id(&host, 0);
    assert!(error.to_string().contains("open failed"));
    assert!(state.get_pinned_image(&image_id).is_ok());
}

#[tokio::test]
async fn clipboard_pin_reopens_the_most_recently_closed_image() {
    let (runtime, _service, host) = make_runtime();
    runtime.pin_png_and_open(make_test_png(2, 2)).await.unwrap();
    let image_id = opened_image_id(&host, 0);

    runtime.close(&image_id).await.unwrap();
    runtime.pin_clipboard().await.unwrap();

    assert_eq!(
        host.calls(),
        vec![
            HostCall::Open(image_id.clone()),
            HostCall::Hide(image_id.clone()),
            HostCall::ShowOrOpen(image_id),
        ]
    );
}

#[tokio::test]
async fn switch_group_applies_service_transition_to_window_host() {
    let (runtime, _service, host) = make_runtime();
    runtime.pin_png_and_open(make_test_png(1, 1)).await.unwrap();
    runtime.pin_png_and_open(make_test_png(1, 1)).await.unwrap();
    let first_id = opened_image_id(&host, 0);
    let second_id = opened_image_id(&host, 1);
    runtime.move_to_next_group(&first_id).await.unwrap();

    let next_group = runtime.switch_group().await.unwrap();

    assert_eq!(next_group, Some(1));
    assert_eq!(
        host.calls().last(),
        Some(&HostCall::ApplyGroupSwitch {
            hide_image_ids: vec![second_id],
            show_image_ids: vec![first_id],
        })
    );
}

#[tokio::test]
async fn destroy_group_removes_state_before_closing_windows() {
    let (runtime, state, host) = make_runtime();
    runtime.pin_png_and_open(make_test_png(1, 1)).await.unwrap();
    let image_id = opened_image_id(&host, 0);

    let removed = runtime.destroy_group(&image_id).await.unwrap();

    assert_eq!(removed, vec![image_id.clone()]);
    assert!(state.get_pinned_image(&image_id).is_err());
    assert_eq!(
        host.calls().last(),
        Some(&HostCall::CloseGroup(vec![image_id]))
    );
}

#[tokio::test]
async fn toggle_visibility_delegates_to_host() {
    let (runtime, _service, host) = make_runtime();

    assert_eq!(runtime.toggle_visibility().await.unwrap(), Some(false));
    assert_eq!(host.calls(), vec![HostCall::ToggleAll]);
}

#[tokio::test]
async fn closes_all_groups_and_forgets_recoverable_pins() {
    let (runtime, state, host) = make_runtime();
    let first = state.pin_png(make_test_png(100, 80)).unwrap();
    let second = state.pin_png(make_test_png(100, 80)).unwrap();
    state.move_pinned_image_to_group(&second, 2).unwrap();
    state.close_pinned_image(&first).unwrap();
    let ids = runtime.close_all().await.unwrap();
    assert_eq!(ids.len(), 2);
    assert!(state.get_pinned_image(&first).is_err());
    assert!(state.get_pinned_image(&second).is_err());
    assert!(state.pop_recoverable_pinned_image().is_none());
    assert_eq!(host.calls(), vec![HostCall::CloseGroup(ids)]);
}

#[tokio::test]
async fn saves_resampled_content_without_altering_the_original() {
    let (runtime, state, _) = make_runtime();
    let id = state.pin_png(make_test_png(100, 80)).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pin.webp");
    runtime
        .save_at_size(
            &id,
            &path,
            Some(crate::domain::capture::PinnedImageExportSize {
                width: 50,
                height: 40,
            }),
            false,
        )
        .await
        .unwrap();
    let image = image::open(path).unwrap();
    assert_eq!((image.width(), image.height()), (50, 40));
    let original = state.get_pinned_image(&id).unwrap();
    assert_eq!((original.width, original.height), (100, 80));
}
