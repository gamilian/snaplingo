use serde::{Deserialize, Serialize};

use super::CaptureSessionRuntime;
use crate::capture::model::CaptureSessionId;
use crate::capture::model::LogicalRect;
use crate::Result;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureEditorToolbarPresentation {
    pub frame: LogicalRect,
    pub ui_scale: f64,
    pub dark: bool,
    #[serde(default)]
    pub interface_theme: String,
    pub visible: bool,
    pub active_tool: Option<String>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub busy: bool,
    pub styling: CaptureEditorStylePresentation,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureAnnotationStyle {
    pub color: [u8; 4],
    pub stroke_width: u32,
    pub filled: bool,
    #[serde(default)]
    pub corner_radius: f64,
    #[serde(default = "default_blur_radius")]
    pub blur_radius: f64,
    #[serde(default)]
    pub font_family: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
}

fn default_blur_radius() -> f64 {
    16.0
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureEditorStylePresentation {
    pub style: CaptureAnnotationStyle,
    pub font_size: u32,
    pub text_mode: bool,
    pub fill_enabled: bool,
    pub round_enabled: bool,
    pub blur_mode: bool,
    pub color_presets: Vec<[u8; 4]>,
    pub font_families: Vec<String>,
    pub error: Option<String>,
}

impl Default for CaptureEditorStylePresentation {
    fn default() -> Self {
        Self {
            style: CaptureAnnotationStyle {
                color: [255, 77, 79, 255],
                stroke_width: 2,
                filled: false,
                corner_radius: 0.0,
                blur_radius: 16.0,
                font_family: String::new(),
                bold: false,
                italic: false,
            },
            font_size: 24,
            text_mode: false,
            fill_enabled: true,
            round_enabled: true,
            blur_mode: false,
            color_presets: Vec::new(),
            font_families: Vec::new(),
            error: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "property", content = "value", rename_all = "kebab-case")]
pub enum CaptureEditorStyleChange {
    Color([u8; 4]),
    StrokeWidth(u32),
    FontSize(u32),
    Filled(bool),
    CornerRadius(f64),
    BlurRadius(f64),
    FontFamily(String),
    Bold(bool),
    Italic(bool),
}

#[derive(Clone, Debug)]
pub enum CaptureEditorControlAction {
    Command(&'static str),
    Style {
        change: CaptureEditorStyleChange,
        commit: bool,
    },
    Presets(Vec<[u8; 4]>),
}

impl CaptureEditorToolbarPresentation {
    fn validate(&self) -> Result<()> {
        let frame = &self.frame;
        if ![frame.x, frame.y, frame.width, frame.height, self.ui_scale]
            .iter()
            .all(|value| value.is_finite())
            || frame.width <= 0.0
            || frame.height <= 0.0
            || self.ui_scale <= 0.0
        {
            return Err("Invalid capture editor toolbar geometry".into());
        }
        if self.active_tool.as_ref().is_some_and(|tool| {
            tool == "scrolling"
                || !crate::settings::model::default_annotation_tool_shortcuts().contains_key(tool)
        }) {
            return Err("Unknown annotation tool".into());
        }
        let style = &self.styling.style;
        if !(1..=32).contains(&style.stroke_width)
            || self.styling.font_size == 0
            || !style.corner_radius.is_finite()
            || !(0.0..=100.0).contains(&style.corner_radius)
            || !style.blur_radius.is_finite()
            || !(4.0..=64.0).contains(&style.blur_radius)
        {
            return Err("Invalid annotation style".into());
        }
        Ok(())
    }
}

impl CaptureSessionRuntime {
    pub async fn capture_editor_owns_focus(&self, session_id: &CaptureSessionId) -> Result<bool> {
        let generation = *self.ocr_generation.borrow();
        if self.presentations.lock().await.get(session_id) != Some(&generation) {
            return Ok(false);
        }
        self.host.capture_editor_owns_focus(session_id).await
    }

    pub async fn update_capture_editor_toolbar(
        &self,
        session_id: &CaptureSessionId,
        presentation: Option<CaptureEditorToolbarPresentation>,
    ) -> Result<bool> {
        if let Some(presentation) = &presentation {
            presentation.validate()?;
            if self.presentations.lock().await.get(session_id)
                != Some(&*self.ocr_generation.borrow())
            {
                return Ok(false);
            }
            self.sessions.window_geometry(session_id)?;
        }
        self.host
            .update_capture_editor_toolbar(session_id, presentation.as_ref())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn presentation() -> CaptureEditorToolbarPresentation {
        CaptureEditorToolbarPresentation {
            frame: LogicalRect {
                x: 10.0,
                y: 20.0,
                width: 300.0,
                height: 64.0,
            },
            ui_scale: 1.0,
            dark: false,
            interface_theme: "native".into(),
            visible: true,
            active_tool: Some("rectangle".into()),
            can_undo: false,
            can_redo: false,
            busy: false,
            styling: CaptureEditorStylePresentation::default(),
        }
    }

    #[test]
    fn toolbar_rejects_invalid_frame_and_scale() {
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let mut state = presentation();
            state.frame.width = value;
            assert!(state.validate().is_err());
            state = presentation();
            state.ui_scale = value;
            assert!(state.validate().is_err());
        }
    }

    #[test]
    fn toolbar_accepts_move_and_rejects_non_annotation_tools() {
        let mut state = presentation();
        assert!(state.validate().is_ok());
        state.active_tool = None;
        assert!(state.validate().is_ok());
        for tool in ["scrolling", "unknown"] {
            state.active_tool = Some(tool.into());
            assert!(state.validate().is_err());
        }
    }

    #[test]
    fn toolbar_rejects_invalid_style_values_without_changing_valid_style() {
        let mut state = presentation();
        state.styling.style.stroke_width = 0;
        assert!(state.validate().is_err());
        state = presentation();
        state.styling.font_size = 0;
        assert!(state.validate().is_err());
        state = presentation();
        state.styling.style.blur_radius = f64::INFINITY;
        assert!(state.validate().is_err());
        state = presentation();
        state.styling.style.corner_radius = -1.0;
        assert!(state.validate().is_err());
        assert!(presentation().validate().is_ok());
    }
}
