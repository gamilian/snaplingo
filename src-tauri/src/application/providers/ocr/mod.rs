mod configuration;
mod coordinator;
pub mod impls;
mod trait_def;

#[cfg(test)]
mod coordinator_test;

pub use configuration::OcrProviderConfiguration;
pub use coordinator::OcrCoordinator;
pub use trait_def::OcrProvider;
