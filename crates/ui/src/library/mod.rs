//! Portable Library list/detail state and drawing. Hosts supply typed records
//! and execute intents; this module has no database or platform dependencies.
mod draw;
mod model;
mod state;

pub use model::*;
pub use state::LibraryUi;

#[cfg(test)]
mod tests;
