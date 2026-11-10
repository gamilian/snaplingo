//! Shared semantic projection of drawn Rust UI geometry into AccessKit trees.
mod capacity;
mod notices;
mod result;
mod window;
use crate::{
    Event, GeometryKind, Key, Modifiers, OcrField, Page, Point, Rect, Scene, Section, SettingsUi,
    Snapshot, Target, TranslationField, Value,
};
use accesskit::{
    Action, ActionData, ActionRequest, AriaCurrent, HasPopup, Live, Node, NodeId, Orientation,
    Role, TreeId, TreeInfo, TreeUpdate,
};
pub use capacity::{capacity_project, CapacityProjection};
pub use notices::{notices_project, NoticesProjection};
pub use result::{result_project, ResultProjection};
use std::collections::BTreeMap;
pub use window::{window_project, window_project_region, WindowProjection};
const ROOT: NodeId = NodeId(1);
const NAVIGATION: NodeId = NodeId(2);
const SECTIONS: NodeId = NodeId(3);
const CONTENT: NodeId = NodeId(4);
const STATUS: NodeId = NodeId(5);
const POPUP: NodeId = NodeId(6);
const CAPACITY: NodeId = NodeId(7);

#[derive(Clone, PartialEq)]
pub struct Projection {
    nodes: BTreeMap<NodeId, Node>,
    targets: BTreeMap<NodeId, Target>,
    focus: NodeId,
    scroll_point: Point,
}

impl Projection {
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

    pub fn actions(&self, request: &ActionRequest) -> Vec<Event> {
        if request.target_tree != TreeId::ROOT {
            return Vec::new();
        }
        let Some(node) = self.nodes.get(&request.target_node) else {
            return Vec::new();
        };
        if !node.supports_action(request.action) || node.is_disabled() {
            return Vec::new();
        }
        if request.target_node == CONTENT {
            let amount = if matches!(
                request.data,
                Some(ActionData::ScrollUnit(accesskit::ScrollUnit::Item))
            ) {
                48.
            } else {
                300.
            };
            return match request.action {
                Action::ScrollDown => vec![Event::Scroll {
                    position: self.scroll_point,
                    delta_y: amount,
                }],
                Action::ScrollUp => vec![Event::Scroll {
                    position: self.scroll_point,
                    delta_y: -amount,
                }],
                _ => Vec::new(),
            };
        }
        let Some(target) = self.targets.get(&request.target_node).copied() else {
            return Vec::new();
        };
        match request.action {
            Action::Click if node.role() == Role::ComboBox && node.is_expanded() == Some(true) => {
                vec![escape()]
            }
            Action::Click => vec![Event::Activate(target)],
            Action::Expand if node.is_expanded() == Some(false) => vec![Event::Activate(target)],
            Action::Collapse if node.is_expanded() == Some(true) => vec![escape()],
            Action::Focus | Action::ScrollIntoView => vec![Event::Focus(target)],
            Action::Increment | Action::Decrement => vec![
                Event::Focus(target),
                Event::KeyDown {
                    key: match (
                        request.action == Action::Increment,
                        node.role() == Role::SpinButton,
                    ) {
                        (true, true) => Key::ArrowUp,
                        (false, true) => Key::ArrowDown,
                        (true, false) => Key::ArrowRight,
                        (false, false) => Key::ArrowLeft,
                    },
                    modifiers: Modifiers::default(),
                    repeat: false,
                },
            ],
            Action::SetValue => match &request.data {
                Some(ActionData::Value(value)) => {
                    vec![Event::SetValue(target, Value::Text(value.to_string()))]
                }
                Some(ActionData::NumericValue(value))
                    if value.is_finite() && *value >= 0. && *value <= u32::MAX as f64 =>
                {
                    vec![Event::SetValue(target, Value::Number(value.round() as u32))]
                }
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }
}

fn escape() -> Event {
    Event::KeyDown {
        key: Key::Escape,
        modifiers: Modifiers::default(),
        repeat: false,
    }
}

fn bounds(rect: Rect, scale: f64) -> accesskit::Rect {
    accesskit::Rect::new(
        rect.x as f64 * scale,
        rect.y as f64 * scale,
        (rect.x + rect.width) as f64 * scale,
        (rect.y + rect.height) as f64 * scale,
    )
}
fn target_id(target: Target, page: Page) -> NodeId {
    NodeId(match target {
        Target::Navigate(p) => 100 + p as u64,
        Target::Section(s) => 200 + s as u64,
        Target::Ocr(f) => 300 + f as u64,
        Target::Translation(f) => 400 + f as u64,
        Target::Option(f, index) => 1000 + f as u64 * 100 + index as u64,
        Target::TranslationOption(f, index) => 3000 + f as u64 * 100 + index as u64,
        Target::Hotkey(index) => 6000 + page as u64 * 100 + index as u64,
        Target::ResetHotkey(index) => 7000 + page as u64 * 100 + index as u64,
        Target::ClearHotkey(index) => 8000 + page as u64 * 100 + index as u64,
        Target::ResetHotkeys => 9000 + page as u64 * 100,
        Target::CheckConflicts => 10000 + page as u64 * 100,
        Target::Capacity => CAPACITY.0,
        Target::Screenshot(field) => 20000 + screenshot_id(field),
        Target::ScreenshotStep(increase) => 62000 + u64::from(increase),
        Target::ScreenshotOption(field, index) => 30000 + screenshot_id(field) * 100 + index as u64,
        Target::ScreenshotColor(field, index) => 50000 + screenshot_id(field) * 100 + index as u64,
        Target::BrowseScreenshotDirectory => 60000,
        Target::PickScreenshotColor(field) => 61000 + screenshot_id(field),
        Target::General(field) => 70000 + field as u64,
        Target::GeneralOption(field, index) => 71000 + field as u64 * 100 + index as u64,
        Target::GeneralCommand(command) => 73000 + command as u64,
        Target::GeneralTheme(index) => 74000 + index as u64,
        Target::GeneralDisclosure(notes) => 76000 + u64::from(notes),
        Target::GeneralUnavailable => 76002,
        Target::GeneralStep(field, increase) => 75000 + field as u64 * 2 + u64::from(increase),
    })
}
fn screenshot_id(field: crate::ScreenshotField) -> u64 {
    use crate::ScreenshotField::*;
    match field {
        SavePath => 0,
        Format => 1,
        Quality => 2,
        NamingRule => 3,
        CustomFileName => 4,
        AutoCopy => 5,
        LongshotMaxHeight => 6,
        SelectionBorderWidth => 7,
        SelectionBorderColor => 8,
        SelectionMaskColor => 9,
        MaskOpacity => 10,
        RememberLastTool => 11,
        ShowSelectionSize => 12,
        ShowMagnifier => 13,
        MagnifierZoom => 14,
        PinOpacity => 15,
        PinShadow => 16,
        PinPosition => 17,
        PinBorder => 18,
        CaptureUiScale => 19,
        ResetAnnotationShortcuts => 20,
        PinBorderShortcut => 21,
        AnnotationShortcut(action) => {
            30 + crate::ANNOTATION_SHORTCUT_LABELS
                .iter()
                .position(|(id, _)| *id == action)
                .unwrap_or(99) as u64
        }
    }
}

fn control(
    target: Target,
    snapshot: &Snapshot,
    page: Page,
    section: Section,
    recording: Option<usize>,
) -> Node {
    let mut node = Node::new(Role::Button);
    let hotkeys: &[crate::Hotkey] = if page == Page::Screenshot {
        &snapshot.screenshot_hotkeys
    } else if page == Page::Translation {
        &snapshot.translation_hotkeys
    } else {
        &snapshot.hotkeys
    };
    let labels: &[&str] = if page == Page::Screenshot {
        &[
            "截图",
            "截图并复制",
            "显示/隐藏贴图",
            "上一组贴图",
            "下一组贴图",
        ]
    } else if page == Page::Translation {
        &["划词翻译", "截图翻译", "显示翻译窗口"]
    } else {
        &["截图 OCR", "静默截图 OCR", "上传图片 OCR"]
    };
    match target {
        Target::General(field) => {
            node = preference(
                field.label(),
                field.value(&snapshot.general),
                field.choices(),
                field
                    .range()
                    .map(|(min, max, step, suffix)| (min as f64, max as f64, step as f64, suffix)),
            );
            if field.text_input() {
                node.set_role(if field.range().is_some() {
                    Role::SpinButton
                } else {
                    Role::TextInput
                });
                node.add_action(Action::SetValue);
            }
        }
        Target::GeneralOption(field, index) => {
            node = option(field.choices(), index, field.value(&snapshot.general))
        }
        Target::GeneralCommand(command) => {
            node.set_label(command.label());
            if !command.enabled(&snapshot.general_runtime) {
                node.set_disabled();
            }
        }
        Target::GeneralUnavailable => {
            node.set_label("应用设置：暂未开放");
            node.set_disabled();
        }
        Target::GeneralDisclosure(notes) => node.set_label(if notes {
            "更新说明"
        } else {
            "授权遇到问题？"
        }),
        Target::GeneralStep(field, increase) => node.set_label(format!(
            "{}{}",
            if increase { "增加" } else { "减少" },
            field.label()
        )),
        Target::GeneralTheme(index) => {
            if let Some((id, label)) = crate::GeneralField::InterfaceTheme.choices().get(index) {
                node.set_label(*label);
                node.set_toggled(if snapshot.general.interface_theme == *id {
                    accesskit::Toggled::True
                } else {
                    accesskit::Toggled::False
                });
            }
        }
        Target::Screenshot(field) => {
            node = preference(
                field.label(),
                field.value(&snapshot.screenshot),
                field.choices(),
                None,
            );
            if field == crate::ScreenshotField::LongshotMaxHeight {
                node.set_role(Role::SpinButton);
                node.set_min_numeric_value(100.);
                node.set_max_numeric_value(100_000.);
                node.set_numeric_value_step(1_000.);
            }
        }
        Target::ScreenshotStep(increase) => node.set_label(if increase {
            "增加长截图最大高度"
        } else {
            "减少长截图最大高度"
        }),
        Target::ScreenshotOption(field, index) => {
            node = option(field.choices(), index, field.value(&snapshot.screenshot));
        }
        Target::ScreenshotColor(field, index) => {
            node.set_label(format!("{}预设颜色{}", field.label(), index + 1));
        }
        Target::BrowseScreenshotDirectory => node.set_label("选择截图保存目录"),
        Target::PickScreenshotColor(field) => node.set_label(format!("自定义{}", field.label())),
        Target::Navigate(p) => {
            node = Node::new(Role::Tab);
            node.set_label(p.label());
            node.set_selected(p == page);
            node.set_aria_current(if p == page {
                AriaCurrent::Page
            } else {
                AriaCurrent::False
            });
        }
        Target::Section(s) => {
            node = Node::new(Role::Tab);
            node.set_label(s.label());
            node.set_selected(s == section);
        }
        Target::Ocr(field) => {
            node = preference(
                field.label(),
                field.value(&snapshot.ocr),
                field.choices(),
                None,
            );
            if field == OcrField::GpuAcceleration {
                node.set_description("用于 PaddleOCR；设备不支持时自动使用 CPU");
            }
        }
        Target::Translation(field) => {
            let range = match field {
                TranslationField::MaxWindowHeightRatio => Some((30., 90., 1., "%")),
                TranslationField::WindowWidth => Some((300., 1000., 10., "pt")),
                _ => None,
            };
            node = preference(
                field.label(),
                field.value(&snapshot.translation),
                field.choices(),
                range,
            );
        }
        Target::Option(field, index) => {
            node = option(field.choices(), index, field.value(&snapshot.ocr));
        }
        Target::TranslationOption(field, index) => {
            node = option(field.choices(), index, field.value(&snapshot.translation));
        }
        Target::Hotkey(index) => {
            node.set_label(format!("{}快捷键", labels[index]));
            node.set_value(&hotkeys[index].value);
            node.set_description(if recording == Some(index) {
                "请按下快捷键，Esc 取消录制"
            } else {
                "激活以修改快捷键"
            });
        }
        Target::ResetHotkey(index) => {
            node.set_label(format!("恢复{}默认快捷键", labels[index]));
            if hotkeys[index].value == hotkeys[index].default_value {
                node.set_disabled();
            }
        }
        Target::ClearHotkey(index) => {
            node.set_label(format!("清除{}快捷键", labels[index]));
            if hotkeys[index].value == "未设置" {
                node.set_disabled();
            }
        }
        Target::ResetHotkeys => node.set_label("恢复这一组默认快捷键"),
        Target::CheckConflicts => node.set_label("检测快捷键冲突"),
        Target::Capacity => {
            node.set_label("历史与收藏设置");
            let c = &snapshot.capacity;
            node.set_description(format!(
                "保留 {} 天；历史 {} / {}；收藏 {} / {}",
                c.retention_days,
                c.history_used
                    .map(|v| v.to_string())
                    .unwrap_or("未知".into()),
                c.history_limit,
                c.favorites_used
                    .map(|v| v.to_string())
                    .unwrap_or("未知".into()),
                c.favorites_limit
            ));
        }
    }
    if !node.is_disabled() {
        if node.role() != Role::Slider {
            node.add_action(Action::Click);
        }
        if !matches!(
            target,
            Target::Option(..)
                | Target::TranslationOption(..)
                | Target::ScreenshotOption(..)
                | Target::GeneralOption(..)
        ) {
            node.add_action(Action::Focus);
            node.add_action(Action::ScrollIntoView);
        }
    }
    node
}
fn preference(
    label: &str,
    value: Value,
    choices: &[(&str, &str)],
    range: Option<(f64, f64, f64, &str)>,
) -> Node {
    match value {
        Value::Toggle(on) => {
            let mut node = Node::new(Role::Switch);
            node.set_label(label);
            node.set_toggled(on.into());
            node
        }
        Value::Text(value) => {
            let mut node = Node::new(Role::ComboBox);
            node.set_label(label);
            node.set_value(
                choices
                    .iter()
                    .find(|(id, _)| *id == value)
                    .map(|(_, label)| *label)
                    .unwrap_or(&value),
            );
            node.set_has_popup(HasPopup::Listbox);
            node.set_expanded(false);
            node.add_action(Action::Expand);
            node
        }
        Value::Number(value) => {
            let mut node = Node::new(Role::Slider);
            node.set_label(label);
            node.set_numeric_value(value as f64);
            node.set_orientation(Orientation::Horizontal);
            if let Some((min, max, step, suffix)) = range {
                node.set_min_numeric_value(min);
                node.set_max_numeric_value(max);
                node.set_numeric_value_step(step);
                node.set_value(format!("{value}{suffix}"));
            }
            node.add_action(Action::Increment);
            node.add_action(Action::Decrement);
            node.add_action(Action::SetValue);
            node
        }
    }
}
fn option(choices: &[(&str, &str)], index: usize, value: Value) -> Node {
    let mut node = Node::new(Role::ListBoxOption);
    if let Some((id, label)) = choices.get(index) {
        node.set_label(*label);
        node.set_selected(value == Value::Text((*id).into()));
    }
    node
}

pub fn project(scene: &Scene, ui: &SettingsUi, device_scale: f64) -> Projection {
    let snapshot = ui.snapshot();
    let page = ui.page();
    let mut nodes = BTreeMap::new();
    let mut targets = BTreeMap::new();
    let mut nav = Vec::new();
    let mut tabs = Vec::new();
    let mut content = Vec::new();
    let mut other = Vec::new();
    let mut options = Vec::new();
    let mut popup_owner = None;
    for geometry in &scene.geometry {
        let GeometryKind::Control(target) = geometry.kind else {
            continue;
        };
        let id = target_id(target, page);
        if nodes.contains_key(&id) {
            continue;
        }
        let mut node = control(
            target,
            snapshot,
            page,
            ui.active_section(),
            ui.recording_index(),
        );
        let rect = scene
            .hits
            .iter()
            .find(|h| h.target == target)
            .map(|h| h.rect)
            .unwrap_or(geometry.rect);
        node.set_bounds(bounds(rect, device_scale));
        targets.insert(id, target);
        match target {
            Target::GeneralOption(field, _) => {
                popup_owner = Some(Target::General(field));
                options.push(id);
            }
            Target::Navigate(_) => nav.push(id),
            Target::Section(_) => tabs.push(id),
            Target::Option(field, _) => {
                popup_owner = Some(Target::Ocr(field));
                options.push(id);
            }
            Target::TranslationOption(field, _) => {
                popup_owner = Some(Target::Translation(field));
                options.push(id);
            }
            Target::ScreenshotOption(field, _) => {
                popup_owner = Some(Target::Screenshot(field));
                options.push(id);
            }
            Target::Capacity => other.push(id),
            _ => content.push(id),
        }
        nodes.insert(id, node);
    }
    let mut root = Node::new(Role::Window);
    root.set_label(format!("SnapLingo · {}设置", page.label()));
    root.set_bounds(bounds(
        Rect {
            x: 0.,
            y: 0.,
            width: scene.size.width,
            height: scene.size.height,
        },
        device_scale,
    ));
    root.set_children(
        [
            vec![NAVIGATION, SECTIONS, CONTENT, STATUS],
            other,
            if popup_owner.is_some() {
                vec![POPUP]
            } else {
                Vec::new()
            },
        ]
        .concat(),
    );
    let mut navigation = Node::new(Role::TabList);
    navigation.set_label("主导航");
    navigation.set_children(nav);
    let mut sections = Node::new(Role::TabList);
    sections.set_label(format!("{}目录", page.label()));
    sections.set_children(tabs);
    let mut scroll = Node::new(Role::ScrollView);
    scroll.set_label(format!("{}设置内容", page.label()));
    scroll.set_bounds(bounds(scene.viewport, device_scale));
    scroll.set_clips_children();
    scroll.set_children(content);
    scroll.add_action(Action::ScrollDown);
    scroll.add_action(Action::ScrollUp);
    let mut status = Node::new(Role::Status);
    status.set_label(snapshot.error.as_deref().unwrap_or("自动保存"));
    status.set_live(Live::Polite);
    if let Some(owner) = popup_owner {
        let mut popup = Node::new(Role::ListBox);
        popup.set_label("选项");
        popup.set_children(options);
        let owner = nodes.get_mut(&target_id(owner, page)).unwrap();
        owner.set_expanded(true);
        owner.add_action(Action::Collapse);
        owner.set_controls([POPUP]);
        nodes.insert(POPUP, popup);
    }
    nodes.extend([
        (ROOT, root),
        (NAVIGATION, navigation),
        (SECTIONS, sections),
        (CONTENT, scroll),
        (STATUS, status),
    ]);
    let focus = ui
        .focused()
        .map(|target| target_id(target, page))
        .filter(|id| nodes.contains_key(id))
        .unwrap_or(ROOT);
    Projection {
        nodes,
        targets,
        focus,
        scroll_point: Point {
            x: scene.viewport.x + scene.viewport.width * 0.5,
            y: scene.viewport.y + scene.viewport.height * 0.5,
        },
    }
}

/// Native embedded pages show only Scene.content. Keep the same semantic
/// controls/actions while translating their real geometry into that NSView.
pub fn project_region(
    scene: &Scene,
    ui: &SettingsUi,
    device_scale: f64,
    region: Rect,
) -> Projection {
    let mut projection = project(scene, ui, device_scale);
    let removed = projection
        .targets
        .iter()
        .filter(|(_, target)| matches!(target, Target::Navigate(_) | Target::Capacity))
        .map(|(id, _)| *id)
        .chain([NAVIGATION])
        .collect::<Vec<_>>();
    for id in &removed {
        projection.nodes.remove(id);
        projection.targets.remove(id);
    }
    for node in projection.nodes.values_mut() {
        if let Some(rect) = node.bounds() {
            node.set_bounds(accesskit::Rect::new(
                rect.x0 - region.x as f64 * device_scale,
                rect.y0 - region.y as f64 * device_scale,
                rect.x1 - region.x as f64 * device_scale,
                rect.y1 - region.y as f64 * device_scale,
            ));
        }
        let children = node
            .children()
            .iter()
            .filter(|id| !removed.contains(id))
            .copied()
            .collect::<Vec<_>>();
        node.set_children(children);
    }
    let root = projection.nodes.get_mut(&ROOT).unwrap();
    root.set_role(Role::Pane);
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
    if removed.contains(&projection.focus) {
        projection.focus = ROOT;
    }
    projection
}

#[cfg(test)]
mod tests {
    #[test]
    fn longshot_spin_button_and_arrows_share_the_draft_and_blur_save() {
        use crate::ScreenshotField;
        let mut ui = SettingsUi::new(Snapshot::default());
        ui.update(Event::Activate(Target::Navigate(Page::Screenshot)));
        let scene = ui.frame(
            Size {
                width: 900.,
                height: 650.,
            },
            1.,
        );
        let tree = project(&scene, &ui, 2.);
        let field = Target::Screenshot(ScreenshotField::LongshotMaxHeight);
        let node = &tree.nodes[&target_id(field, Page::Screenshot)];
        assert_eq!(node.role(), Role::SpinButton);
        assert_eq!(node.numeric_value_step(), Some(1000.));
        for event in tree.actions(&request(field, Page::Screenshot, Action::Increment)) {
            assert!(ui.update(event).is_empty());
        }
        assert_eq!(ui.text_edit().unwrap().text(), "31000");
        for event in tree.actions(&request(
            Target::ScreenshotStep(false),
            Page::Screenshot,
            Action::Click,
        )) {
            assert!(ui.update(event).is_empty());
        }
        assert_eq!(ui.text_edit().unwrap().text(), "30000");
        for event in tree.actions(&request(
            Target::ScreenshotStep(true),
            Page::Screenshot,
            Action::Click,
        )) {
            assert!(ui.update(event).is_empty());
        }
        assert_eq!(
            ui.update(Event::FocusLost),
            vec![UiAction::EditScreenshot {
                field: ScreenshotField::LongshotMaxHeight,
                value: Value::Number(31_000)
            }]
        );
    }
    #[test]
    fn general_numeric_text_and_host_actions_follow_the_same_public_ui_edits() {
        use crate::{GeneralCommand, GeneralField};
        let mut ui = SettingsUi::new(Snapshot::default());
        ui.update(Event::Activate(Target::Navigate(Page::General)));
        let scene = ui.frame(
            Size {
                width: 900.,
                height: 650.,
            },
            1.25,
        );
        let tree = project(&scene, &ui, 2.);
        let target = Target::General(GeneralField::RequestTimeout);
        assert_eq!(
            tree.nodes[&target_id(target, Page::General)].role(),
            Role::SpinButton
        );
        assert_eq!(
            tree.actions(&request(target, Page::General, Action::Increment))
                .into_iter()
                .flat_map(|event| ui.update(event))
                .collect::<Vec<_>>(),
            [UiAction::EditGeneral {
                field: GeneralField::RequestTimeout,
                value: Value::Number(11_000)
            }]
        );
        let mut set = request(target, Page::General, Action::SetValue);
        set.data = Some(ActionData::NumericValue(24_000.));
        assert_eq!(
            tree.actions(&set)
                .into_iter()
                .flat_map(|event| ui.update(event))
                .collect::<Vec<_>>(),
            [UiAction::EditGeneral {
                field: GeneralField::RequestTimeout,
                value: Value::Number(24_000)
            }]
        );
        let target = Target::GeneralCommand(GeneralCommand::ClearLogs);
        assert_eq!(
            tree.actions(&request(target, Page::General, Action::Click))
                .into_iter()
                .flat_map(|event| ui.update(event))
                .collect::<Vec<_>>(),
            [UiAction::GeneralCommand(GeneralCommand::ClearLogs)]
        );
        assert!(tree.nodes[&target_id(Target::GeneralUnavailable, Page::General)].is_disabled());
        assert!(tree
            .actions(&request(
                Target::GeneralUnavailable,
                Page::General,
                Action::Click
            ))
            .is_empty());
        ui.update(Event::SetValue(
            Target::General(GeneralField::ProxyMode),
            Value::Text("manual".into()),
        ));
        let tree = project(
            &ui.frame(
                Size {
                    width: 900.,
                    height: 650.,
                },
                1.,
            ),
            &ui,
            1.,
        );
        let target = Target::General(GeneralField::ProxyUrl);
        assert_eq!(
            tree.nodes[&target_id(target, Page::General)].role(),
            Role::TextInput
        );
        let mut set = request(target, Page::General, Action::SetValue);
        set.data = Some(ActionData::Value("http://127.0.0.1:7890".into()));
        assert_eq!(
            tree.actions(&set)
                .into_iter()
                .flat_map(|event| ui.update(event))
                .collect::<Vec<_>>(),
            [UiAction::EditGeneral {
                field: GeneralField::ProxyUrl,
                value: Value::Text("http://127.0.0.1:7890".into())
            }]
        );
        let mut snapshot = ui.snapshot().clone();
        snapshot.general_runtime.update_busy = true;
        ui.update(Event::Snapshot(snapshot));
        let scene = ui.frame(
            Size {
                width: 900.,
                height: 650.,
            },
            1.,
        );
        let tree = project(&scene, &ui, 1.);
        let target = Target::GeneralCommand(GeneralCommand::CheckUpdates);
        assert!(tree.nodes[&target_id(target, Page::General)].is_disabled());
        assert!(tree
            .actions(&request(target, Page::General, Action::Click))
            .is_empty());
    }
    use super::*;
    use crate::{Action as UiAction, Size};
    fn request(target: Target, page: Page, action: Action) -> ActionRequest {
        ActionRequest {
            action,
            target_node: target_id(target, page),
            target_tree: TreeId::ROOT,
            data: None,
        }
    }
    #[test]
    fn semantics_share_geometry_scale_values_and_real_ui_actions() {
        let mut ui = SettingsUi::new(Snapshot::default());
        let scene = ui.frame(
            Size {
                width: 900.,
                height: 650.,
            },
            1.5,
        );
        let tree = project(&scene, &ui, 2.);
        let target = Target::Ocr(OcrField::GpuAcceleration);
        let id = target_id(target, Page::Ocr);
        let geometry = scene
            .geometry
            .iter()
            .find(|g| g.kind == GeometryKind::Control(target))
            .unwrap();
        assert_eq!(tree.nodes[&id].bounds(), Some(bounds(geometry.rect, 2.)));
        assert_eq!(tree.nodes[&id].role(), Role::Switch);
        assert_eq!(tree.nodes[&id].toggled(), Some(accesskit::Toggled::True));
        for event in tree.actions(&request(target, Page::Ocr, Action::Focus)) {
            assert!(ui.update(event).is_empty());
        }
        assert!(ui.snapshot().ocr.gpu_acceleration);
        assert_eq!(ui.focused(), Some(target));
        let mut actions = Vec::new();
        for event in tree.actions(&request(target, Page::Ocr, Action::Click)) {
            actions.extend(ui.update(event));
        }
        assert_eq!(
            actions,
            [UiAction::EditOcr {
                field: OcrField::GpuAcceleration,
                value: Value::Toggle(false)
            }]
        );
        assert!(tree.nodes[&target_id(Target::ResetHotkey(0), Page::Ocr)].is_disabled());
        assert!(tree
            .actions(&request(Target::ResetHotkey(0), Page::Ocr, Action::Click))
            .is_empty());
    }
    #[test]
    fn dropdown_and_incremental_publication_track_the_shared_model() {
        let mut ui = SettingsUi::new(Snapshot::default());
        ui.update(Event::Activate(Target::Ocr(OcrField::RecognitionLanguage)));
        let scene = ui.frame(
            Size {
                width: 900.,
                height: 650.,
            },
            1.,
        );
        let tree = project(&scene, &ui, 1.);
        assert_eq!(
            tree.nodes[&target_id(Target::Ocr(OcrField::RecognitionLanguage), Page::Ocr)]
                .is_expanded(),
            Some(true)
        );
        assert!(tree.nodes.contains_key(&POPUP));
        let action = request(
            Target::Option(OcrField::RecognitionLanguage, 3),
            Page::Ocr,
            Action::Click,
        );
        let actions = tree
            .actions(&action)
            .into_iter()
            .flat_map(|e| ui.update(e))
            .collect::<Vec<_>>();
        assert_eq!(
            actions,
            [UiAction::EditOcr {
                field: OcrField::RecognitionLanguage,
                value: Value::Text("en".into())
            }]
        );
        let changed = project(
            &ui.frame(
                Size {
                    width: 900.,
                    height: 650.,
                },
                1.,
            ),
            &ui,
            1.,
        );
        assert!(!changed.nodes.contains_key(&POPUP));
        assert!(changed.update(Some(&tree)).tree.is_none());
        assert!(changed.update(Some(&changed)).nodes.is_empty());
        assert!(
            changed.actions(&action).is_empty(),
            "A removed popup must reject stale accessibility actions"
        );
    }
    #[test]
    fn native_events_are_safe_to_send_through_winit_proxy() {
        fn send<T: Send>() {}

        send::<Projection>();
    }

    #[test]
    fn embedded_page_semantics_use_the_same_content_crop_and_exclude_retained_navigation() {
        let mut ui = SettingsUi::new(Snapshot::default());
        let scene = ui.frame(
            Size {
                width: 900.,
                height: 650.,
            },
            1.,
        );
        let full = project(&scene, &ui, 2.);
        let cropped = project_region(&scene, &ui, 2., scene.content);
        assert!(!cropped.nodes.contains_key(&NAVIGATION));
        assert!(!cropped.nodes.contains_key(&CAPACITY));
        assert_eq!(cropped.nodes[&ROOT].role(), Role::Pane);
        let target = Target::Ocr(OcrField::GpuAcceleration);
        let id = target_id(target, Page::Ocr);
        let a = full.nodes[&id].bounds().unwrap();
        let b = cropped.nodes[&id].bounds().unwrap();
        assert_eq!(a.x0 - scene.content.x as f64 * 2., b.x0);
        assert_eq!(a.y0 - scene.content.y as f64 * 2., b.y0);
        assert_eq!(
            cropped.actions(&request(target, Page::Ocr, Action::Click)),
            [Event::Activate(target)]
        );
    }

    #[test]
    fn translation_range_accessibility_uses_validated_shared_edits_and_rejects_old_page_nodes() {
        let mut ui = SettingsUi::new(Snapshot::default());
        ui.update(Event::Activate(Target::Navigate(Page::Translation)));
        let scene = ui.frame(
            Size {
                width: 1100.,
                height: 700.,
            },
            1.,
        );
        let tree = project(&scene, &ui, 1.75);
        let target = Target::Translation(TranslationField::WindowWidth);
        let node = &tree.nodes[&target_id(target, Page::Translation)];
        assert_eq!(node.role(), Role::Slider);
        assert_eq!(node.numeric_value(), Some(660.));
        assert_eq!(node.min_numeric_value(), Some(300.));
        assert_eq!(node.max_numeric_value(), Some(1000.));
        assert_eq!(node.numeric_value_step(), Some(10.));
        let actions = tree
            .actions(&request(target, Page::Translation, Action::Increment))
            .into_iter()
            .flat_map(|e| ui.update(e))
            .collect::<Vec<_>>();
        assert_eq!(
            actions,
            [UiAction::EditTranslation {
                field: TranslationField::WindowWidth,
                value: Value::Number(670)
            }]
        );
        let mut set = request(target, Page::Translation, Action::SetValue);
        set.data = Some(ActionData::NumericValue(720.));
        let actions = tree
            .actions(&set)
            .into_iter()
            .flat_map(|e| ui.update(e))
            .collect::<Vec<_>>();
        assert_eq!(
            actions,
            [UiAction::EditTranslation {
                field: TranslationField::WindowWidth,
                value: Value::Number(720)
            }]
        );
        set.data = Some(ActionData::NumericValue(5000.));
        assert!(tree
            .actions(&set)
            .into_iter()
            .flat_map(|e| ui.update(e))
            .collect::<Vec<_>>()
            .is_empty());
        assert!(tree
            .actions(&request(Target::Hotkey(0), Page::Ocr, Action::Click))
            .is_empty());
    }

    #[test]
    fn combo_collapse_clears_shared_popup_and_does_not_reopen_it() {
        let mut ui = SettingsUi::new(Snapshot::default());
        let target = Target::Ocr(OcrField::RecognitionLanguage);
        ui.update(Event::Activate(target));
        let scene = ui.frame(
            Size {
                width: 900.,
                height: 650.,
            },
            1.,
        );
        let tree = project(&scene, &ui, 1.);
        for event in tree.actions(&request(target, Page::Ocr, Action::Collapse)) {
            ui.update(event);
        }
        let tree = project(
            &ui.frame(
                Size {
                    width: 900.,
                    height: 650.,
                },
                1.,
            ),
            &ui,
            1.,
        );
        assert!(!tree.nodes.contains_key(&POPUP));
        assert!(tree
            .actions(&request(target, Page::Ocr, Action::Collapse))
            .is_empty());
        assert_eq!(
            tree.nodes[&target_id(target, Page::Ocr)].is_expanded(),
            Some(false)
        );
    }
}
