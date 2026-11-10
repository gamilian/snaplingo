//! Capture presentation calculations, independent of a native window's DPI.

use snaplingo_core::capture::CaptureEditorStylePresentation;
#[cfg(target_os = "macos")]
use snaplingo_core::capture::CaptureEditorToolbarPresentation;
use snaplingo_ui::capture::CaptureStyle;
#[cfg(target_os = "macos")]
use snaplingo_ui::capture::{CaptureCommand, CaptureSnapshot, CaptureTarget, CaptureTool};

pub(super) fn style(source: &CaptureEditorStylePresentation) -> CaptureStyle {
    CaptureStyle {
        color: source.style.color,
        stroke_width: source.style.stroke_width,
        font_size: source.font_size,
        filled: source.style.filled,
        corner_radius: source.style.corner_radius as f32,
        blur_radius: source.style.blur_radius as f32,
        bold: source.style.bold,
        italic: source.style.italic,
        font_family: source.style.font_family.clone(),
        fonts: source.font_families.clone(),
        presets: source.color_presets.clone(),
        text_mode: source.text_mode,
        fill_enabled: source.fill_enabled,
        round_enabled: source.round_enabled,
        blur_mode: source.blur_mode,
        error: source.error.clone(),
    }
}

#[cfg(target_os = "macos")]
pub(super) fn toolbar(source: &CaptureEditorToolbarPresentation) -> CaptureSnapshot {
    CaptureSnapshot {
        size: snaplingo_ui::Size {
            width: source.frame.width as f32,
            height: source.frame.height as f32,
        },
        ui_scale: source.ui_scale as f32,
        theme: match source.interface_theme.as_str() {
            "studio" => snaplingo_ui::Theme::Studio,
            "paper" => snaplingo_ui::Theme::Paper,
            _ => snaplingo_ui::Theme::Native,
        },
        dark: source.dark,
        editing: source.visible,
        tool: CaptureTool::ALL
            .into_iter()
            .find(|tool| Some(tool.name()) == source.active_tool.as_deref()),
        can_undo: source.can_undo,
        can_redo: source.can_redo,
        busy: source.busy,
        style: style(&source.styling),
        ..Default::default()
    }
}

#[cfg(target_os = "macos")]
pub(super) fn target(name: &str) -> CaptureTarget {
    if let Some(tool) = CaptureTool::ALL
        .into_iter()
        .find(|tool| tool.name() == name)
    {
        return CaptureTarget::Tool(tool);
    }
    CaptureTarget::Command(match name {
        "select" => CaptureCommand::Select,
        "undo" => CaptureCommand::Undo,
        "redo" => CaptureCommand::Redo,
        "clear" => CaptureCommand::Clear,
        "cancel" => CaptureCommand::Cancel,
        "ocr" => CaptureCommand::Ocr,
        "scrolling" => CaptureCommand::Scrolling,
        "favorite" => CaptureCommand::Favorite,
        "pin" => CaptureCommand::Pin,
        "save" => CaptureCommand::Save,
        "copy" => CaptureCommand::Copy,
        _ => unreachable!("Unknown native toolbar action: {name}"),
    })
}
