use std::sync::Arc;

use crate::application::providers::ocr::OcrCoordinator;
use crate::application::services::{
    CaptureOutputService, CaptureSessionOutput, CaptureSessionService, ImageCompositionService,
};
use crate::domain::capture::{
    AnnotationCommand, CaptureOutputAction, CaptureSessionId, LogicalRect,
};
use crate::domain::ocr::OcrResult;
use crate::Result;

/// Coordinates Capture Session operations that need several application services.
pub struct CaptureSessionRuntime {
    sessions: Arc<CaptureSessionService>,
    image_composition: Arc<ImageCompositionService>,
    output: Arc<CaptureOutputService>,
    ocr: Arc<OcrCoordinator>,
}

impl CaptureSessionRuntime {
    pub fn new(
        sessions: Arc<CaptureSessionService>,
        image_composition: Arc<ImageCompositionService>,
        output: Arc<CaptureOutputService>,
        ocr: Arc<OcrCoordinator>,
    ) -> Self {
        Self {
            sessions,
            image_composition,
            output,
            ocr,
        }
    }

    pub fn render_png_base64(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        include_cursor: bool,
    ) -> Result<String> {
        self.sessions.render_png_base64(
            &self.image_composition,
            session_id,
            rect,
            annotations,
            include_cursor,
        )
    }

    pub async fn recognize_selection_text(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
    ) -> Result<OcrResult> {
        self.sessions
            .recognize_selection_text(&self.image_composition, &self.ocr, session_id, rect)
            .await
    }

    pub async fn output_selection(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        include_cursor: bool,
        action: CaptureOutputAction,
    ) -> Result<CaptureSessionOutput> {
        self.sessions
            .output_selection(
                &self.image_composition,
                &self.output,
                session_id,
                rect,
                annotations,
                include_cursor,
                action,
            )
            .await
    }
}
