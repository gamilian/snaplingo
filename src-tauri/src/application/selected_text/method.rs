use async_trait::async_trait;

use crate::domain::{MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind};

#[async_trait]
pub trait SelectionMethod: Send + Sync {
    fn kind(&self) -> SelectionMethodKind;
    fn availability(&self, context: &SelectionContext) -> MethodAvailability;
    async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt;
}

pub trait SelectionContextProvider: Send + Sync {
    fn context(&self) -> SelectionContext;
}

pub trait SystemSelectionProvider: SelectionContextProvider {
    fn default_scheme(&self) -> Vec<SelectionMethodKind>;
    fn methods(&self) -> Vec<Box<dyn SelectionMethod>>;
}
