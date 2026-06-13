mod trait_def;
mod registry;
mod service;

#[cfg(test)]
mod registry_test;

#[cfg(test)]
mod service_test;

pub use trait_def::OcrProvider;
pub use registry::OcrRegistry;
pub use service::OcrService;
