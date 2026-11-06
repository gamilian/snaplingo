#[cfg_attr(not(test), allow(dead_code))]
mod port;
#[cfg_attr(not(test), allow(dead_code))]
mod runtime;
#[cfg(any(target_os = "macos", test))]
mod session;
#[cfg(any(target_os = "macos", test))]
mod text;

#[cfg(test)]
mod tests;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use port::{
    ResultWindowMode, ResultWindowNotifierPort, ResultWindowOcrIntent, ResultWindowOpenRequest,
    ResultWindowOrigin, ResultWindowPayload, ResultWindowRequestId, ResultWindowWindowPort,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use runtime::ResultWindowRuntime;
#[cfg(any(target_os = "macos", test))]
pub(crate) use session::{
    resolved_target, AutoTranslationPlan, ResultWindowPresentationPort, ResultWindowSession,
    ResultWindowSnapshot, TranslationRow, TranslationStatus,
};
#[cfg(any(target_os = "macos", test))]
pub(crate) use text::{copy_tokens, normalize_ocr_text, OcrCopyToken, TokenCandidate};
