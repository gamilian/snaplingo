use super::*;
use accesskit::{Action as AxAction, Node, NodeId, Role, TreeId, TreeInfo};
use std::collections::BTreeMap;

pub(super) struct Semantics {
    nodes: BTreeMap<NodeId, Node>,
    targets: BTreeMap<NodeId, CaptureTarget>,
    text_id: Option<NodeId>,
    focus: Option<NodeId>,
}
impl Default for Semantics {
    fn default() -> Self {
        Self {
            nodes: BTreeMap::from([(NodeId(1), Node::new(Role::Window))]),
            targets: BTreeMap::new(),
            text_id: None,
            focus: None,
        }
    }
}
impl Semantics {
    pub(super) fn update(&self) -> TreeUpdate {
        TreeUpdate {
            nodes: self
                .nodes
                .iter()
                .map(|(id, node)| (*id, node.clone()))
                .collect(),
            tree: Some(TreeInfo::new(NodeId(1))),
            tree_id: TreeId::ROOT,
            focus: self.focus.unwrap_or(NodeId(1)),
        }
    }
}
fn identity(session: &str, target: &str) -> NodeId {
    let hash = session
        .bytes()
        .chain(target.bytes())
        .fold(0xcbf29ce484222325u64, |hash, b| {
            (hash ^ u64::from(b)).wrapping_mul(0x100000001b3)
        });
    NodeId(hash | 0x8000000000000000)
}
pub(super) fn project(
    frame: &CaptureFrame,
    snapshot: &CaptureSnapshot,
    scale: f64,
    session: &str,
    text: Option<&snaplingo_ui::TextEdit>,
    text_rect: Option<Rect>,
    focus: Option<CaptureTarget>,
) -> Semantics {
    let mut result = Semantics::default();
    let mut root = Node::new(Role::Window);
    root.set_label("截图和标注");
    let mut children = vec![];
    for control in &frame.controls {
        let id = identity(session, &format!("{:?}", control.target));
        let mut node = Node::new(if control.range.is_some() {
            Role::Slider
        } else {
            Role::Button
        });
        node.set_label(control.label.clone());
        let r = control.rect;
        node.set_bounds(accesskit::Rect::new(
            r.x as f64 * scale,
            r.y as f64 * scale,
            (r.x + r.width) as f64 * scale,
            (r.y + r.height) as f64 * scale,
        ));
        if control.enabled {
            node.add_action(AxAction::Click);
            node.add_action(AxAction::Focus);
            if let Some((min, max)) = control.range {
                node.set_min_numeric_value(min);
                node.set_max_numeric_value(max);
                node.add_action(AxAction::SetValue);
                node.add_action(AxAction::Increment);
                node.add_action(AxAction::Decrement);
            }
        } else {
            node.set_disabled();
        }
        if let Some(value) = control.value {
            node.set_numeric_value(value);
        }
        if matches!(
            control.target,
            CaptureTarget::Tool(_)
                | CaptureTarget::Fill
                | CaptureTarget::Bold
                | CaptureTarget::Italic
                | CaptureTarget::Palette(_)
                | CaptureTarget::Command(CaptureCommand::Select)
        ) {
            node.set_toggled(control.selected.into());
        }
        if focus == Some(control.target) {
            result.focus = Some(id);
        }
        result.nodes.insert(id, node);
        result.targets.insert(id, control.target);
        children.push(id);
    }
    if let Some(text) = text {
        let id = identity(session, "annotation-text");
        let mut node = Node::new(Role::MultilineTextInput);
        node.set_label("标注文字");
        node.set_value(text.display_text());
        if let Some(r) = text_rect {
            node.set_bounds(accesskit::Rect::new(
                r.x as f64 * scale,
                r.y as f64 * scale,
                (r.x + r.width) as f64 * scale,
                (r.y + r.height) as f64 * scale,
            ));
        }
        if snapshot.busy {
            node.set_disabled();
        } else {
            node.add_action(AxAction::Focus);
            node.add_action(AxAction::SetValue);
        }
        result.text_id = Some(id);
        result.focus = Some(id);
        result.nodes.insert(id, node);
        children.push(id);
    }
    let mut hint = Node::new(Role::Status);
    hint.set_label(snapshot.hint.clone());
    hint.set_live(accesskit::Live::Polite);
    result.nodes.insert(NodeId(2), hint);
    children.push(NodeId(2));
    root.set_children(children);
    result.nodes.insert(NodeId(1), root);
    result
}
impl View {
    pub(super) fn accessibility(&mut self, request: &accesskit::ActionRequest) {
        if request.target_tree != TreeId::ROOT {
            return;
        }
        if self.busy {
            return;
        }
        let text_id = self.semantics.lock().unwrap().text_id;
        if text_id == Some(request.target_node) {
            match request.action {
                AxAction::Focus => {
                    self.window.focus_window();
                }
                AxAction::SetValue => {
                    if let Some(accesskit::ActionData::Value(value)) = &request.data {
                        if let Some(text) = &mut self.text {
                            self.text_undo.push(text.clone());
                            self.text_redo.clear();
                            text.select_all();
                            text.insert(value);
                        }
                        self.sync_text();
                    }
                }
                _ => {}
            }
            return;
        }
        let target = self
            .semantics
            .lock()
            .unwrap()
            .targets
            .get(&request.target_node)
            .copied();
        let Some(target) = target else {
            return;
        };
        let Some(control) = self
            .frame
            .controls
            .iter()
            .find(|c| c.target == target && c.enabled)
            .cloned()
        else {
            return;
        };
        match request.action {
            AxAction::Click => {
                let intents = self.ui.activate(target, &self.snapshot);
                self.intents(intents);
            }
            AxAction::Focus => {
                self.toolbar_focus = Some(target);
                self.window.focus_window();
                self.refresh();
            }
            AxAction::SetValue | AxAction::Increment | AxAction::Decrement => {
                if let Some((min, max)) = control.range {
                    let value = match (&request.action, &request.data) {
                        (AxAction::SetValue, Some(accesskit::ActionData::NumericValue(value))) => {
                            *value
                        }
                        (AxAction::Increment, _) => control.value.unwrap_or(min) + 1.,
                        (AxAction::Decrement, _) => control.value.unwrap_or(min) - 1.,
                        _ => return,
                    }
                    .clamp(min, max);
                    let point = Point {
                        x: control.rect.x
                            + control.rect.width * ((value - min) / (max - min)) as f32,
                        y: control.rect.y + control.rect.height / 2.,
                    };
                    let down = self
                        .ui
                        .pointer_down(point, &self.snapshot)
                        .unwrap_or_default();
                    self.intents(down);
                    let up = self.ui.pointer_up(point, &self.snapshot);
                    self.intents(up);
                }
            }
            _ => {}
        }
    }
}
