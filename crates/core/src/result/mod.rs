mod port;
mod runtime;
mod session;
mod text;

#[cfg(test)]
mod tests;

pub use port::{
    ResultWindowMode, ResultWindowNotifierPort, ResultWindowOcrIntent, ResultWindowOpenRequest,
    ResultWindowOrigin, ResultWindowPayload, ResultWindowRequestId, ResultWindowWindowPort,
};
pub use runtime::ResultWindowRuntime;
pub use session::{
    resolved_target, AutoTranslationPlan, ResultWindowPresentationPort, ResultWindowSession,
    ResultWindowSnapshot, TranslationAutoCopy, TranslationRow, TranslationStatus,
};
pub use text::{copy_tokens, normalize_ocr_text, OcrCopyToken, TokenCandidate};
