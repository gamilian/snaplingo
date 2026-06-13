mod trait_def;
mod registry;
mod service;

#[cfg(test)]
mod registry_test;

#[cfg(test)]
mod service_test;

pub use trait_def::TranslationProvider;
pub use registry::TranslationRegistry;
pub use service::TranslationService;

// Future modules (to be implemented in subsequent tasks)
// mod impls;
