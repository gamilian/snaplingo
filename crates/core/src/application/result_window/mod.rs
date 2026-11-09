#[cfg_attr(not(test), allow(dead_code))]
mod port;
#[cfg_attr(not(test), allow(dead_code))]
mod runtime;
mod session;
mod text;

#[cfg(test)]
mod tests;

#[cfg_attr(not(test), allow(unused_imports))]
pub use port::{
    ResultWindowMode, ResultWindowNotifierPort, ResultWindowOcrIntent, ResultWindowOpenRequest,
    ResultWindowOrigin, ResultWindowPayload, ResultWindowRequestId, ResultWindowWindowPort,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub use runtime::ResultWindowRuntime;
pub use session::{
    resolved_target, AutoTranslationPlan, ResultWindowPresentationPort, ResultWindowSession,
    ResultWindowSnapshot, TranslationRow, TranslationStatus,
};
pub use text::{copy_tokens, normalize_ocr_text, OcrCopyToken, TokenCandidate};
