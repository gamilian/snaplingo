use std::collections::HashMap;

use crate::domain::SelectionMethodKind;

use super::backend::SelectionMethod;

pub struct SelectionMethodRegistry {
    methods: HashMap<SelectionMethodKind, Box<dyn SelectionMethod>>,
}

impl SelectionMethodRegistry {
    pub fn new(methods: Vec<Box<dyn SelectionMethod>>) -> Self {
        let methods = methods
            .into_iter()
            .map(|method| (method.kind(), method))
            .collect();
        Self { methods }
    }

    pub fn get(&self, kind: SelectionMethodKind) -> Option<&dyn SelectionMethod> {
        self.methods.get(&kind).map(Box::as_ref)
    }
}

#[cfg(test)]
mod selection_registry_tests {
    use super::*;
    use crate::domain::{
        MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind,
    };
    use async_trait::async_trait;

    struct FakeMethod(SelectionMethodKind);

    #[async_trait]
    impl SelectionMethod for FakeMethod {
        fn kind(&self) -> SelectionMethodKind {
            self.0
        }

        fn availability(&self, _context: &SelectionContext) -> MethodAvailability {
            MethodAvailability::Available
        }

        async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
            SelectionAttempt::empty(self.0, context.clone())
        }
    }

    #[test]
    fn registry_returns_methods_by_kind() {
        let registry = SelectionMethodRegistry::new(vec![Box::new(FakeMethod(
            SelectionMethodKind::Accessibility,
        ))]);

        assert!(registry.get(SelectionMethodKind::Accessibility).is_some());
        assert!(registry.get(SelectionMethodKind::MenuCopy).is_none());
    }
}
