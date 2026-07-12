pub mod configuration;
mod policy;
pub mod runtime;
mod store;

pub use configuration::HotkeyConfiguration;
pub(crate) use policy::{display_hotkey_to_accelerator, should_register_hotkey_on_release};
pub(crate) use runtime::{HotkeyRegistrar, HotkeyRegistration, HotkeyTriggerTiming};
pub use runtime::{HotkeyRuntime, HotkeyUpdateOutcome};
pub(crate) use store::HotkeyStore;
