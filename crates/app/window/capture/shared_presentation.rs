//! Capture presentation calculations, independent of a native window's DPI.
use snaplingo_core::application::capture::AnnotationFontProvider;

#[derive(Default)]
pub(super) struct PreviewRequests {
    revision: u64,
    requested: Option<(u64, u64)>,
}
impl PreviewRequests {
    pub(super) fn request(&mut self, editor: u64, selection: u64) -> Option<u64> {
        let key = (editor, selection);
        if self.requested == Some(key) {
            return None;
        }
        self.requested = Some(key);
        self.revision = self.revision.wrapping_add(1);
        Some(self.revision)
    }
    pub(super) fn invalidate(&mut self) {
        self.requested = None;
    }
    pub(super) fn is_current(&self, revision: u64) -> bool {
        self.requested.is_some() && revision == self.revision
    }
}
use ab_glyph::{Font, FontArc, ScaleFont};
use snaplingo_core::application::capture::CaptureEditor;
use snaplingo_core::domain::capture::{AnnotationCommand, CapturedCursor, LogicalRect};
use snaplingo_ui::{Point, Rect};

/// The same font, fallback-glyph and pixel rounding rules as annotation output.
/// Text is not wrapped: explicit newlines have the same layout in the editor
/// and in copied/saved pixels.
pub(super) fn text_geometry(
    editor: &CaptureEditor,
    text: &snaplingo_ui::TextEdit,
    fonts: &dyn AnnotationFontProvider,
    desktop_scale: f64,
    bounds: &LogicalRect,
) -> Option<(Rect, snaplingo_ui::TextLayout)> {
    static FALLBACK: std::sync::OnceLock<FontArc> = std::sync::OnceLock::new();
    let fallback = FALLBACK.get_or_init(|| {
        FontArc::try_from_slice(snaplingo_assets::NOTO_SANS_CJK_SC)
            .expect("Bundled annotation font is valid")
    });
    let draft = editor.text.as_ref()?;
    let style = &editor.styling;
    let resolved = fonts.resolve(
        &style.style.font_family,
        style.style.bold,
        style.style.italic,
    );
    let font = resolved.as_ref().map(|font| &font.font).unwrap_or(fallback);
    let size = (style.font_size as f64 * desktop_scale).ceil() as f32;
    let line_height = size * 1.2 / desktop_scale as f32;
    let display = text.display_text();
    let mut validator = snaplingo_ui::TextEdit::multiline(display.clone());
    let mut layout = snaplingo_ui::TextLayout {
        height: line_height,
        caret_stops: vec![],
    };
    let mut point = Point::default();
    let mut width = 0f32;
    for (byte, ch) in display.char_indices() {
        if validator.set_selection(byte..byte) {
            layout.caret_stops.push((byte, point));
        }
        if ch == '\n' {
            width = width.max(point.x);
            point.x = 0.;
            point.y += line_height;
            continue;
        }
        let font = if font.glyph_id(ch).0 == 0 {
            fallback
        } else {
            font
        };
        point.x += font.as_scaled(size).h_advance(font.glyph_id(ch)) / desktop_scale as f32;
    }
    layout.caret_stops.push((display.len(), point));
    layout.height = point.y + line_height;
    width = width.max(point.x);
    let rect = Rect {
        x: (editor.selection.x + draft.position.x - bounds.x) as f32,
        y: (editor.selection.y + draft.position.y - bounds.y) as f32 - size / desktop_scale as f32,
        width: width.max(40.),
        height: layout.height,
    };
    Some((rect, layout))
}

pub(super) fn local_rect(rect: &LogicalRect, bounds: &LogicalRect) -> Rect {
    Rect {
        x: (rect.x - bounds.x) as f32,
        y: (rect.y - bounds.y) as f32,
        width: rect.width as f32,
        height: rect.height as f32,
    }
}

/// winit pointer locations are physical client pixels. The desktop's uniform
/// scale, rather than the HWND's current monitor DPI, defines core coordinates.
pub(super) fn local_point(x: f64, y: f64, desktop_scale: f64) -> Point {
    Point {
        x: (x / desktop_scale) as f32,
        y: (y / desktop_scale) as f32,
    }
}

pub(super) fn cursor_rect(cursor: &CapturedCursor, bounds: &LogicalRect) -> Rect {
    local_rect(
        &LogicalRect {
            x: cursor.logical_position.x - cursor.hotspot.x,
            y: cursor.logical_position.y - cursor.hotspot.y,
            width: cursor.image_width as f64 / cursor.scale_factor,
            height: cursor.image_height as f64 / cursor.scale_factor,
        },
        bounds,
    )
}

/// A moving annotation retains its stacking position. A text draft replaces
/// the committed annotation while editing, so it is never painted twice.
pub(super) fn preview_annotations(editor: &CaptureEditor) -> Vec<AnnotationCommand> {
    let moving = editor.moving_index();
    let replacing = editor.text.as_ref().and_then(|draft| draft.replacing);
    let draft = editor.draft_annotation();
    let text = editor.text.as_ref().map(|draft| {
        let style = &editor.styling.style;
        AnnotationCommand::Text {
            position: draft.position.clone(),
            text: draft.text.clone(),
            color: style.color,
            font_size: editor.styling.font_size,
            font_family: style.font_family.clone(),
            bold: style.bold,
            italic: style.italic,
        }
    });
    let mut annotations = Vec::new();
    for (index, annotation) in editor.annotations().enumerate() {
        if moving == Some(index) {
            annotations.extend(draft.clone());
        } else if replacing == Some(index) {
            annotations.extend(text.clone());
        } else {
            annotations.push(annotation.clone());
        }
    }
    if moving.is_none() {
        annotations.extend(draft);
    }
    if replacing.is_none() {
        annotations.extend(text);
    }
    annotations
}

#[cfg(test)]
mod tests {
    use super::*;
    use snaplingo_core::application::capture::{
        AnnotationTool, CaptureEditorControlAction as Control, CaptureEditorStylePresentation,
        Selection,
    };
    use snaplingo_core::domain::capture::LogicalPoint;

    fn point(x: f64, y: f64) -> LogicalPoint {
        LogicalPoint { x, y }
    }
    fn rect(x: f64, y: f64, width: f64, height: f64) -> LogicalRect {
        LogicalRect {
            x,
            y,
            width,
            height,
        }
    }
    fn editor() -> CaptureEditor {
        CaptureEditor::new(
            rect(-800., -540., 400., 300.),
            Some(AnnotationTool::Rectangle),
            CaptureEditorStylePresentation::default(),
        )
    }

    #[test]
    fn mixed_dpi_negative_desktop_keeps_physical_pointer_frozen_pixels_and_cursor_aligned() {
        // A 2x primary display, a 1.25x display left of it and a display above.
        // The spanning HWND can report 1.25x while core uses uniform 2x.
        let bounds = rect(-1280., -540., 4160., 1620.);
        let physical = local_point(960., 600., 2.);
        assert_eq!(physical, Point { x: 480., y: 300. });
        let selection = rect(-800., -240., 400., 300.);
        let local = local_rect(&selection, &bounds);
        assert_eq!((local.x, local.y), (physical.x, physical.y));
        let cursor = CapturedCursor {
            logical_position: point(-800., -240.),
            hotspot: point(2., 3.),
            image_width: 48,
            image_height: 64,
            scale_factor: 2.,
            png_data: vec![],
        };
        assert_eq!(
            cursor_rect(&cursor, &bounds),
            Rect {
                x: 478.,
                y: 297.,
                width: 24.,
                height: 32.
            }
        );
    }

    #[test]
    fn selection_editor_move_resize_cancel_and_annotation_use_one_desktop_space() {
        let bounds = rect(-1280., -540., 4160., 1620.);
        let mut selection = Selection::new(bounds, vec![], vec![]);
        selection.press(point(-800., -540.), false);
        let selected = selection.release(point(-400., -240.), false).unwrap();
        let mut editor =
            CaptureEditor::new(selected, None, CaptureEditorStylePresentation::default());
        assert!(selection.begin_edit(point(-700., -440.), true));
        let moved = selection.update_edit(point(-600., -390.), false).unwrap();
        editor.selection_changed(moved);
        assert_eq!(editor.selection, rect(-700., -490., 400., 300.));
        assert!(selection.cancel_edit());
        editor.selection_changed(selection.rect.clone().unwrap());
        assert_eq!(editor.selection, rect(-800., -540., 400., 300.));
        editor.control(&Control::Command("rectangle"));
        editor.pointer_down(point(20., 30.), 1, false);
        editor.pointer_up(point(60., 80.), false);
        assert!(
            matches!(preview_annotations(&editor).as_slice(), [AnnotationCommand::Rectangle { rect: r, .. }] if *r == rect(20.,30.,40.,50.))
        );
    }

    #[test]
    fn previews_keep_stack_order_during_move_and_replace_edited_text_once() {
        let mut editor = editor();
        editor.pointer_down(point(20., 30.), 1, false);
        editor.pointer_up(point(90., 80.), false);
        editor.control(&Control::Command("text"));
        editor.pointer_down(point(50., 120.), 1, false);
        editor.update_text("原文🙂".into());
        assert_eq!(
            preview_annotations(&editor).len(),
            2,
            "Uncommitted text appears in the preview"
        );
        editor.commit_text();
        editor.control(&Control::Command("select"));
        editor.pointer_down(point(20., 50.), 1, false);
        editor.pointer_move(point(30., 60.), false);
        let preview = preview_annotations(&editor);
        assert!(matches!(preview[0], AnnotationCommand::Rectangle { .. }));
        assert!(matches!(preview[1], AnnotationCommand::Text { .. }));
        editor.pointer_up(point(30., 60.), false);
        editor.pointer_down(point(60., 110.), 2, true);
        assert!(editor.text.is_some());
        editor.update_text("更新中文".into());
        let preview = preview_annotations(&editor);
        assert_eq!(preview.len(), 2);
        assert!(matches!(&preview[1], AnnotationCommand::Text { text, .. } if text == "更新中文"));
        editor.escape();
        assert!(
            matches!(&preview_annotations(&editor)[1], AnnotationCommand::Text { text, .. } if text == "原文🙂")
        );
    }

    #[test]
    fn late_previews_cannot_restore_text_or_selection_from_an_older_request() {
        let mut editor = editor();
        let mut requests = PreviewRequests::default();
        let first = requests.request(editor.revision, 0).unwrap();
        assert!(requests.request(editor.revision, 0).is_none());
        editor.control(&Control::Command("text"));
        editor.pointer_down(point(30., 60.), 1, false);
        editor.update_text("before".into());
        let second = requests.request(editor.revision, 0).unwrap();
        editor.update_text("中文🙂".into());
        requests.invalidate();
        assert!(
            !requests.is_current(second),
            "Invalidation rejects an old callback before the next request"
        );
        let text_revision = requests.request(editor.revision, 0).unwrap();
        assert!(!requests.is_current(first));
        assert!(!requests.is_current(second));
        let moved = rect(-600., -400., 400., 300.);
        editor.selection_changed(moved);
        let selection_revision = requests.request(editor.revision, 1).unwrap();
        assert!(!requests.is_current(text_revision));
        assert!(requests.is_current(selection_revision));
        assert!(
            matches!(&preview_annotations(&editor)[0],AnnotationCommand::Text{text,..}if text=="中文🙂")
        );
    }

    #[test]
    fn text_caret_uses_output_font_advances_and_only_whole_graphemes_across_ime_lines() {
        struct Fallback;
        impl AnnotationFontProvider for Fallback {
            fn families(&self) -> Vec<String> {
                vec![]
            }
            fn resolve(
                &self,
                _: &str,
                _: bool,
                _: bool,
            ) -> Option<snaplingo_core::application::capture::AnnotationFont> {
                None
            }
        }
        let mut editor = editor();
        editor.control(&Control::Command("text"));
        editor.pointer_down(point(30., 60.), 1, false);
        let mut edit = snaplingo_ui::TextEdit::multiline("Ae\u{301}中\n🙂".into());
        let bounds = rect(-1280., -540., 4160., 1620.);
        let (area, layout) = text_geometry(&editor, &edit, &Fallback, 1.25, &bounds).unwrap();
        let font = FontArc::try_from_slice(snaplingo_assets::NOTO_SANS_CJK_SC).unwrap();
        let expected = font.as_scaled(30.).h_advance(font.glyph_id('A')) / 1.25;
        assert!(
            (layout
                .caret_stops
                .iter()
                .find(|(byte, _)| *byte == 1)
                .unwrap()
                .1
                .x
                - expected)
                .abs()
                < 0.001
        );
        assert!(
            !layout.caret_stops.iter().any(|(byte, _)| *byte == 2),
            "Caret cannot split e and combining accent"
        );
        assert_eq!(area.x, 510.);
        assert_eq!(area.y, 36.);
        assert_eq!(layout.caret_stops.last().unwrap().1.y, 28.8);
        edit.select_all();
        edit.preedit("候选", Some(6..6));
        let (_, draft) = text_geometry(&editor, &edit, &Fallback, 1.25, &bounds).unwrap();
        assert_eq!(draft.caret_stops.last().unwrap().0, 6);
        assert_eq!(edit.text(), "Ae\u{301}中\n🙂");
    }
}
