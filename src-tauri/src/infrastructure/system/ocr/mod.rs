mod backend;
#[cfg(target_os = "macos")]
mod macos;

use std::sync::Arc;

pub use backend::SystemOcrEngine;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use macos::vision_languages_for_request;

pub fn get_system_ocr_engine() -> Arc<dyn SystemOcrEngine> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::MacOSVisionOcrEngine::new())
    }
}
