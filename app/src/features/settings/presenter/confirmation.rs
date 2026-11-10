//! Confirmation owns the original action until the current Settings interaction
//! accepts it. Native adapters display the request; they never classify actions.
use snaplingo_ui::{library::LibraryIntent, services::ServicesIntent, Action, Page, WindowIntent};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_CONFIRMATION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct Confirmation {
    id: u64,
    pub message: &'static str,
}

pub enum PreparedAction {
    Execute(Page, WindowIntent),
    Confirm(Confirmation),
}

/// Shared by the product session and the isolated preferences preview.
#[derive(Default)]
pub struct Confirmations {
    pending: Option<(u64, Page, WindowIntent)>,
}
impl Confirmations {
    pub fn prepare(&mut self, page: Page, intent: WindowIntent) -> PreparedAction {
        self.cancel();
        let message = match &intent {
            WindowIntent::Settings(Action::ResetHotkeys) => {
                Some("确定要恢复当前分类的默认快捷键吗？")
            }
            WindowIntent::Settings(Action::GeneralCommand(command)) => {
                super::general::confirmation(*command)
            }
            WindowIntent::Services(ServicesIntent::ConfirmRemove(_)) => {
                Some("确定要删除这个自定义翻译服务吗？")
            }
            WindowIntent::Library(LibraryIntent::Delete(_)) => Some("确定要删除这条记录吗？"),
            WindowIntent::Library(LibraryIntent::ConfirmClearHistory(_)) => {
                Some("确定要清空当前类型的全部历史记录吗？独立收藏不会被删除。")
            }
            _ => None,
        };
        if let Some(message) = message {
            let id = NEXT_CONFIRMATION.fetch_add(1, Ordering::Relaxed);
            self.pending = Some((id, page, intent));
            PreparedAction::Confirm(Confirmation { id, message })
        } else {
            PreparedAction::Execute(page, intent)
        }
    }

    pub fn resolve(
        &mut self,
        request: Confirmation,
        accepted: bool,
    ) -> Option<(Page, WindowIntent)> {
        if self.pending.as_ref().map(|(id, _, _)| *id) != Some(request.id) {
            return None;
        }
        let (_, page, intent) = self.pending.take().unwrap();
        accepted.then_some((page, intent))
    }

    pub fn cancel(&mut self) {
        self.pending = None;
    }
}
