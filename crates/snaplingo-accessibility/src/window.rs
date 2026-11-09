//! Typed accessibility for fixed preferences and dynamic Services/Library pages.
use std::collections::BTreeMap;

use accesskit::{
    Action, ActionData, ActionRequest, Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate,
};
use snaplingo_ui::{
    library::{LibraryInput, LibraryRole, LibraryTarget, LibraryUi},
    services::{ServiceControl, ServicesInput, ServicesRole, ServicesTarget},
    Event, Key, Modifiers, Page, Point, Rect, Target, WindowControls, WindowFrame, WindowInput,
    WindowUi,
};

use crate::{bounds, project_region, Projection, ROOT};

#[derive(Clone, PartialEq)]
enum DynamicTarget {
    Shell(Target),
    Services(ServicesTarget),
    Library(LibraryTarget),
}

fn service_field_label(
    field: &snaplingo_ui::services::ServiceField,
    ui: &snaplingo_ui::services::ServicesUi,
) -> String {
    use snaplingo_ui::services::{ServiceCustomField, ServiceField, ServiceForm};
    match field {
        ServiceField::Credential { name, .. } => match &ui.snapshot().form {
            Some(ServiceForm::Credentials { fields, .. }) => fields
                .iter()
                .find(|field| &field.name == name)
                .map(|field| field.label.clone())
                .unwrap_or_else(|| name.clone()),
            _ => name.clone(),
        },
        ServiceField::Custom(field) => match field {
            ServiceCustomField::Name => "服务名称",
            ServiceCustomField::Endpoint => "接口地址",
            ServiceCustomField::ApiKey => "API Key",
            ServiceCustomField::Model => "模型",
            ServiceCustomField::PromptName => "策略名称",
            ServiceCustomField::PromptDescription => "适用说明",
            ServiceCustomField::PromptText => "系统提示词",
        }
        .into(),
    }
}

#[derive(Clone, PartialEq)]
struct DynamicProjection {
    page: Page,
    nodes: BTreeMap<NodeId, Node>,
    targets: BTreeMap<NodeId, DynamicTarget>,
    scroll: BTreeMap<NodeId, Point>,
    focus: NodeId,
}

#[derive(Clone, PartialEq)]
enum Content {
    Settings(Projection),
    Dynamic(DynamicProjection),
}

#[derive(Clone, PartialEq)]
pub struct WindowProjection(Content);

impl WindowProjection {
    pub fn update(&self, previous: Option<&Self>) -> TreeUpdate {
        match &self.0 {
            Content::Settings(projection) => projection.update(previous.and_then(|previous| {
                if let Content::Settings(projection) = &previous.0 {
                    Some(projection)
                } else {
                    None
                }
            })),
            Content::Dynamic(projection) => {
                let previous = previous.and_then(|previous| {
                    if let Content::Dynamic(projection) = &previous.0 {
                        Some(projection)
                    } else {
                        None
                    }
                });
                TreeUpdate {
                    nodes: projection
                        .nodes
                        .iter()
                        .filter(|(id, node)| {
                            previous.is_none_or(|old| old.nodes.get(id) != Some(node))
                        })
                        .map(|(id, node)| (*id, node.clone()))
                        .collect(),
                    tree: previous.is_none().then(|| TreeInfo::new(ROOT)),
                    tree_id: TreeId::ROOT,
                    focus: projection.focus,
                }
            }
        }
    }

    pub fn actions(&self, request: &ActionRequest) -> Vec<WindowInput> {
        let Content::Dynamic(projection) = &self.0 else {
            let Content::Settings(projection) = &self.0 else {
                unreachable!()
            };
            return projection
                .actions(request)
                .into_iter()
                .map(WindowInput::Event)
                .collect();
        };
        if request.target_tree != TreeId::ROOT {
            return vec![];
        }
        let Some(node) = projection.nodes.get(&request.target_node) else {
            return vec![];
        };
        if node.is_disabled() || !node.supports_action(request.action) {
            return vec![];
        }
        if let Some(position) = projection
            .scroll
            .get(&request.target_node)
            .filter(|_| matches!(request.action, Action::ScrollUp | Action::ScrollDown))
        {
            let amount = if matches!(
                request.data,
                Some(ActionData::ScrollUnit(accesskit::ScrollUnit::Item))
            ) {
                48.
            } else {
                300.
            };
            let delta_y = if request.action == Action::ScrollUp {
                -amount
            } else {
                amount
            };
            return vec![if projection.page == Page::Services {
                WindowInput::Services(ServicesInput::Scroll {
                    position: *position,
                    delta_y,
                })
            } else {
                WindowInput::Library(LibraryInput::Scroll {
                    position: *position,
                    delta_y,
                })
            }];
        }
        let Some(target) = projection.targets.get(&request.target_node) else {
            return vec![];
        };
        match target {
            DynamicTarget::Shell(target) => match request.action {
                Action::Click => vec![WindowInput::Event(Event::Activate(*target))],
                Action::Focus | Action::ScrollIntoView => {
                    vec![WindowInput::Event(Event::Focus(*target))]
                }
                _ => vec![],
            },
            DynamicTarget::Services(target) => {
                let input = match request.action {
                    Action::Click
                        if node.role() == Role::ComboBox && node.is_expanded() == Some(true) =>
                    {
                        ServicesInput::KeyDown {
                            key: Key::Escape,
                            modifiers: Modifiers::default(),
                        }
                    }
                    Action::Click | Action::Expand => ServicesInput::Activate(target.clone()),
                    Action::Collapse => ServicesInput::KeyDown {
                        key: Key::Escape,
                        modifiers: Modifiers::default(),
                    },
                    Action::Focus | Action::ScrollIntoView => ServicesInput::Focus(target.clone()),
                    Action::SetValue => {
                        let value = match &request.data {
                            Some(ActionData::Value(value)) => value.to_string(),
                            Some(ActionData::NumericValue(value)) if value.is_finite() => {
                                value.round().to_string()
                            }
                            _ => return vec![],
                        };
                        ServicesInput::SetValue(target.clone(), value)
                    }
                    Action::Increment | Action::Decrement => {
                        let Some(value) = node.numeric_value() else {
                            return vec![];
                        };
                        let step = node.numeric_value_step().unwrap_or(1.);
                        let value = value
                            + if request.action == Action::Increment {
                                step
                            } else {
                                -step
                            };
                        ServicesInput::SetValue(target.clone(), value.round().to_string())
                    }
                    _ => return vec![],
                };
                vec![WindowInput::Services(input)]
            }
            DynamicTarget::Library(target) => match request.action {
                Action::Click => vec![WindowInput::Library(LibraryInput::Activate(*target))],
                Action::Focus | Action::ScrollIntoView => {
                    vec![WindowInput::Library(LibraryInput::Focus(*target))]
                }
                Action::SetValue => {
                    let Some(ActionData::Value(value)) = &request.data else {
                        return vec![];
                    };
                    vec![
                        WindowInput::Library(LibraryInput::Focus(*target)),
                        WindowInput::Library(LibraryInput::ImeDisabled),
                        WindowInput::Library(LibraryInput::Selection(
                            0..node.value().unwrap_or("").len(),
                        )),
                        WindowInput::Library(LibraryInput::Text(value.to_string())),
                    ]
                }
                _ => vec![],
            },
        }
    }
}

/// Complete standalone window semantics, including shared navigation.
pub fn window_project(frame: &WindowFrame, ui: &WindowUi, device_scale: f64) -> WindowProjection {
    if let (WindowControls::Settings, WindowUi::Settings(ui)) = (&frame.controls, ui) {
        return WindowProjection(Content::Settings(crate::project(
            &frame.scene,
            ui,
            device_scale,
        )));
    }
    let region = Rect {
        x: 0.,
        y: 0.,
        width: frame.scene.size.width,
        height: frame.scene.size.height,
    };
    let mut projection = window_project_region(frame, ui, device_scale, region);
    let Content::Dynamic(dynamic) = &mut projection.0 else {
        unreachable!()
    };
    let mut navigation = Vec::new();
    for hit in &frame.scene.hits {
        if !matches!(hit.target, Target::Navigate(_) | Target::Capacity) {
            continue;
        }
        let node_id = id(ui.page(), hit.target);
        if dynamic.nodes.contains_key(&node_id) {
            continue;
        }
        let mut node = Node::new(if matches!(hit.target, Target::Navigate(_)) {
            Role::Tab
        } else {
            Role::Button
        });
        common(&mut node, &hit.label, true, hit.rect, region, device_scale);
        node.remove_action(Action::Focus);
        node.remove_action(Action::ScrollIntoView);
        if let Target::Navigate(page) = hit.target {
            node.set_selected(page == ui.page());
        }
        dynamic
            .targets
            .insert(node_id, DynamicTarget::Shell(hit.target));
        dynamic.nodes.insert(node_id, node);
        navigation.push(node_id);
    }
    let mut children = navigation;
    children.extend_from_slice(dynamic.nodes[&ROOT].children());
    let root = dynamic.nodes.get_mut(&ROOT).unwrap();
    root.set_role(Role::Window);
    root.set_children(children);
    projection
}

// Namespaced deterministic FNV identifiers retain item identity across reordering;
// Services epochs and Library kind prevent callbacks from targeting another page.
fn id(page: Page, identity: impl std::fmt::Debug) -> NodeId {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in format!("{page:?}:{identity:?}").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    NodeId(0x1000_0000_0000_0000 | (hash & 0x0fff_ffff_ffff_ffff))
}

fn relative(rect: Rect, region: Rect, scale: f64) -> accesskit::Rect {
    bounds(
        Rect {
            x: rect.x - region.x,
            y: rect.y - region.y,
            ..rect
        },
        scale,
    )
}

fn editable(node: &mut Node) {
    node.add_action(Action::SetValue);
}

fn common(node: &mut Node, label: &str, enabled: bool, rect: Rect, region: Rect, scale: f64) {
    node.set_label(label);
    node.set_bounds(relative(rect, region, scale));
    if enabled {
        node.add_action(Action::Focus);
        node.add_action(Action::ScrollIntoView);
        node.add_action(Action::Click);
    } else {
        node.set_disabled();
    }
}

fn library_value(ui: &LibraryUi, target: LibraryTarget) -> String {
    if ui.focused() == Some(target) {
        if let Some(edit) = ui.text_edit() {
            return edit.text().to_owned();
        }
    }
    match target {
        LibraryTarget::Search => ui.query().search.clone(),
        LibraryTarget::Note(key) => ui
            .snapshot()
            .items
            .iter()
            .find(|item| item.key == key)
            .and_then(|item| item.note.clone())
            .unwrap_or_default(),
        LibraryTarget::Tags(key) => ui
            .snapshot()
            .items
            .iter()
            .find(|item| item.key == key)
            .map(|item| item.tags.join(", "))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Project the exact pixel crop mounted by a native host. Settings reuse their
/// existing projection; dynamic controls retain the page's typed targets.
pub fn window_project_region(
    frame: &WindowFrame,
    ui: &WindowUi,
    device_scale: f64,
    region: Rect,
) -> WindowProjection {
    if let (WindowControls::Settings, WindowUi::Settings(ui)) = (&frame.controls, ui) {
        return WindowProjection(Content::Settings(project_region(
            &frame.scene,
            ui,
            device_scale,
            region,
        )));
    }
    let page = ui.page();
    let mut projection = DynamicProjection {
        page,
        nodes: BTreeMap::new(),
        targets: BTreeMap::new(),
        scroll: BTreeMap::new(),
        focus: ROOT,
    };
    let mut children = Vec::new();
    match (&frame.controls, ui) {
        (WindowControls::Services(controls), WindowUi::Services(ui)) => {
            for control in controls {
                if matches!(control.target.control, ServiceControl::Shell(_)) {
                    continue;
                }
                let target = DynamicTarget::Services(control.target.clone());
                let node_id = id(page, &control.target);
                let mut node = Node::new(match control.role {
                    ServicesRole::Button | ServicesRole::DragHandle => Role::Button,
                    ServicesRole::Switch => Role::Switch,
                    ServicesRole::Tab => Role::Tab,
                    ServicesRole::TextInput => Role::TextInput,
                    ServicesRole::SecretInput => Role::PasswordInput,
                    ServicesRole::MultilineInput => Role::MultilineTextInput,
                    ServicesRole::Slider => Role::Slider,
                    ServicesRole::ComboBox => Role::ComboBox,
                    ServicesRole::Option => Role::ListBoxOption,
                });
                let label = if let ServiceControl::Field(field) = &control.target.control {
                    service_field_label(field, ui)
                } else {
                    control.label.clone()
                };
                common(
                    &mut node,
                    &label,
                    control.enabled,
                    control.rect,
                    region,
                    device_scale,
                );
                if let Some(value) = &control.value {
                    node.set_value(if control.role == ServicesRole::SecretInput {
                        "•".repeat(value.chars().count())
                    } else {
                        value.clone()
                    });
                }
                if matches!(control.role, ServicesRole::Tab | ServicesRole::Option) {
                    node.set_selected(control.selected);
                }
                if control.role == ServicesRole::Switch {
                    node.set_toggled(control.selected.into());
                }
                if matches!(
                    control.role,
                    ServicesRole::TextInput
                        | ServicesRole::SecretInput
                        | ServicesRole::MultilineInput
                        | ServicesRole::Slider
                ) {
                    editable(&mut node);
                }
                if let Some((min, max, step)) = control.range {
                    node.set_min_numeric_value(min as f64);
                    node.set_max_numeric_value(max as f64);
                    node.set_numeric_value_step(step as f64);
                    if let Some(value) = control
                        .value
                        .as_deref()
                        .and_then(|value| value.parse::<f64>().ok())
                    {
                        node.set_numeric_value(value);
                    }
                    node.add_action(Action::Increment);
                    node.add_action(Action::Decrement);
                }
                if control.role == ServicesRole::ComboBox {
                    let expanded = control.target.control == ServiceControl::VoiceMenu
                        && controls.iter().any(|control| {
                            matches!(control.target.control, ServiceControl::Voice(_))
                        });
                    node.set_has_popup(accesskit::HasPopup::Listbox);
                    node.set_expanded(expanded);
                    node.add_action(if expanded {
                        Action::Collapse
                    } else {
                        Action::Expand
                    });
                }
                if ui.focused().as_ref() == Some(&control.target) {
                    projection.focus = node_id;
                }
                children.push(node_id);
                projection.targets.insert(node_id, target);
                projection.nodes.insert(node_id, node);
            }
        }
        (WindowControls::Library(controls), WindowUi::Library(ui)) => {
            for control in controls {
                let node_id = id(page, control.target);
                let mut node = Node::new(match control.role {
                    LibraryRole::Button => Role::Button,
                    LibraryRole::ListItem => Role::ListBoxOption,
                    LibraryRole::SearchInput => Role::SearchInput,
                    LibraryRole::TextInput => Role::TextInput,
                    LibraryRole::MultilineInput => Role::MultilineTextInput,
                });
                common(
                    &mut node,
                    &control.label,
                    control.enabled,
                    control.rect,
                    region,
                    device_scale,
                );
                if matches!(control.role, LibraryRole::ListItem)
                    || matches!(control.target, LibraryTarget::Filter(_))
                {
                    node.set_selected(control.selected);
                }
                if matches!(
                    control.target,
                    LibraryTarget::Item(_) | LibraryTarget::Note(_) | LibraryTarget::Tags(_)
                ) {
                    node.add_action(Action::ScrollDown);
                    node.add_action(Action::ScrollUp);
                    projection.scroll.insert(
                        node_id,
                        Point {
                            x: control.hit_rect.x + control.hit_rect.width * 0.5,
                            y: control.hit_rect.y + control.hit_rect.height * 0.5,
                        },
                    );
                }
                if matches!(
                    control.role,
                    LibraryRole::SearchInput | LibraryRole::TextInput | LibraryRole::MultilineInput
                ) {
                    editable(&mut node);
                    node.set_value(library_value(ui, control.target));
                }
                if ui.focused() == Some(control.target) {
                    projection.focus = node_id;
                }
                children.push(node_id);
                projection
                    .targets
                    .insert(node_id, DynamicTarget::Library(control.target));
                projection.nodes.insert(node_id, node);
            }
        }
        _ => {}
    }
    let scroll_id = id(page, "scroll");
    let mut scroll = Node::new(Role::ScrollView);
    scroll.set_label(format!("{}内容", page.label()));
    scroll.set_bounds(relative(frame.scene.content, region, device_scale));
    scroll.set_clips_children();
    scroll.set_children(children);
    scroll.add_action(Action::ScrollDown);
    scroll.add_action(Action::ScrollUp);
    projection.scroll.insert(
        scroll_id,
        Point {
            x: frame.scene.viewport.x + frame.scene.viewport.width * 0.5,
            y: frame.scene.viewport.y + frame.scene.viewport.height * 0.5,
        },
    );
    projection.nodes.insert(scroll_id, scroll);
    let mut root = Node::new(Role::Pane);
    root.set_label(format!("SnapLingo · {}", page.label()));
    root.set_bounds(bounds(
        Rect {
            x: 0.,
            y: 0.,
            width: region.width,
            height: region.height,
        },
        device_scale,
    ));
    root.set_clips_children();
    root.set_children([scroll_id]);
    projection.nodes.insert(ROOT, root);
    WindowProjection(Content::Dynamic(projection))
}

#[cfg(test)]
mod tests {
    use super::*;
    use snaplingo_ui::{
        library::{
            ContentKind, ItemKey, LibraryContent, LibraryItem, LibraryKind, LibrarySnapshot,
        },
        services::{
            ServiceCustomDraft, ServiceCustomField, ServiceField, ServiceForm,
            ServicePromptWorkspace, ServicesIntent, ServicesRoute, ServicesSnapshot,
        },
        FontFamily, Size, TextMeasurement, TextMeasurer, WindowIntent, WindowSnapshot,
    };

    struct Metrics;
    impl TextMeasurer for Metrics {
        fn measure(&mut self, text: &str, size: f32, _: u16, _: FontFamily) -> TextMeasurement {
            TextMeasurement {
                caret_stops: text
                    .char_indices()
                    .enumerate()
                    .map(|(n, (byte, _))| (byte, n as f32 * size * 0.6))
                    .chain(std::iter::once((
                        text.len(),
                        text.chars().count() as f32 * size * 0.6,
                    )))
                    .collect(),
            }
        }
    }
    fn make_frame(ui: &mut WindowUi) -> WindowFrame {
        ui.frame(
            Size {
                width: 1100.,
                height: 800.,
            },
            1.25,
            &mut Metrics,
        )
    }
    fn project(ui: &mut WindowUi) -> WindowProjection {
        let frame = make_frame(ui);
        window_project_region(&frame, ui, 2., frame.scene.content)
    }
    fn nodes(projection: &WindowProjection) -> &BTreeMap<NodeId, Node> {
        match &projection.0 {
            Content::Dynamic(projection) => &projection.nodes,
            Content::Settings(projection) => &projection.nodes,
        }
    }
    fn request(target_node: NodeId, action: Action) -> ActionRequest {
        ActionRequest {
            action,
            target_node,
            target_tree: TreeId::ROOT,
            data: None,
        }
    }
    fn services_snapshot() -> ServicesSnapshot {
        ServicesSnapshot {
            route: ServicesRoute::Translation,
            form: Some(ServiceForm::Custom {
                draft: ServiceCustomDraft {
                    name: "Fixture".into(),
                    api_key: "secret-中文-123".into(),
                    ..Default::default()
                },
                prompts: ServicePromptWorkspace::default(),
            }),
            ..Default::default()
        }
    }
    fn service_target(frame: &WindowFrame, wanted: ServiceControl) -> ServicesTarget {
        let WindowControls::Services(controls) = &frame.controls else {
            panic!()
        };
        controls
            .iter()
            .find(|control| control.target.control == wanted)
            .unwrap()
            .target
            .clone()
    }

    #[test]
    fn deepl_mode_exposes_a_switch_checked_state_and_activates_it() {
        let mut ui = WindowUi::new(WindowSnapshot::Services(ServicesSnapshot {
            route: ServicesRoute::Translation,
            form: Some(ServiceForm::Credentials {
                provider_id: "deeplx".into(),
                name: "DeepLX".into(),
                fields: vec![],
            }),
            ..Default::default()
        }));
        let frame = make_frame(&mut ui);
        let target = service_target(&frame, ServiceControl::DeepLMode(true));
        let projection = window_project_region(&frame, &ui, 1., frame.scene.content);
        let node_id = id(Page::Services, &target);
        let node = &nodes(&projection)[&node_id];
        assert_eq!(node.role(), Role::Switch);
        assert_eq!(node.toggled(), Some(accesskit::Toggled::False));
        for input in projection.actions(&request(node_id, Action::Click)) {
            ui.input(input);
        }
        let frame = make_frame(&mut ui);
        let target = service_target(&frame, ServiceControl::DeepLMode(false));
        let projection = window_project_region(&frame, &ui, 1., frame.scene.content);
        assert_eq!(
            nodes(&projection)[&id(Page::Services, &target)].toggled(),
            Some(accesskit::Toggled::True)
        );
    }

    #[test]
    fn services_secret_values_are_masked_and_typed_edits_reject_old_epochs_and_pages() {
        let snapshot = services_snapshot();
        let mut ui = WindowUi::new(WindowSnapshot::Services(snapshot.clone()));
        let frame = make_frame(&mut ui);
        let target = service_target(
            &frame,
            ServiceControl::Field(ServiceField::Custom(ServiceCustomField::ApiKey)),
        );
        let node_id = id(Page::Services, &target);
        let projection = window_project_region(&frame, &ui, 2., frame.scene.content);
        let node = &nodes(&projection)[&node_id];
        assert_eq!(node.role(), Role::PasswordInput);
        assert!(node
            .value()
            .unwrap()
            .chars()
            .all(|character| character == '•'));
        assert!(!format!("{:?}", projection.update(None)).contains("secret-中文-123"));
        let mut set = request(node_id, Action::SetValue);
        set.data = Some(ActionData::Value("new-secret".into()));
        let intents = projection
            .actions(&set)
            .into_iter()
            .flat_map(|input| ui.input(input))
            .collect::<Vec<_>>();
        assert!(intents.iter().any(|intent| matches!(intent,
            WindowIntent::Services(ServicesIntent::EditCustom { field: ServiceCustomField::ApiKey, value, .. }) if value == "new-secret")));
        ui.snapshot(WindowSnapshot::Services(snapshot));
        let current = project(&mut ui);
        assert!(
            current.actions(&set).is_empty(),
            "Prior provider epochs must not reuse field nodes"
        );
        ui.snapshot(WindowSnapshot::Settings(snaplingo_ui::Snapshot::default()));
        assert!(
            project(&mut ui).actions(&set).is_empty(),
            "A hidden Services page must reject old actions"
        );
    }

    #[test]
    fn services_range_focus_disabled_and_crop_use_the_real_frame() {
        let mut ui = WindowUi::new(WindowSnapshot::Services(ServicesSnapshot {
            route: ServicesRoute::Tts,
            rate: 180,
            ..Default::default()
        }));
        let frame = make_frame(&mut ui);
        let target = service_target(&frame, ServiceControl::Rate);
        let node_id = id(Page::Services, &target);
        let projection = window_project_region(&frame, &ui, 1.75, frame.scene.content);
        let node = &nodes(&projection)[&node_id];
        assert_eq!(node.role(), Role::Slider);
        assert_eq!(node.numeric_value(), Some(180.));
        let WindowControls::Services(controls) = &frame.controls else {
            panic!()
        };
        let rect = controls
            .iter()
            .find(|control| control.target == target)
            .unwrap()
            .rect;
        assert_eq!(
            node.bounds(),
            Some(relative(rect, frame.scene.content, 1.75))
        );
        for input in projection.actions(&request(node_id, Action::Focus)) {
            ui.input(input);
        }
        let focused = project(&mut ui);
        assert_eq!(focused.update(None).focus, node_id);
        let intents = projection
            .actions(&request(node_id, Action::Increment))
            .into_iter()
            .flat_map(|input| ui.input(input))
            .collect::<Vec<_>>();
        assert!(intents.iter().any(|intent| matches!(intent, WindowIntent::Services(ServicesIntent::Rate(value)) if *value > 180)));
        let mut snapshot = services_snapshot();
        snapshot.busy = true;
        ui.snapshot(WindowSnapshot::Services(snapshot));
        let frame = make_frame(&mut ui);
        let save = service_target(&frame, ServiceControl::SaveForm);
        let projection = window_project_region(&frame, &ui, 1., frame.scene.content);
        let node_id = id(Page::Services, &save);
        assert!(nodes(&projection)[&node_id].is_disabled());
        assert!(projection
            .actions(&request(node_id, Action::Click))
            .is_empty());
    }

    fn item(id: i64) -> LibraryItem {
        LibraryItem {
            key: ItemKey {
                kind: ContentKind::Ocr,
                id,
            },
            relative_time: "刚刚".into(),
            date: "2026-11-08".into(),
            content: LibraryContent::Ocr {
                text: format!("OCR {id}"),
                language: Some("zh".into()),
                provider: "system".into(),
                confidence: None,
                can_rerun: true,
                favorite: false,
            },
            thumbnail: None,
            note: Some("原备注🙂".into()),
            tags: vec!["标签".into()],
        }
    }
    #[test]
    fn library_stable_items_edit_whole_unicode_values_and_reject_removed_or_other_page_nodes() {
        let first = item(4);
        let second = item(9);
        let key = first.key;
        let mut snapshot = LibrarySnapshot {
            kind: LibraryKind::Favorites,
            items: vec![first, second],
            total: 2,
            ..Default::default()
        };
        let mut ui = WindowUi::new(WindowSnapshot::Library(snapshot.clone()));
        let projection = project(&mut ui);
        let note_id = id(Page::Favorites, LibraryTarget::Note(key));
        assert_eq!(nodes(&projection)[&note_id].value(), Some("原备注🙂"));
        let mut set = request(note_id, Action::SetValue);
        set.data = Some(ActionData::Value("替换🙂".into()));
        for input in projection.actions(&set) {
            ui.input(input);
        }
        assert_eq!(ui.text_edit().unwrap().text(), "替换🙂");
        let focused = project(&mut ui);
        assert_eq!(focused.update(None).focus, note_id);
        assert!(focused.update(Some(&focused)).nodes.is_empty());
        snapshot.items.reverse();
        ui.snapshot(WindowSnapshot::Library(snapshot.clone()));
        let reordered = project(&mut ui);
        let item_id = id(Page::Favorites, LibraryTarget::Item(key));
        assert!(
            nodes(&reordered).contains_key(&item_id),
            "Reordering may not change an item's semantic identity"
        );
        snapshot.items.retain(|item| item.key != key);
        ui.snapshot(WindowSnapshot::Library(snapshot.clone()));
        assert!(project(&mut ui)
            .actions(&request(item_id, Action::Click))
            .is_empty());
        snapshot.kind = LibraryKind::History;
        snapshot.items.push(item(4));
        ui = WindowUi::new(WindowSnapshot::Library(snapshot));
        assert!(project(&mut ui)
            .actions(&request(item_id, Action::Click))
            .is_empty());
    }

    #[test]
    fn library_scroll_and_set_value_produce_validated_page_intents() {
        fn send<T: Send>() {}
        send::<WindowProjection>();
        let mut ui = WindowUi::new(WindowSnapshot::Library(LibrarySnapshot {
            kind: LibraryKind::History,
            items: vec![item(4)],
            total: 1,
            ..Default::default()
        }));
        let projection = project(&mut ui);
        let mut set = request(id(Page::History, LibraryTarget::Search), Action::SetValue);
        set.data = Some(ActionData::Value("中文".into()));
        let intents = projection
            .actions(&set)
            .into_iter()
            .flat_map(|input| ui.input(input))
            .collect::<Vec<_>>();
        assert!(intents.iter().any(|intent| matches!(intent, WindowIntent::Library(snaplingo_ui::library::LibraryIntent::Query { query, .. }) if query.search == "中文")));
        let inputs = projection.actions(&request(id(Page::History, "scroll"), Action::ScrollDown));
        assert!(
            matches!(inputs.as_slice(), [WindowInput::Library(LibraryInput::Scroll { delta_y, .. })] if *delta_y > 0.)
        );
        let mut wrong_tree = set;
        wrong_tree.target_tree = TreeId("00000000-0000-0000-0000-000000000001".parse().unwrap());
        assert!(projection.actions(&wrong_tree).is_empty());
    }

    #[test]
    fn standalone_dynamic_windows_expose_navigation_without_leaking_it_into_embedded_crops() {
        for snapshot in [
            WindowSnapshot::Services(ServicesSnapshot::default()),
            WindowSnapshot::Library(LibrarySnapshot::default()),
        ] {
            let mut ui = WindowUi::new(snapshot);
            let frame = make_frame(&mut ui);
            let full = window_project(&frame, &ui, 2.);
            let embedded = window_project_region(&frame, &ui, 2., frame.scene.content);
            for target in [Target::Navigate(Page::General), Target::Capacity] {
                let node_id = id(ui.page(), target);
                assert!(nodes(&full).contains_key(&node_id));
                assert!(!nodes(&embedded).contains_key(&node_id));
                assert!(
                    matches!(full.actions(&request(node_id, Action::Click)).as_slice(),
                    [WindowInput::Event(Event::Activate(actual))] if *actual == target)
                );
            }
            assert_eq!(nodes(&full)[&ROOT].role(), Role::Window);
            assert_eq!(nodes(&embedded)[&ROOT].role(), Role::Pane);
        }
    }
}
