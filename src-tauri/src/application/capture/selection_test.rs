use super::*;

fn rect(x: f64, y: f64, width: f64, height: f64) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}
fn point(x: f64, y: f64) -> LogicalPoint {
    LogicalPoint { x, y }
}
fn candidate(id: &str, bounds: LogicalRect, priority: i32) -> CaptureCandidateView {
    CaptureCandidateView {
        id: id.into(),
        kind: "window".into(),
        rect: bounds,
        priority,
    }
}
fn selection(candidates: Vec<CaptureCandidateView>) -> Selection {
    let bounds = rect(-300.0, -100.0, 800.0, 600.0);
    Selection::new(bounds.clone(), vec![bounds], candidates)
}

#[test]
fn all_capture_modes_use_native_selection() {
    for mode in [
        "screenshot",
        "screenshot-copy",
        "screenshot-ocr",
        "silent-screenshot-ocr",
        "screenshot-translate",
    ] {
        assert_eq!(CaptureSelectionMode::parse(mode).unwrap().as_str(), mode);
    }
    assert_eq!(CaptureSelectionMode::parse("unknown"), None);
}

#[test]
fn backwards_drag_preserves_negative_desktop_coordinates() {
    let mut state = selection(Vec::new());
    state.press(point(100.0, 250.0), true);
    state.move_to(point(-150.0, -60.0), false);
    assert_eq!(state.rect, Some(rect(-150.0, -60.0, 250.0, 310.0)));
    assert_eq!(state.release(point(-150.0, -60.0), false), state.rect);
}

#[test]
fn captured_drag_can_continue_onto_an_adjacent_frozen_display() {
    let mut state = selection(Vec::new());
    state.press(point(480.0, 300.0), false);
    state.move_to(point(650.0, 450.0), false);
    assert_eq!(
        state.release(point(650.0, 450.0), false),
        Some(rect(480.0, 300.0, 170.0, 150.0))
    );
}

#[test]
fn snap_and_square_match_existing_selection_rules() {
    let mut state = selection(vec![candidate(
        "window",
        rect(-100.0, 50.0, 200.0, 200.0),
        1,
    )]);
    state.press(point(-96.0, 52.0), false);
    assert_eq!(state.anchor, Some(point(-100.0, 50.0)));
    state.move_to(point(96.0, 210.0), true);
    assert_eq!(state.rect, Some(rect(-100.0, 50.0, 160.0, 160.0)));
    assert_eq!(
        state.release(point(96.0, 210.0), true),
        Some(rect(-100.0, 50.0, 160.0, 160.0))
    );
}

#[test]
fn tiny_drag_selects_pressed_window_and_empty_click_stays_open() {
    let window = rect(-100.0, 50.0, 200.0, 200.0);
    let mut state = selection(vec![candidate("window", window.clone(), 1)]);
    state.move_to(point(40.0, 80.0), false);
    state.press(point(40.0, 80.0), true);
    assert_eq!(state.release(point(41.0, 81.0), false), Some(window));
    state.press(point(350.0, 350.0), true);
    assert_eq!(state.release(point(350.0, 350.0), false), None);
}

#[test]
fn automatic_candidates_ignore_monitor_sized_and_tiny_windows() {
    let full = rect(-300.0, -100.0, 800.0, 600.0);
    let small = rect(0.0, 0.0, 47.0, 47.0);
    let window = rect(-20.0, -20.0, 100.0, 100.0);
    let mut state = selection(vec![
        candidate("full", full, 100),
        candidate("tiny", small, 90),
        candidate("window", window.clone(), 10),
    ]);
    state.move_to(point(10.0, 10.0), false);
    assert_eq!(state.hover, Some(window));
}

#[test]
fn z_order_then_area_break_candidate_ties() {
    let smaller = rect(0.0, 0.0, 100.0, 100.0);
    let top = rect(-20.0, -20.0, 200.0, 200.0);
    let mut state = selection(vec![
        candidate("small", smaller.clone(), 5),
        candidate("top", top.clone(), 6),
    ]);
    state.move_to(point(10.0, 10.0), false);
    assert_eq!(state.hover, Some(top));
    let mut state = selection(vec![
        candidate("small", smaller.clone(), 5),
        candidate("large", rect(-20.0, -20.0, 200.0, 200.0), 5),
    ]);
    state.move_to(point(10.0, 10.0), false);
    assert_eq!(state.hover, Some(smaller));
}

#[test]
fn keyboard_endpoint_wins_over_stale_mouse_release_and_stays_in_bounds() {
    let mut state = selection(Vec::new());
    state.press(point(300.0, 300.0), false);
    state.move_to(point(480.0, 480.0), false);
    state.nudge_cursor(1.0, -1.0);
    assert_eq!(
        state.release(point(480.0, 480.0), false),
        Some(rect(300.0, 300.0, 181.0, 179.0))
    );
    state.move_to(point(499.0, 499.0), false);
    assert_eq!(state.nudge_cursor(1.0, 1.0), Some(point(0.0, 0.0)));
}

#[test]
fn late_control_results_cannot_replace_a_new_pointer_or_drag() {
    let mut state = selection(Vec::new());
    state.move_to(point(10.0, 10.0), false);
    state.toggle_detection();
    let revision = state.revision;
    state.move_to(point(20.0, 20.0), false);
    assert!(!state.control_result(revision, Some(rect(0.0, 0.0, 100.0, 100.0))));
    assert!(state.control_result(state.revision, Some(rect(0.0, 0.0, 100.0, 100.0))));
    state.press(point(20.0, 20.0), true);
    assert!(!state.control_result(state.revision, Some(rect(0.0, 0.0, 200.0, 200.0))));
    assert_eq!(
        state.release(point(20.0, 20.0), false),
        Some(rect(0.0, 0.0, 100.0, 100.0))
    );
}

#[test]
fn control_failure_returns_to_window_detection_only_if_current() {
    let window = rect(0.0, 0.0, 100.0, 100.0);
    let mut state = selection(vec![candidate("window", window.clone(), 1)]);
    state.move_to(point(20.0, 20.0), false);
    state.toggle_detection();
    assert!(!state.control_failed(state.revision - 1));
    assert!(state.control_failed(state.revision));
    assert!(!state.control_detection);
    assert_eq!(state.hover, Some(window));
}

#[test]
fn history_deduplicates_limits_and_restores_on_changed_displays() {
    let current = (0..25)
        .map(|i| rect(i as f64, 0.0, 100.0, 100.0))
        .collect::<Vec<_>>();
    let history = merge_selection_history(current.clone(), current);
    assert_eq!(history.len(), 20);
    let history = remember_selection(history, rect(10.0, 0.0, 100.0, 100.0));
    assert_eq!(history.len(), 20);
    assert_eq!(history[0].x, 10.0);
    assert_eq!(
        restore_selection(&history[0], &rect(-300.0, -100.0, 200.0, 150.0)),
        Some(rect(-200.0, -50.0, 100.0, 100.0))
    );
    assert!(restore_selection(&history[0], &rect(0.0, 0.0, 50.0, 50.0)).is_none());
    assert!(merge_selection_history(
        Vec::new(),
        vec![
            rect(f64::NAN, 0.0, 100.0, 100.0),
            rect(0.0, 0.0, 1.0, 100.0)
        ]
    )
    .is_empty());
}
