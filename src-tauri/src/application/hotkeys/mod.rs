pub mod configuration;
mod policy;
mod recording;
pub mod runtime;
mod store;

pub use configuration::HotkeyConfiguration;
pub(crate) use policy::{display_hotkey_to_accelerator, should_register_hotkey_on_release};
pub use recording::HotkeyRecording;
pub use runtime::{HotkeyChangeNotifier, HotkeyRuntime, HotkeyUpdateOutcome};
pub(crate) use runtime::{HotkeyRegistrar, HotkeyRegistration, HotkeyTriggerTiming};
pub(crate) use store::HotkeyStore;
