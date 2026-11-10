//! Shared Services lists and editors. Hosts project real provider workflows and
//! execute intents; this module owns only drawing and local interaction.
mod draw;
mod model;
mod state;
pub use model::*;
pub use state::ServicesUi;
#[cfg(test)]
mod tests;
