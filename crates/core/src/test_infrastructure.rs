//! Transitional fixtures: compile the real portable adapters in this test crate
//! so the pre-extraction unit tests retain one identity for their private types.
#[path = "../../app/src/infrastructure/events/mod.rs"]
pub mod events;
#[path = "../../storage/src/lib.rs"]
pub mod storage;
pub mod http {
    pub use crate::application::providers::{HttpClient, HttpResponse};
}
#[path = "../../app/src/infrastructure/llm/mod.rs"]
pub mod llm;

pub mod system {
    pub mod ocr {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../app/src/infrastructure/system/ocr/image_file.rs"
        ));
    }
}
