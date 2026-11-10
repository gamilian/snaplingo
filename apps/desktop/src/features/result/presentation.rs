//! Convert workflow values into portable presentation data.
use super::controller::Action;
use snaplingo_core::result::resolved_target;
use snaplingo_core::result::ResultWindowMode;
use snaplingo_core::result::ResultWindowSnapshot;
use snaplingo_core::result::TranslationStatus;
use snaplingo_ui::result::{ResultIntent, ResultMode, ResultRow, ResultSnapshot, RowStatus};
use snaplingo_ui::Snapshot;

pub(super) fn project(snapshot: &ResultWindowSnapshot, shell: Snapshot) -> ResultSnapshot {
    ResultSnapshot {
        shell,
        revision: snapshot.revision,
        epoch: snapshot.epoch,
        visible: snapshot.visible,
        mode: if snapshot.mode == ResultWindowMode::Ocr {
            ResultMode::Ocr
        } else {
            ResultMode::Translation
        },
        source: snapshot.source_text.clone(),
        composing: snapshot.composing,
        source_language: snapshot.source_lang.clone(),
        target_language: snapshot.target_lang.clone(),
        detected_language: snapshot
            .rows
            .iter()
            .find_map(|row| row.result.detected_language.clone()),
        rows: snapshot
            .rows
            .iter()
            .map(|row| ResultRow {
                id: row.id.clone(),
                name: row.name.clone(),
                text: row.result.translated_text.clone(),
                status: match row.status {
                    TranslationStatus::Pending => RowStatus::Pending,
                    TranslationStatus::Success => RowStatus::Success,
                    TranslationStatus::Error => RowStatus::Error,
                },
                error: row.result.error.as_ref().map(|error| error.message.clone()),
                favorite: row.favorite,
                retryable: row
                    .result
                    .error
                    .as_ref()
                    .is_none_or(|error| error.retryable),
            })
            .collect(),
        translating: snapshot.translating,
        ocr_text: snapshot.ocr_text.clone(),
        ocr_tokens: snaplingo_core::result::copy_tokens(&snapshot.ocr_text, vec![])
            .into_iter()
            .map(|token| snaplingo_ui::result::ResultCopyToken {
                label: token.label,
                value: token.value,
            })
            .collect(),
        ocr_image: None,
        ocr_image_present: snapshot.ocr_image.is_some(),
        ocr_language: snapshot.ocr_language.clone(),
        ocr_confidence: snapshot
            .preferences
            .ocr
            .show_confidence
            .then_some(snapshot.ocr_confidence)
            .flatten(),
        ocr_running: snapshot.ocr_running,
        dialog_open: snapshot.dialog_open,
        favorite_pending: snapshot.favorite_pending,
        ocr_favorite: snapshot.ocr_favorite,
        error: snapshot.error.clone(),
    }
}

pub(super) fn action(intent: ResultIntent, snapshot: &ResultWindowSnapshot) -> Option<Action> {
    Some(match intent {
        ResultIntent::Edit(text, composing) => Action::Edit(text, composing),
        ResultIntent::SourceLanguage(code) => Action::SourceLanguage(code),
        ResultIntent::TargetLanguage(code) => Action::TargetLanguage(code),
        ResultIntent::Swap => Action::Swap,
        ResultIntent::Translate => Action::Translate,
        ResultIntent::Retry(id) => Action::Retry(id),
        ResultIntent::FileOcr => Action::FileOcr,
        ResultIntent::ClearOcr => Action::ClearOcr,
        ResultIntent::Copy(text) => Action::Copy(text),
        ResultIntent::Speak(text, language) => Action::Speak(
            text,
            language.map(|language| {
                if language == snapshot.target_lang {
                    resolved_target(
                        &snapshot.source_text,
                        &snapshot.source_lang,
                        &snapshot.target_lang,
                    )
                } else {
                    language
                }
            }),
        ),
        ResultIntent::Favorite(ids) => Action::Favorite(snapshot.clone(), ids),
        ResultIntent::Close => Action::Close,
        ResultIntent::ReadClipboard | ResultIntent::Drag => return None,
    })
}
