use crate::domain::hotkey::HotkeyAction;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::domain::capture::CaptureRegion;
use crate::application::CaptureService;
use crate::application::providers::ocr::OcrCoordinator;
use crate::application::providers::translation::TranslationCoordinator;
use crate::Result;
use std::sync::Arc;

/// Outcome of a workflow execution.
///
/// This represents what should be shown to the user after a workflow completes.
#[derive(Debug, Clone)]
pub enum WorkflowOutcome {
    /// Show screenshot image
    ShowScreenshot {
        image_data: Vec<u8>,
    },

    /// Show OCR result
    ShowOcrResult {
        source_text: String,
        detected_language: Option<String>,
    },

    /// Show translation results
    ShowTranslationResults {
        source_text: String,
        source_lang: String,
        target_lang: String,
        translations: Vec<TranslationResult>,
    },

    /// Show input window (no immediate result)
    ShowInputWindow,
}

/// Service that orchestrates multi-step workflows triggered by hotkeys.
///
/// This service coordinates between capture, OCR, and translation services
/// to execute complete user workflows like "screenshot → OCR → translate".
pub struct WorkflowService {
    capture_service: Arc<CaptureService>,
    ocr_coordinator: Arc<OcrCoordinator>,
    translation_coordinator: Arc<TranslationCoordinator>,
}

impl WorkflowService {
    /// Creates a new WorkflowService.
    pub fn new(
        capture_service: Arc<CaptureService>,
        ocr_coordinator: Arc<OcrCoordinator>,
        translation_coordinator: Arc<TranslationCoordinator>,
    ) -> Self {
        Self {
            capture_service,
            ocr_coordinator,
            translation_coordinator,
        }
    }

    /// Executes a workflow based on the hotkey action.
    ///
    /// # Arguments
    ///
    /// * `action` - The workflow action to execute
    ///
    /// # Returns
    ///
    /// * `Result<WorkflowOutcome>` - The outcome to present to the user
    pub async fn execute(&self, action: HotkeyAction) -> Result<WorkflowOutcome> {
        match action {
            HotkeyAction::Screenshot => self.workflow_screenshot().await,
            HotkeyAction::ScreenshotOcr => self.workflow_screenshot_ocr().await,
            HotkeyAction::ScreenshotTranslate => self.workflow_screenshot_translate().await,
            HotkeyAction::SelectionTranslate => self.workflow_selection_translate().await,
            HotkeyAction::InputTranslate => self.workflow_input_translate().await,
        }
    }

    /// Workflow: Capture screenshot only
    async fn workflow_screenshot(&self) -> Result<WorkflowOutcome> {
        // TODO: Implement region selection UI
        // For now, capture full screen
        let image_data = self.capture_service.capture_full_screen().await?;

        Ok(WorkflowOutcome::ShowScreenshot { image_data })
    }

    /// Workflow: Screenshot → OCR
    async fn workflow_screenshot_ocr(&self) -> Result<WorkflowOutcome> {
        // Capture screenshot
        let image_data = self.capture_service.capture_full_screen().await?;

        // Perform OCR
        let ocr_request = OcrRequest {
            image_data: image_data.clone(),
            language: None, // Auto-detect
        };

        let ocr_result = self.ocr_coordinator.recognize(&ocr_request).await?;

        Ok(WorkflowOutcome::ShowOcrResult {
            source_text: ocr_result.text,
            detected_language: None, // OcrResult doesn't have language field
        })
    }

    /// Workflow: Screenshot → OCR → Translate
    async fn workflow_screenshot_translate(&self) -> Result<WorkflowOutcome> {
        // Capture screenshot
        let image_data = self.capture_service.capture_full_screen().await?;

        // Perform OCR
        let ocr_request = OcrRequest {
            image_data: image_data.clone(),
            language: None, // Auto-detect
        };

        let ocr_result = self.ocr_coordinator.recognize(&ocr_request).await?;

        // Translate the OCR result
        let translation_request = TranslationRequest {
            text: ocr_result.text.clone(),
            source_lang: "auto".to_string(), // Auto-detect
            target_lang: "zh-CN".to_string(), // TODO: Get from user preferences
        };

        let translations = self.translation_coordinator
            .translate(&translation_request)
            .await?;

        Ok(WorkflowOutcome::ShowTranslationResults {
            source_text: ocr_result.text,
            source_lang: "auto".to_string(), // No language detection info from OCR
            target_lang: "zh-CN".to_string(),
            translations,
        })
    }

    /// Workflow: Translate selected text
    async fn workflow_selection_translate(&self) -> Result<WorkflowOutcome> {
        // TODO: Get selected text from system clipboard or accessibility API
        Err(crate::AppError::Other(
            "Selection translate not yet implemented. Use Input Translate as workaround.".to_string()
        ))
    }

    /// Workflow: Show input window for manual translation
    async fn workflow_input_translate(&self) -> Result<WorkflowOutcome> {
        // This workflow just opens the input window
        Ok(WorkflowOutcome::ShowInputWindow)
    }
}
