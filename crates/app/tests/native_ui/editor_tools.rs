use super::*;

pub(super) async fn draw_annotation(
    app: &NativeTestApp,
    id: &str,
    start: (f64, f64),
    end: (f64, f64),
) {
    let id = id.to_owned();
    on_main(app, move || {
        pointer(
            &id,
            &fixture_bounds(),
            start.0,
            start.1,
            "down",
            Flags::empty(),
        );
        pointer(
            &id,
            &fixture_bounds(),
            (start.0 + end.0) / 2.0,
            (start.1 + end.1) / 2.0,
            "move",
            Flags::empty(),
        );
        pointer(&id, &fixture_bounds(), end.0, end.1, "move", Flags::empty());
        pointer(&id, &fixture_bounds(), end.0, end.1, "up", Flags::empty());
    })
    .await;
}

fn is_tool(annotation: &AnnotationCommand, tool: &str) -> bool {
    matches!(
        (annotation, tool),
        (AnnotationCommand::Rectangle { .. }, "rectangle")
            | (AnnotationCommand::Ellipse { .. }, "ellipse")
            | (AnnotationCommand::Arrow { .. }, "arrow")
            | (AnnotationCommand::Line { .. }, "line")
            | (AnnotationCommand::Freehand { .. }, "pen")
            | (AnnotationCommand::Highlight { .. }, "highlight")
            | (
                AnnotationCommand::RegionHighlight { .. },
                "region-highlight"
            )
            | (AnnotationCommand::Blur { .. }, "blur")
            | (AnnotationCommand::Spotlight { .. }, "spotlight")
            | (AnnotationCommand::Mosaic { .. }, "mosaic")
            | (AnnotationCommand::Eraser { .. }, "eraser")
    )
}

pub(super) async fn run(app: &NativeTestApp, clipboard: &Mutex<Vec<u8>>) {
    let state = app.state.clone();
    let selected = LogicalRect {
        x: 120.0,
        y: 140.0,
        width: 240.0,
        height: 160.0,
    };
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
        let id = open(app, "screenshot").await;
        let frozen = state
            .capture
            .runtime
            .render_png(&CaptureSessionId(id.clone()), &selected, &[], false)
            .unwrap();
        drag(app, &id).await;
        native_editor_state(app, &id).await;
        // Enter select mode before choosing the tool for this isolated workflow.
        on_main(app, || toolbar::test_click("select")).await;
        if tool == "eraser" {
            on_main(app, || toolbar::test_click("rectangle")).await;
            on_main(app, || toolbar::test_styles(|style| style.test_button(0))).await;
            wait_for_styles(app, |style| style.style.filled).await;
            draw_annotation(app, &id, (150.0, 170.0), (300.0, 250.0)).await;
            native_editor_state(app, &id).await;
            on_main(app, || toolbar::test_click("select")).await;
        }
        on_main(app, move || toolbar::test_click(tool)).await;
        wait_for_toolbar(app, &id, Some(tool), tool == "eraser", false, true).await;
        if tool == "eraser" {
            draw_annotation(app, &id, (160.0, 210.0), (290.0, 210.0)).await;
        } else {
            draw_annotation(app, &id, (150.0, 170.0), (300.0, 250.0)).await;
        }
        let mut annotations = native_editor_state(app, &id).await.annotations;
        assert_eq!(annotations.len(), if tool == "eraser" { 2 } else { 1 });
        assert!(
            is_tool(annotations.last().unwrap(), tool),
            "Native {tool} produced the wrong annotation"
        );

        if tool == "rectangle" {
            draw_annotation(app, &id, (300.0, 250.0), (330.0, 270.0)).await;
            annotations = native_editor_state(app, &id).await.annotations;
            assert_eq!(annotations.len(), 1, "Resize created another rectangle");
            assert!(
                matches!(&annotations[0], AnnotationCommand::Rectangle { rect, .. }
                if rect.width == 180.0 && rect.height == 100.0)
            );
            draw_annotation(app, &id, (330.0, 270.0), (360.0, 300.0)).await;
            native_editor_state(app, &id).await;
            draw_annotation(app, &id, (360.0, 300.0), (350.0, 290.0)).await;
            let edge_resize = native_editor_state(app, &id).await;
            assert_eq!(
                edge_resize.selection, selected,
                "The annotation handle at the crop corner resized the crop"
            );
            assert_eq!(edge_resize.annotations.len(), 1);
            assert!(
                matches!(&edge_resize.annotations[0], AnnotationCommand::Rectangle { rect, .. }
                if rect.width == 200.0 && rect.height == 120.0)
            );
            press(app, &id, "\u{f703}", Flags::Shift, 124).await;
            press(app, &id, "\u{f701}", Flags::empty(), 125).await;
            annotations = native_editor_state(app, &id).await.annotations;
            assert!(
                matches!(&annotations[0], AnnotationCommand::Rectangle { rect, .. }
                if rect.x == 40.0 && rect.y == 31.0 && rect.width == 200.0 && rect.height == 120.0),
                "Keyboard nudge moved the selection instead of the selected annotation"
            );
        } else if tool == "arrow" {
            on_main(app, || toolbar::test_click("select")).await;
            draw_annotation(app, &id, (225.0, 210.0), (225.0, 210.0)).await;
            native_editor_state(app, &id).await;
            draw_annotation(
                app,
                &id,
                (200.0, 196.66666666666666),
                (190.0, 226.66666666666666),
            )
            .await;
            annotations = native_editor_state(app, &id).await.annotations;
            assert_eq!(annotations.len(), 1, "Curve editing created another arrow");
            assert!(
                matches!(&annotations[0], AnnotationCommand::Arrow { control1: Some(point), .. }
                if (point.x - 70.0).abs() < 0.001 && (point.y - 86.66666666666666).abs() < 0.001)
            );
        }

        let expected = state
            .capture
            .runtime
            .render_png(
                &CaptureSessionId(id.clone()),
                &selected,
                &annotations,
                false,
            )
            .unwrap();
        assert_ne!(
            expected, frozen,
            "Native {tool} did not change the frozen image"
        );
        if tool == "eraser" {
            let original = image::load_from_memory(&frozen).unwrap().to_rgba8();
            let erased = image::load_from_memory(&expected).unwrap().to_rgba8();
            let painted = state
                .capture
                .runtime
                .render_png(
                    &CaptureSessionId(id.clone()),
                    &selected,
                    &annotations[..1],
                    false,
                )
                .unwrap();
            let painted = image::load_from_memory(&painted).unwrap().to_rgba8();
            assert_ne!(painted.get_pixel(90, 70), original.get_pixel(90, 70));
            assert_eq!(
                erased.get_pixel(90, 70),
                original.get_pixel(90, 70),
                "Native eraser did not restore frozen pixels"
            );
        }
        on_main(app, || toolbar::test_click("undo")).await;
        let undone = native_editor_state(app, &id).await.annotations;
        assert_ne!(
            undone, annotations,
            "Native {tool} undo did not change state"
        );
        on_main(app, || toolbar::test_click("redo")).await;
        assert_eq!(
            native_editor_state(app, &id).await.annotations,
            annotations,
            "Native {tool} redo lost its geometry"
        );
        let snapshot_id = id.clone();
        let bytes = on_main(app, move || native::test_snapshot(&snapshot_id)).await;
        snapshot(&format!("capture-native-editor-{tool}.png"), &bytes);
        on_main(app, || toolbar::test_click("copy")).await;
        released(app, &id).await;
        assert_eq!(
            *clipboard.lock().unwrap(),
            expected,
            "Native {tool} clipboard output differed from its annotation state"
        );
        assert!(on_main(app, toolbar::test_state).await.is_none());
    }
    selection_and_suspension(app, clipboard).await;
    global_pin_shortcut(app, clipboard).await;
    keyboard_parity(app).await;
    println!("All native annotation tools, rectangle resize, arrow curves, undo/redo, eraser pixels and clipboard output passed without WebViews");
}

async fn keyboard_parity(app: &NativeTestApp) {
    let state = app.state.clone();
    let original = state.settings.configuration.snapshot().unwrap().screenshot;
    let mut preferences = original.clone();
    preferences
        .annotation_tool_shortcuts
        .insert("ellipse".into(), "v".into());
    state
        .settings
        .configuration
        .update_screenshot(preferences)
        .unwrap();
    let id = open(app, "screenshot").await;
    drag(app, &id).await;
    let editor = native_editor_state(app, &id).await;
    let width = editor.styling.style.stroke_width;
    press(app, &id, "]", Flags::empty(), 30).await;
    wait_for_styles(app, |style| style.style.stroke_width == (width + 1).min(32)).await;
    press(app, &id, "[", Flags::empty(), 33).await;
    wait_for_styles(app, |style| style.style.stroke_width == width).await;
    press(app, &id, "f", Flags::empty(), 3).await;
    wait_for_styles(app, |style| {
        style.style.filled != editor.styling.style.filled
    })
    .await;
    press(app, &id, "v", Flags::empty(), 9).await;
    wait_for_toolbar(app, &id, Some("ellipse"), false, false, true).await;
    press(app, &id, "v", Flags::empty(), 9).await;
    wait_for_toolbar(app, &id, None, false, false, true).await;
    press(app, &id, "a", Flags::empty(), 0).await;
    wait_for_toolbar(app, &id, Some("arrow"), false, false, true).await;
    press(app, &id, "\t", Flags::empty(), 48).await;
    wait_for_toolbar(app, &id, Some("line"), false, false, true).await;
    press(app, &id, "\t", Flags::empty(), 48).await;
    wait_for_toolbar(app, &id, Some("arrow"), false, false, true).await;
    draw_annotation(app, &id, (150.0, 170.0), (300.0, 250.0)).await;
    assert_eq!(native_editor_state(app, &id).await.annotations.len(), 1);
    press(app, &id, "\u{8}", Flags::Command | Flags::Option, 51).await;
    assert!(native_editor_state(app, &id).await.annotations.is_empty());
    press(app, &id, "", Flags::empty(), 53).await;
    released(app, &id).await;
    state
        .settings
        .configuration
        .update_screenshot(original)
        .unwrap();
    println!("Native editor retained configured tool keys, fill/size/cycle shortcuts, clear and one-key Escape cancellation");
}

async fn global_pin_shortcut(app: &NativeTestApp, clipboard: &Mutex<Vec<u8>>) {
    use crate::window::pinned as pinned_window;

    let state = app.state.clone();
    let prior_ids = on_main(app, pinned_window::test_ids).await;
    let id = open(app, "screenshot").await;
    drag(app, &id).await;
    native_editor_state(app, &id).await;
    on_main(app, || toolbar::test_click("select")).await;
    on_main(app, || toolbar::test_click("rectangle")).await;
    draw_annotation(app, &id, (150.0, 170.0), (300.0, 250.0)).await;
    let edited = native_editor_state(app, &id).await;
    let expected = state
        .capture
        .runtime
        .render_png(
            &CaptureSessionId(id.clone()),
            &edited.selection,
            &edited.annotations,
            false,
        )
        .unwrap();
    *clipboard.lock().unwrap() = vec![1, 2, 3];
    state.capture.runtime.pin_from_shortcut().await.unwrap();
    released(app, &id).await;
    let new_ids = on_main(app, pinned_window::test_ids)
        .await
        .into_iter()
        .filter(|id| !prior_ids.contains(id))
        .collect::<Vec<_>>();
    assert_eq!(
        new_ids.len(),
        1,
        "Global pin shortcut did not create exactly one native pin"
    );
    let pin_id = &new_ids[0];
    let pin = state.capture.pinned_images.get(pin_id).unwrap();
    assert_eq!((pin.width, pin.height), (240, 160));
    state.capture.pinned_images.copy(pin_id).await.unwrap();
    assert_eq!(
        *clipboard.lock().unwrap(),
        expected,
        "Global pin shortcut exported the frozen crop without its native annotations"
    );
    state.capture.pinned_images.remove(pin_id).unwrap();
    let closing_id = pin_id.clone();
    on_main(app, move || pinned_window::test_close(&closing_id))
        .await
        .unwrap();
    assert_eq!(
        on_main(app, pinned_window::test_ids).await.len(),
        prior_ids.len()
    );
    assert!(on_main(app, toolbar::test_state).await.is_none());
    println!("Global pin shortcut preserved native annotations and released its editor and pin resources");
}

async fn selection_and_suspension(app: &NativeTestApp, clipboard: &Mutex<Vec<u8>>) {
    let state = app.state.clone();
    let id = open(app, "screenshot").await;
    drag(app, &id).await;
    native_editor_state(app, &id).await;
    on_main(app, || toolbar::test_click("select")).await;
    on_main(app, || toolbar::test_click("rectangle")).await;
    draw_annotation(app, &id, (150.0, 170.0), (300.0, 250.0)).await;
    let annotations = native_editor_state(app, &id).await.annotations;
    on_main(app, || toolbar::test_click("select")).await;

    draw_annotation(app, &id, (360.0, 300.0), (390.0, 320.0)).await;
    let resized = native_editor_state(app, &id).await;
    assert_eq!(
        resized.selection,
        LogicalRect {
            x: 120.0,
            y: 140.0,
            width: 270.0,
            height: 180.0
        }
    );
    assert_eq!(
        resized.annotations, annotations,
        "Resizing the crop discarded its annotations"
    );

    let pointer_id = id.clone();
    on_main(app, move || {
        pointer(
            &pointer_id,
            &fixture_bounds(),
            140.0,
            280.0,
            "down",
            Flags::empty(),
        );
        pointer(
            &pointer_id,
            &fixture_bounds(),
            160.0,
            290.0,
            "move",
            Flags::empty(),
        );
    })
    .await;
    assert_eq!(native_editor_state(app, &id).await.selection.x, 140.0);
    press(app, &id, "", Flags::empty(), 53).await;
    assert_eq!(
        native_editor_state(app, &id).await.selection,
        resized.selection,
        "Escape did not restore the crop before its in-progress move"
    );
    assert!(state
        .capture
        .sessions
        .has_session(&CaptureSessionId(id.clone())));

    draw_annotation(app, &id, (140.0, 280.0), (160.0, 290.0)).await;
    press(app, &id, "\u{f702}", Flags::empty(), 123).await;
    press(app, &id, "\u{f700}", Flags::Shift, 126).await;
    let moved = native_editor_state(app, &id).await;
    assert_eq!(
        moved.selection,
        LogicalRect {
            x: 139.0,
            y: 140.0,
            width: 270.0,
            height: 180.0
        }
    );
    assert_eq!(
        moved.annotations, annotations,
        "Moving the crop changed local annotation coordinates"
    );

    on_main(app, || {
        toolbar::test_styles(|style| style.test_open_color())
    })
    .await;
    assert!(
        on_main(app, || toolbar::test_styles(
            |style| style.test_color_active()
        ))
        .await
    );
    let suspend_id = id.clone();
    on_main(app, move || native::suspend(&suspend_id)).await;
    assert!(
        !on_main(app, toolbar::test_state).await.unwrap().visible,
        "Suspending the capture left its toolbar visible"
    );
    assert!(
        !on_main(app, || {
            objc2_app_kit::NSColorPanel::sharedColorPanel(MainThreadMarker::new().unwrap())
                .isVisible()
        })
        .await,
        "Suspending the capture left its color picker visible"
    );
    let query_id = id.clone();
    assert!(
        !on_main(app, move || native::test_ready_size(&query_id))
            .await
            .2
    );
    let resume_id = id.clone();
    on_main(app, move || native::action_finished(&resume_id, None)).await;
    let resumed = native_editor_state(app, &id).await;
    assert!(!resumed.busy);
    assert_eq!(resumed.selection, moved.selection);
    assert_eq!(resumed.annotations, annotations);
    assert!(on_main(app, toolbar::test_state).await.unwrap().visible);
    let query_id = id.clone();
    assert!(
        on_main(app, move || native::test_ready_size(&query_id))
            .await
            .2
    );

    let expected = state
        .capture
        .runtime
        .render_png(
            &CaptureSessionId(id.clone()),
            &moved.selection,
            &annotations,
            false,
        )
        .unwrap();
    on_main(app, || toolbar::test_click("copy")).await;
    released(app, &id).await;
    assert_eq!(
        *clipboard.lock().unwrap(),
        expected,
        "Native output used the crop before its move/resize"
    );
    let copied = image::load_from_memory(&expected).unwrap();
    assert_eq!((copied.width(), copied.height()), (270, 180));
    assert!(on_main(app, toolbar::test_state).await.is_none());
    println!("Native crop move/resize, Escape restoration, keyboard nudges and dialog suspension passed with preserved annotations");
}
