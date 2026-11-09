use super::*;

fn point(x: f64, y: f64) -> LogicalPoint {
    LogicalPoint { x, y }
}
fn editor() -> CaptureEditor {
    CaptureEditor::new(
        LogicalRect {
            x: -300.0,
            y: 20.0,
            width: 400.0,
            height: 300.0,
        },
        Some(AnnotationTool::Rectangle),
        CaptureEditorStylePresentation::default(),
    )
}
fn draw(editor: &mut CaptureEditor, tool: &'static str) {
    assert!(editor.control(&CaptureEditorControlAction::Command(tool)));
    editor.pointer_down(point(30.0, 40.0), 1, false);
    editor.pointer_move(point(50.0, 60.0), false);
    editor.pointer_up(point(90.0, 100.0), false);
}
fn style(editor: &mut CaptureEditor, change: CaptureEditorStyleChange) {
    editor.control(&CaptureEditorControlAction::Style {
        change,
        commit: false,
    });
}

#[test]
fn every_drawing_tool_produces_the_canonical_output_command() {
    for tool in [
        "rectangle",
        "ellipse",
        "arrow",
        "line",
        "pen",
        "highlight",
        "region-highlight",
        "blur",
        "spotlight",
        "mosaic",
        "eraser",
    ] {
        let mut editor = editor();
        draw(&mut editor, tool);
        assert_eq!(editor.annotations().count(), 1, "{tool}");
        assert_eq!(tool_of(editor.annotations().next().unwrap()).name(), tool);
        assert!(editor.can_undo());
    }
}

#[test]
fn reverse_and_shift_draws_preserve_selection_local_geometry() {
    let mut editor = editor();
    editor.pointer_down(point(100.0, 80.0), 1, false);
    editor.pointer_up(point(40.0, 30.0), true);
    assert!(
        matches!(editor.annotations().next().unwrap(), AnnotationCommand::Rectangle { rect, .. }
        if *rect == LogicalRect { x: 50.0, y: 30.0, width: 50.0, height: 50.0 })
    );
    draw(&mut editor, "line");
    editor.pointer_down(point(20.0, 20.0), 1, false);
    editor.pointer_up(point(120.0, 40.0), true);
    assert!(
        matches!(editor.annotations().next_back().unwrap(), AnnotationCommand::Line { end, .. } if *end == point(120.0, 20.0))
    );
}

#[test]
fn tiny_shapes_are_discarded_but_mosaic_and_eraser_support_single_clicks() {
    let mut editor = editor();
    editor.pointer_down(point(20.0, 20.0), 1, false);
    editor.pointer_up(point(21.0, 21.0), false);
    assert_eq!(editor.annotations().count(), 0);
    assert!(!editor.can_undo());
    for tool in ["mosaic", "eraser"] {
        editor.control(&CaptureEditorControlAction::Command(tool));
        editor.pointer_down(point(20.0, 20.0), 1, false);
        editor.pointer_up(point(20.0, 20.0), false);
    }
    assert_eq!(editor.annotations().count(), 2);
}

#[test]
fn style_edits_and_moves_are_undoable_and_no_op_moves_do_not_add_history() {
    let mut editor = editor();
    draw(&mut editor, "rectangle");
    style(
        &mut editor,
        CaptureEditorStyleChange::Color([8, 16, 24, 200]),
    );
    style(&mut editor, CaptureEditorStyleChange::Filled(true));
    style(&mut editor, CaptureEditorStyleChange::CornerRadius(12.0));
    assert!(
        matches!(editor.annotations().next().unwrap(), AnnotationCommand::Rectangle { color, filled, corner_radius, .. }
        if *color == [8, 16, 24, 200] && *filled && *corner_radius == 12.0)
    );
    editor.control(&CaptureEditorControlAction::Command("select"));
    let count = editor.undo.len();
    editor.pointer_down(point(60.0, 70.0), 1, false);
    editor.pointer_up(point(60.0, 70.0), false);
    assert_eq!(editor.undo.len(), count);
    editor.pointer_down(point(60.0, 70.0), 1, false);
    editor.pointer_up(point(70.0, 90.0), true);
    assert_eq!(editor.undo.len(), count + 1);
    editor.control(&CaptureEditorControlAction::Command("undo"));
    assert_eq!(
        super::super::editor_geometry::bounds(editor.annotations().next().unwrap()).x,
        30.0
    );
    editor.control(&CaptureEditorControlAction::Command("redo"));
    assert_eq!(
        super::super::editor_geometry::bounds(editor.annotations().next().unwrap()).y,
        60.0
    );
}

#[test]
fn history_shares_unchanged_annotations_and_limits_undo_steps() {
    let mut editor = editor();
    draw(&mut editor, "pen");
    let original = editor.annotations[0].clone();
    for _ in 0..110 {
        draw(&mut editor, "rectangle");
    }
    assert!(Arc::ptr_eq(&original, &editor.annotations[0]));
    assert_eq!(editor.undo.len(), HISTORY_LIMIT);
    editor.control(&CaptureEditorControlAction::Command("undo"));
    draw(&mut editor, "line");
    assert!(!editor.can_redo());
    editor.control(&CaptureEditorControlAction::Command("clear"));
    assert_eq!(editor.annotations().count(), 0);
    assert!(!editor.can_undo());
    assert!(!editor.can_redo());
}

#[test]
fn text_drafts_keep_style_changes_and_support_editing_existing_text() {
    let mut editor = editor();
    editor.control(&CaptureEditorControlAction::Command("text"));
    editor.pointer_down(point(20.0, 60.0), 1, false);
    editor.update_text("原生文字".into());
    style(
        &mut editor,
        CaptureEditorStyleChange::FontFamily("Arial".into()),
    );
    style(&mut editor, CaptureEditorStyleChange::FontSize(36));
    style(&mut editor, CaptureEditorStyleChange::Bold(true));
    style(&mut editor, CaptureEditorStyleChange::Italic(true));
    assert_eq!(editor.annotations().count(), 0);
    assert!(editor.styling.text_mode);
    editor.commit_text();
    assert!(
        matches!(editor.annotations().next().unwrap(), AnnotationCommand::Text { text, font_family, font_size, bold, italic, .. }
        if text == "原生文字" && font_family == "Arial" && *font_size == 36 && *bold && *italic)
    );
    editor.control(&CaptureEditorControlAction::Command("select"));
    editor.pointer_down(point(30.0, 40.0), 2, false);
    assert_eq!(editor.text.as_ref().unwrap().replacing, Some(0));
    editor.update_text("修改文字".into());
    editor.commit_text();
    assert_eq!(editor.annotations().count(), 1);
    editor.control(&CaptureEditorControlAction::Command("undo"));
    assert!(
        matches!(editor.annotations().next().unwrap(), AnnotationCommand::Text { text, .. } if text == "原生文字")
    );
}

#[test]
fn escape_reverts_a_draft_before_deselecting_or_canceling_the_session() {
    let mut editor = editor();
    draw(&mut editor, "rectangle");
    assert!(editor.escape());
    assert!(editor.selected.is_none());
    assert!(editor.tool.is_some());
    assert!(editor.escape());
    assert!(editor.tool.is_none());
    assert!(!editor.escape());
    editor.control(&CaptureEditorControlAction::Command("text"));
    editor.pointer_down(point(20.0, 60.0), 1, false);
    editor.update_text("未保存".into());
    assert!(editor.escape());
    assert!(editor.text.is_none());
    assert_eq!(editor.annotations().count(), 1);
}

#[test]
fn curved_arrow_hit_testing_uses_the_curve_and_moves_control_points() {
    let mut editor = editor();
    editor.append(AnnotationCommand::Arrow {
        start: point(20.0, 20.0),
        end: point(100.0, 20.0),
        color: [255, 0, 0, 255],
        stroke_width: 2,
        control1: Some(point(20.0, 100.0)),
        control2: Some(point(100.0, 100.0)),
    });
    editor.tool = None;
    editor.pointer_down(point(60.0, 20.0), 1, false);
    assert!(editor.selected.is_none());
    editor.pointer_down(point(60.0, 80.0), 1, false);
    assert_eq!(editor.selected, Some(0));
    editor.pointer_up(point(70.0, 90.0), false);
    assert!(
        matches!(editor.annotations().next().unwrap(), AnnotationCommand::Arrow { control1: Some(control), .. } if *control == point(30.0, 110.0))
    );
}

#[test]
fn resizing_selected_rectangles_previews_without_history_then_commits_once() {
    let mut editor = editor();
    draw(&mut editor, "rectangle");
    let original = editor.export_annotations();
    let undo = editor.undo.len();
    let corner = editor
        .selected_handles()
        .into_iter()
        .find(|(handle, _)| *handle == AnnotationHandle::SouthEast)
        .unwrap()
        .1;
    editor.pointer_down(corner.clone(), 1, false);
    editor.pointer_move(point(110.0, 150.0), true);
    assert_eq!(editor.export_annotations(), original);
    assert_eq!(editor.undo.len(), undo);
    assert_eq!(
        editor.selected_bounds().unwrap(),
        LogicalRect {
            x: 30.0,
            y: 40.0,
            width: 110.0,
            height: 110.0
        }
    );
    editor.pointer_up(point(110.0, 150.0), true);
    assert_eq!(editor.undo.len(), undo + 1);
    assert_eq!(editor.annotations().count(), 1);
    editor.control(&CaptureEditorControlAction::Command("undo"));
    assert_eq!(editor.export_annotations(), original);
    editor.control(&CaptureEditorControlAction::Command("redo"));
    assert_eq!(
        super::super::editor_geometry::bounds(editor.annotations().next().unwrap()).width,
        110.0
    );
}

#[test]
fn edge_resize_keeps_minimum_size_and_escape_discards_the_preview() {
    let mut editor = editor();
    editor.append(AnnotationCommand::Ellipse {
        rect: LogicalRect {
            x: 30.0,
            y: 40.0,
            width: 200.0,
            height: 160.0,
        },
        color: [1, 2, 3, 255],
        stroke_width: 2,
        filled: false,
    });
    editor.selected = Some(0);
    editor.pointer_down(point(70.0, 40.0), 1, false);
    editor.pointer_move(point(70.0, 250.0), false);
    assert_eq!(editor.selected_bounds().unwrap().height, 4.0);
    assert!(editor.escape());
    assert_eq!(editor.selected_bounds().unwrap().height, 160.0);
    let undo = editor.undo.len();
    editor.pointer_down(point(230.0, 200.0), 1, false);
    editor.pointer_up(point(230.0, 200.0), false);
    assert_eq!(editor.undo.len(), undo);
}

#[test]
fn corner_radius_handle_updates_the_selected_shape_and_toolbar_style() {
    let mut editor = editor();
    draw(&mut editor, "rectangle");
    let handle = editor
        .selected_handles()
        .into_iter()
        .find(|(handle, _)| *handle == AnnotationHandle::Radius)
        .unwrap()
        .1;
    editor.pointer_down(handle.clone(), 1, false);
    editor.pointer_up(point(handle.x - 20.0, handle.y + 20.0), false);
    assert!(
        matches!(editor.annotations().next().unwrap(), AnnotationCommand::Rectangle { corner_radius, .. }
        if *corner_radius == 20.0)
    );
    assert_eq!(editor.styling.style.corner_radius, 20.0);
    editor.control(&CaptureEditorControlAction::Command("undo"));
    assert!(
        matches!(editor.annotations().next().unwrap(), AnnotationCommand::Rectangle { corner_radius, .. }
        if *corner_radius == 0.0)
    );
}

#[test]
fn arrow_control_drag_materializes_controls_and_double_click_resets_the_curve() {
    let mut editor = editor();
    editor.append(AnnotationCommand::Arrow {
        start: point(20.0, 30.0),
        end: point(140.0, 30.0),
        color: [1, 2, 3, 255],
        stroke_width: 2,
        control1: None,
        control2: None,
    });
    editor.selected = Some(0);
    editor.pointer_down(point(60.0, 30.0), 1, false);
    editor.pointer_up(point(60.0, 90.0), false);
    let curved = editor.export_annotations();
    assert!(
        matches!(&curved[0], AnnotationCommand::Arrow { control1: Some(c1), control2: Some(c2), .. }
        if *c1 == point(60.0, 90.0) && *c2 == point(100.0, 30.0))
    );
    editor.pointer_down(point(60.0, 90.0), 2, false);
    editor.pointer_up(point(60.0, 90.0), false);
    assert!(matches!(
        editor.annotations().next().unwrap(),
        AnnotationCommand::Arrow {
            control1: None,
            control2: None,
            ..
        }
    ));
    editor.control(&CaptureEditorControlAction::Command("undo"));
    assert_eq!(editor.export_annotations(), curved);
}

#[test]
fn arrow_endpoint_rotation_preserves_curvature_relative_to_the_anchor() {
    let mut editor = editor();
    editor.append(AnnotationCommand::Arrow {
        start: point(20.0, 30.0),
        end: point(140.0, 30.0),
        color: [1, 2, 3, 255],
        stroke_width: 2,
        control1: Some(point(60.0, 90.0)),
        control2: Some(point(100.0, 90.0)),
    });
    editor.selected = Some(0);
    editor.pointer_down(point(140.0, 30.0), 1, false);
    editor.pointer_up(point(20.0, 150.0), false);
    assert!(
        matches!(editor.annotations().next().unwrap(), AnnotationCommand::Arrow { start, end, control1: Some(c1), control2: Some(c2), .. }
        if *start == point(20.0, 30.0) && *end == point(20.0, 150.0)
        && *c1 == point(-40.0, 70.0) && *c2 == point(-40.0, 110.0))
    );
}

#[test]
fn resizing_multiline_text_changes_font_size_and_keeps_the_opposite_corner() {
    let mut editor = editor();
    editor.append(AnnotationCommand::Text {
        position: point(20.0, 100.0),
        text: "One\nTwo".into(),
        color: [1, 2, 3, 255],
        font_size: 20,
        font_family: "Arial".into(),
        bold: true,
        italic: true,
    });
    editor.selected = Some(0);
    let bottom_right = editor
        .selected_handles()
        .into_iter()
        .find(|(handle, _)| *handle == AnnotationHandle::SouthEast)
        .unwrap()
        .1;
    editor.pointer_down(bottom_right.clone(), 1, false);
    editor.pointer_up(point(bottom_right.x + 36.0, bottom_right.y + 48.0), true);
    assert!(
        matches!(editor.annotations().next().unwrap(), AnnotationCommand::Text { position, font_size, font_family, bold, italic, .. }
        if *font_size == 40 && *position == point(20.0, 148.0) && font_family == "Arial" && *bold && *italic)
    );
    assert_eq!(editor.styling.font_size, 40);
}

#[test]
fn selected_nudges_and_shift_equal_axis_moves_stay_diagonal() {
    let mut editor = editor();
    draw(&mut editor, "rectangle");
    assert!(editor.nudge_selected(point(1.0, -1.0)));
    editor.tool = None;
    editor.pointer_down(point(60.0, 70.0), 1, false);
    editor.pointer_up(point(70.0, 80.0), true);
    assert_eq!(editor.selected_bounds().unwrap().x, 41.0);
    assert_eq!(editor.selected_bounds().unwrap().y, 49.0);
    editor.selected = None;
    assert!(!editor.nudge_selected(point(1.0, 0.0)));
    draw(&mut editor, "line");
    editor.selected = Some(1);
    assert!(editor.selected_handles().is_empty());
}

#[test]
fn committing_whitespace_existing_text_deletes_it_and_undo_restores_it() {
    let mut editor = editor();
    editor.control(&CaptureEditorControlAction::Command("text"));
    editor.pointer_down(point(20.0, 80.0), 1, false);
    editor.update_text("保留原始文字".into());
    editor.commit_text();
    let original = editor.export_annotations();
    editor.control(&CaptureEditorControlAction::Command("select"));
    editor.pointer_down(point(30.0, 65.0), 2, false);
    editor.update_text(" \t\n ".into());
    editor.commit_text();
    assert_eq!(editor.annotations().count(), 0);
    assert!(editor.selected.is_none());
    editor.control(&CaptureEditorControlAction::Command("undo"));
    assert_eq!(editor.export_annotations(), original);
    editor.control(&CaptureEditorControlAction::Command("redo"));
    assert_eq!(editor.annotations().count(), 0);
}

#[test]
fn committing_new_whitespace_text_adds_no_history() {
    let mut editor = editor();
    editor.control(&CaptureEditorControlAction::Command("text"));
    editor.pointer_down(point(20.0, 80.0), 1, false);
    editor.update_text(" \n ".into());
    editor.commit_text();
    assert_eq!(editor.annotations().count(), 0);
    assert!(!editor.can_undo());
}

#[test]
fn selection_changes_keep_local_annotations_and_do_not_add_annotation_history() {
    let mut editor = editor();
    draw(&mut editor, "rectangle");
    let original = editor.export_annotations();
    let history = editor.undo.len();
    let revision = editor.image_revision;
    let next = LogicalRect {
        x: -200.0,
        y: -40.0,
        width: 250.0,
        height: 180.0,
    };
    assert!(editor.selection_changed(next.clone()));
    assert_eq!(editor.selection, next);
    assert_eq!(editor.export_annotations(), original);
    assert_eq!(editor.undo.len(), history);
    assert_eq!(editor.image_revision, revision + 1);
    assert!(!editor.selection_changed(next));
    assert_eq!(editor.image_revision, revision + 1);
}

#[test]
fn activating_the_current_tool_keeps_it_active_like_the_frontend() {
    let mut editor = editor();
    editor.control(&CaptureEditorControlAction::Command("rectangle"));
    assert_eq!(editor.tool, Some(AnnotationTool::Rectangle));
}
