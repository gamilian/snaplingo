mod trait_def;
mod registry;

#[cfg(test)]
mod registry_test;

pub use trait_def::TranslationProvider;
pub use registry::TranslationRegistry;

// Future modules (to be implemented in subsequent tasks)
// mod service;
// mod impls;

// pub use service::TranslationService;
