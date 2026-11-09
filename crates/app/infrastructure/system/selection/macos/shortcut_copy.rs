use std::time::Duration;

use async_trait::async_trait;

use crate::infrastructure::system::selection::common::shortcut_copy::wait_for_shortcut_modifiers_to_clear_with;
use snaplingo_core::application::selected_text::SelectionMethod;
use snaplingo_core::domain::{
    MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind, SelectionSource,
};

const MODIFIER_RELEASE_TIMEOUT: Duration = Duration::from_millis(1500);
const MODIFIER_POLL_INTERVAL: Duration = Duration::from_millis(15);

pub struct ShortcutCopySelectionMethod;

#[async_trait]
impl SelectionMethod for ShortcutCopySelectionMethod {
    fn kind(&self) -> SelectionMethodKind {
        SelectionMethodKind::ShortcutCopy
    }

    fn availability(&self, _context: &SelectionContext) -> MethodAvailability {
        if super::context::accessibility_permission_granted(false) {
            MethodAvailability::Available
        } else {
            MethodAvailability::Unavailable(
                super::context::selection_accessibility_permission_error(),
            )
        }
    }

    async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
        let result = async {
            wait_for_shortcut_modifiers_to_clear().await?;
            super::pasteboard::with_temporary_pasteboard_text(press_copy_shortcut_on_main_thread)
                .await
        }
        .await;

        match result {
            Ok(text) => SelectionAttempt::success(
                self.kind(),
                SelectionSource::ShortcutCopy,
                text,
                context.clone(),
            ),
            Err(err) => SelectionAttempt::failed(self.kind(), context.clone(), err),
        }
    }
}

async fn wait_for_shortcut_modifiers_to_clear() -> Result<(), String> {
    let waited = wait_for_shortcut_modifiers_to_clear_with(
        shortcut_modifier_keys_pressed,
        MODIFIER_RELEASE_TIMEOUT,
        MODIFIER_POLL_INTERVAL,
    )
    .await?;

    if waited >= Duration::from_millis(50) {
        log::info!(
            "Selection copy waited {}ms for shortcut modifiers to clear",
            waited.as_millis()
        );
    }

    Ok(())
}

fn shortcut_modifier_keys_pressed() -> Result<bool, String> {
    use objc2_app_kit::{NSEvent, NSEventModifierFlags};

    let flags = NSEvent::modifierFlags_class();
    let shortcut_modifiers = NSEventModifierFlags::Shift
        | NSEventModifierFlags::Control
        | NSEventModifierFlags::Option
        | NSEventModifierFlags::Command;

    Ok(flags.intersects(shortcut_modifiers))
}

async fn press_copy_shortcut_on_main_thread() -> Result<(), String> {
    crate::infrastructure::ui_thread::run("selection copy shortcut", press_copy_shortcut)
        .await
        .map_err(|error| error.to_string())
}

fn press_copy_shortcut() -> Result<(), String> {
    use enigo::{Enigo, Key, KeyboardControllable};

    let mut enigo = Enigo::new();
    enigo.key_down(Key::Meta);
    enigo.key_click(Key::Layout('c'));
    enigo.key_up(Key::Meta);

    Ok(())
}
