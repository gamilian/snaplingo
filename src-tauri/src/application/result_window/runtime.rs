use std::sync::Arc;

use super::{
    ResultWindowClipboardPort, ResultWindowNotifierPort, ResultWindowOpenRequest,
    ResultWindowPayload, ResultWindowWindowPort,
};

#[allow(dead_code)]
pub(crate) struct ResultWindowRuntime;

#[allow(dead_code)]
impl ResultWindowRuntime {
    pub(crate) fn new(
        _window: Arc<dyn ResultWindowWindowPort>,
        _clipboard: Arc<dyn ResultWindowClipboardPort>,
        _notifier: Arc<dyn ResultWindowNotifierPort>,
    ) -> Self {
        unimplemented!("ResultWindowRuntime is introduced by the next task")
    }

    pub(crate) async fn open(&self, _request: ResultWindowOpenRequest) -> crate::Result<()> {
        unimplemented!("ResultWindowRuntime is introduced by the next task")
    }

    pub(crate) fn take(&self) -> crate::Result<Option<ResultWindowPayload>> {
        unimplemented!("ResultWindowRuntime is introduced by the next task")
    }
}
