pub mod configuration;
pub mod runtime;
mod store;

pub use configuration::HotkeyConfiguration;
pub(crate) use runtime::{HotkeyRegistrar, HotkeyRegistration, HotkeyTriggerTiming};
pub use runtime::{HotkeyRuntime, HotkeyUpdateOutcome};
pub(crate) use store::HotkeyStore;
