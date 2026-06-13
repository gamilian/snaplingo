mod trait_def;
mod coordinator;
pub mod impls;

#[cfg(test)]
mod coordinator_test;

pub use trait_def::OcrProvider;
pub use coordinator::OcrCoordinator;
