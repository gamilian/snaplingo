//! Shared modal capacity semantics for native hosts.
use accesskit::{
    Action, ActionData, ActionRequest, Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate,
};
use snaplingo_ui::{capacity::*, Key, Modifiers};
use std::collections::BTreeMap;
const ROOT: NodeId = NodeId(1);
#[derive(Clone, PartialEq)]
pub struct CapacityProjection {
    nodes: BTreeMap<NodeId, Node>,
    targets: BTreeMap<NodeId, CapacityTarget>,
    focus: NodeId,
}
impl CapacityProjection {
    pub fn update(&self, previous: Option<&Self>) -> TreeUpdate {
        TreeUpdate {
            nodes: self
                .nodes
                .iter()
                .filter(|(id, node)| previous.is_none_or(|old| old.nodes.get(id) != Some(node)))
                .map(|(id, node)| (*id, node.clone()))
                .collect(),
            tree: previous.is_none().then(|| TreeInfo::new(ROOT)),
            tree_id: TreeId::ROOT,
            focus: self.focus,
        }
    }
    pub fn actions(&self, request: &ActionRequest) -> Vec<CapacityInput> {
        if request.target_tree != TreeId::ROOT {
            return vec![];
        }
        let Some(node) = self.nodes.get(&request.target_node) else {
            return vec![];
        };
        if !node.supports_action(request.action) {
            return vec![];
        }
        let Some(target) = self.targets.get(&request.target_node).copied() else {
            return vec![];
        };
        match request.action {
            Action::Click => vec![CapacityInput::Activate(target)],
            Action::Focus => vec![CapacityInput::Focus(target)],
            Action::SetValue => match &request.data {
                Some(ActionData::Value(value)) => {
                    vec![CapacityInput::SetValue(target, value.clone().into())]
                }
                Some(ActionData::NumericValue(value)) if value.is_finite() => {
                    vec![CapacityInput::SetValue(target, format!("{value:.0}"))]
                }
                _ => vec![],
            },
            Action::Increment | Action::Decrement => vec![
                CapacityInput::Focus(target),
                CapacityInput::Key(
                    if request.action == Action::Increment {
                        Key::ArrowUp
                    } else {
                        Key::ArrowDown
                    },
                    Modifiers::default(),
                ),
            ],
            _ => vec![],
        }
    }
}
pub fn capacity_project(
    frame: &CapacityFrame,
    ui: &CapacityUi,
    device_scale: f64,
    epoch: u64,
) -> CapacityProjection {
    let mut nodes = BTreeMap::new();
    let mut targets = BTreeMap::new();
    let mut children = vec![];
    let mut focus = ROOT;
    for control in &frame.controls {
        let id = NodeId(10 + epoch.saturating_mul(16) + control.target as u64);
        let target = control.target;
        let mut node = Node::new(if target == CapacityTarget::Cleanup {
            Role::Switch
        } else if target.range().is_some() {
            Role::SpinButton
        } else {
            Role::Button
        });
        node.set_label(target.label());
        let r = control.rect;
        node.set_bounds(accesskit::Rect::new(
            r.x as f64 * device_scale,
            r.y as f64 * device_scale,
            (r.x + r.width) as f64 * device_scale,
            (r.y + r.height) as f64 * device_scale,
        ));
        node.add_action(Action::Click);
        node.add_action(Action::Focus);
        if let Some(value) = control.checked {
            node.set_toggled(value.into());
        }
        if let Some(value) = &control.value {
            node.set_value(value);
            if let Ok(value) = value.parse::<f64>() {
                node.set_numeric_value(value);
            }
        }
        if let Some((min, max, step)) = target.range() {
            node.set_min_numeric_value(min as f64);
            node.set_max_numeric_value(max as f64);
            node.set_numeric_value_step(step as f64);
            node.add_action(Action::SetValue);
            node.add_action(Action::Increment);
            node.add_action(Action::Decrement);
        }
        if ui.focused() == Some(target) {
            focus = id;
        }
        nodes.insert(id, node);
        targets.insert(id, target);
        children.push(id);
    }
    let mut root = Node::new(Role::Dialog);
    root.set_label("历史与收藏");
    root.set_modal();
    root.set_children(children);
    nodes.insert(ROOT, root);
    CapacityProjection {
        nodes,
        targets,
        focus,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use snaplingo_ui::{FontFamily, Snapshot, TextMeasurement, TextMeasurer};
    struct Metrics;
    impl TextMeasurer for Metrics {
        fn measure(&mut self, _: &str, _: f32, _: u16, _: FontFamily) -> TextMeasurement {
            Default::default()
        }
    }
    #[test]
    fn numeric_actions_respect_modal_epoch_and_validate_through_the_draft() {
        let mut ui = CapacityUi::new(
            Snapshot::default(),
            CapacityValue {
                automatic_cleanup: true,
                retention_days: 30,
                history_limit: 5000,
                favorites_limit: 1000,
            },
        );
        let frame = ui.frame(1.75, &mut Metrics);
        let projection = capacity_project(&frame, &ui, 2., 3);
        let id = *projection
            .targets
            .iter()
            .find(|(_, target)| **target == CapacityTarget::Days)
            .unwrap()
            .0;
        let request = ActionRequest {
            action: Action::SetValue,
            target_tree: TreeId::ROOT,
            target_node: id,
            data: Some(ActionData::NumericValue(999.)),
        };
        for input in projection.actions(&request) {
            ui.update(input);
        }
        assert!(ui
            .update(CapacityInput::Activate(CapacityTarget::Save))
            .is_none());
        assert!(capacity_project(&frame, &ui, 2., 4)
            .actions(&request)
            .is_empty());
    }
}
