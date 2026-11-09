use accesskit::{
    Action, ActionData, ActionRequest, Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate,
};
use snaplingo_ui::notices::{NoticesFrame, NoticesInput, NoticesTarget, NoticesUi};
use std::collections::BTreeMap;
const ROOT: NodeId = NodeId(1);
const BODY: NodeId = NodeId(2);
#[derive(Clone, PartialEq)]
pub struct NoticesProjection {
    nodes: BTreeMap<NodeId, Node>,
    targets: BTreeMap<NodeId, NoticesTarget>,
    focus: NodeId,
}
impl NoticesProjection {
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
    pub fn actions(&self, request: &ActionRequest) -> Vec<NoticesInput> {
        if request.target_tree != TreeId::ROOT {
            return vec![];
        }
        let Some(node) = self.nodes.get(&request.target_node) else {
            return vec![];
        };
        if !node.supports_action(request.action) {
            return vec![];
        }
        if request.target_node == BODY {
            return match request.action {
                Action::ScrollDown => vec![NoticesInput::Scroll(250.)],
                Action::ScrollUp => vec![NoticesInput::Scroll(-250.)],
                _ => vec![],
            };
        }
        let Some(target) = self.targets.get(&request.target_node) else {
            return vec![];
        };
        match request.action {
            Action::Click => vec![NoticesInput::Activate(target.clone())],
            Action::Focus => vec![NoticesInput::Focus(target.clone())],
            Action::SetValue if *target == NoticesTarget::Search => {
                if let Some(ActionData::Value(value)) = &request.data {
                    vec![NoticesInput::SetSearch(value.clone().into())]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}
pub fn notices_project(
    frame: &NoticesFrame,
    ui: &NoticesUi,
    scale: f64,
    epoch: u64,
) -> NoticesProjection {
    let mut nodes = BTreeMap::new();
    let mut targets = BTreeMap::new();
    let mut children = vec![];
    let mut focus = ROOT;
    for c in &frame.controls {
        let identity = format!(
            "{epoch}:{:?}:{}:{}",
            c.target,
            ui.query(),
            ui.selected()
                .map_or("", |component| component.source.as_str())
        );
        let hash = identity.bytes().fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        });
        let id = NodeId(hash | 0x8000000000000000);
        let mut node = Node::new(match c.target {
            NoticesTarget::Search => Role::SearchInput,
            NoticesTarget::License => Role::MultilineTextInput,
            NoticesTarget::Component(_) => Role::ListItem,
            NoticesTarget::Source => Role::Link,
            _ => Role::Button,
        });
        node.set_label(c.label.clone());
        if let Some(value) = &c.value {
            node.set_value(value.clone());
        }
        node.set_bounds(accesskit::Rect::new(
            c.hit_rect.x as f64 * scale,
            c.hit_rect.y as f64 * scale,
            (c.hit_rect.x + c.hit_rect.width) as f64 * scale,
            (c.hit_rect.y + c.hit_rect.height) as f64 * scale,
        ));
        node.add_action(Action::Focus);
        if c.target == NoticesTarget::Search {
            node.add_action(Action::SetValue);
        } else if c.target == NoticesTarget::License {
            node.set_read_only();
        } else {
            node.add_action(Action::Click);
        }
        if ui.focused() == Some(&c.target) {
            focus = id;
        }
        nodes.insert(id, node);
        targets.insert(id, c.target.clone());
        children.push(id);
    }
    let mut body = Node::new(Role::ScrollView);
    body.set_children(children);
    body.add_action(Action::ScrollDown);
    body.add_action(Action::ScrollUp);
    nodes.insert(BODY, body);
    let mut root = Node::new(Role::Window);
    root.set_label("第三方组件与许可");
    root.set_children([BODY]);
    nodes.insert(ROOT, root);
    NoticesProjection {
        nodes,
        targets,
        focus,
    }
}
