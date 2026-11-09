//! Result-window semantics and validated native accessibility actions.
use accesskit::{
    Action, ActionData, ActionRequest, Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate,
};
use snaplingo_ui::result::{ResultFrame, ResultInput, ResultRole, ResultTarget, ResultUi};
use std::collections::BTreeMap;

const ROOT: NodeId = NodeId(1);
const BODY: NodeId = NodeId(2);
#[derive(Clone, PartialEq)]
pub struct ResultProjection {
    nodes: BTreeMap<NodeId, Node>,
    targets: BTreeMap<NodeId, ResultTarget>,
    focus: NodeId,
}
impl ResultProjection {
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
    pub fn actions(&self, request: &ActionRequest) -> Vec<ResultInput> {
        if request.target_tree != TreeId::ROOT {
            return vec![];
        }
        let Some(node) = self.nodes.get(&request.target_node) else {
            return vec![];
        };
        if node.is_disabled() || !node.supports_action(request.action) {
            return vec![];
        }
        if request.target_node == BODY {
            return match request.action {
                Action::ScrollDown => vec![ResultInput::Scroll(250.)],
                Action::ScrollUp => vec![ResultInput::Scroll(-250.)],
                _ => vec![],
            };
        }
        let Some(target) = self.targets.get(&request.target_node) else {
            return vec![];
        };
        match request.action {
            Action::Click => vec![ResultInput::Activate(target.clone())],
            Action::Focus => vec![ResultInput::Focus(target.clone())],
            Action::SetValue if *target == ResultTarget::Source => {
                if let Some(ActionData::Value(value)) = &request.data {
                    vec![ResultInput::SetSource(value.clone().into())]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}
pub fn result_project(frame: &ResultFrame, ui: &ResultUi, device_scale: f64) -> ResultProjection {
    let mut nodes = BTreeMap::new();
    let mut targets = BTreeMap::new();
    let mut children = vec![];
    let mut focus = ROOT;
    for control in &frame.controls {
        let identity = format!("{}:{:?}", ui.snapshot().epoch, control.target);
        let hash = identity.bytes().fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        });
        let id = NodeId(hash | 0x8000000000000000);
        let role = match control.role {
            ResultRole::Button => Role::Button,
            ResultRole::MenuItem => Role::MenuItem,
            ResultRole::MultilineInput => Role::MultilineTextInput,
            ResultRole::Text => Role::MultilineTextInput,
        };
        let mut node = Node::new(role);
        node.set_label(control.label.clone());
        if control.target == ResultTarget::Pin {
            node.set_toggled(ui.pinned().into());
        }
        if let ResultTarget::ToggleRow(id) = &control.target {
            node.set_expanded(ui.row_expanded(id));
        }

        let rect = control.hit_rect;
        node.set_bounds(accesskit::Rect {
            x0: rect.x as f64 * device_scale,
            y0: rect.y as f64 * device_scale,
            x1: (rect.x + rect.width) as f64 * device_scale,
            y1: (rect.y + rect.height) as f64 * device_scale,
        });
        if let Some(value) = &control.value {
            node.set_value(value.clone());
        }
        if control.role == ResultRole::Text {
            node.set_read_only();
        }
        if control.enabled {
            node.add_action(Action::Focus);
            if matches!(control.role, ResultRole::Button | ResultRole::MenuItem) {
                node.add_action(Action::Click);
            }
            if control.target == ResultTarget::Source {
                node.add_action(Action::SetValue);
            }
        } else {
            node.set_disabled();
        }
        if ui.focused() == Some(&control.target) {
            focus = id;
        }
        nodes.insert(id, node);
        targets.insert(id, control.target.clone());
        children.push(id);
    }
    let mut body = Node::new(Role::ScrollView);
    body.set_children(children);
    body.add_action(Action::ScrollUp);
    body.add_action(Action::ScrollDown);
    nodes.insert(BODY, body);
    let mut root = Node::new(Role::Window);
    root.set_label("SnapLingo 结果窗口");
    root.set_children([BODY]);
    nodes.insert(ROOT, root);
    ResultProjection {
        nodes,
        targets,
        focus,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snaplingo_ui::result::ResultSnapshot;
    use snaplingo_ui::{FontFamily, Size, TextMeasurement, TextMeasurer};
    struct Metrics;
    impl TextMeasurer for Metrics {
        fn measure(&mut self, _: &str, _: f32, _: u16, _: FontFamily) -> TextMeasurement {
            TextMeasurement::default()
        }
    }
    #[test]
    fn source_set_value_is_typed_and_previous_epoch_actions_are_rejected() {
        let mut ui = ResultUi::new(ResultSnapshot {
            source: "hello".into(),
            epoch: 4,
            ..ResultSnapshot::default()
        });
        let frame = ui.frame(
            Size {
                width: 660.,
                height: 700.,
            },
            1.,
            &mut Metrics,
        );
        let projection = result_project(&frame, &ui, 2.);
        let id = *projection
            .targets
            .iter()
            .find(|(_, target)| **target == ResultTarget::Source)
            .unwrap()
            .0;
        let request = ActionRequest {
            action: Action::SetValue,
            target_tree: TreeId::ROOT,
            target_node: id,
            data: Some(ActionData::Value("新的原文".into())),
        };
        assert!(
            matches!(projection.actions(&request).as_slice(),[ResultInput::SetSource(text)] if text=="新的原文")
        );
        ui.replace(ResultSnapshot {
            source: "next".into(),
            epoch: 5,
            revision: 1,
            ..ResultSnapshot::default()
        });
        let frame = ui.frame(
            Size {
                width: 660.,
                height: 700.,
            },
            1.,
            &mut Metrics,
        );
        assert!(result_project(&frame, &ui, 2.).actions(&request).is_empty());
    }
}
