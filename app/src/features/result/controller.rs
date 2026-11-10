use base64::Engine;
use std::path::PathBuf;
use std::sync::{Arc, Weak};

use snaplingo_core::providers::ocr::model::OcrResult;
use snaplingo_core::providers::translation::model::TranslationRequest;
use snaplingo_core::result::resolved_target;
use snaplingo_core::result::AutoTranslationPlan;
use snaplingo_core::result::ResultWindowMode;
use snaplingo_core::result::ResultWindowRequestId;
use snaplingo_core::result::ResultWindowRuntime;
use snaplingo_core::result::ResultWindowSession;
use snaplingo_core::result::ResultWindowSnapshot;
use snaplingo_core::result::TranslationStatus;

#[derive(Clone, Debug)]
pub(crate) enum Action {
    Edit(String, bool),
    SourceLanguage(String),
    TargetLanguage(String),
    Swap,
    Translate,
    Retry(String),
    FileOcr,
    ClearOcr,
    Copy(String),
    Speak(String, Option<String>),
    Favorite(ResultWindowSnapshot, Vec<String>),
    Close,
    Moved(i32, i32),
}

#[async_trait::async_trait]
pub(crate) trait ResultWindowEffects: Send + Sync {
    async fn pick_image(&self) -> crate::Result<Option<PathBuf>>;
    fn copy_text(&self, text: String) -> crate::Result<()>;
}

#[derive(Clone)]
pub(crate) struct Services {
    pub(crate) executor: tokio::runtime::Handle,
    pub(crate) window: Weak<ResultWindowRuntime>,
    pub(crate) session: Arc<ResultWindowSession>,
    pub(crate) settings: Arc<snaplingo_core::settings::SettingsConfiguration>,
    pub(crate) ocr: Arc<snaplingo_core::providers::ocr::OcrCoordinator>,
    pub(crate) file_ocr: Arc<snaplingo_core::providers::ocr::FileOcr>,
    pub(crate) favorites: Arc<snaplingo_core::library::favorites::Favorites>,
    pub(crate) effects: Arc<dyn ResultWindowEffects>,
    pub(crate) tts: Arc<snaplingo_core::speech::TtsRuntime>,
}

/// Shared result workflows; a host supplies UI presentation and platform effects.
#[derive(Clone)]
pub(crate) struct ResultWindowController {
    services: Services,
}

impl ResultWindowController {
    pub(crate) fn new(services: Services) -> Self {
        Self { services }
    }

    pub(crate) fn snapshot(&self) -> ResultWindowSnapshot {
        self.services.session.snapshot()
    }

    pub(crate) fn receive_payload(&self, request_id: ResultWindowRequestId) -> Result<(), String> {
        receive_payload(&self.services, request_id)
    }

    pub(crate) fn settings_changed(&self) {
        if let Err(error) = self.services.session.refresh_settings() {
            log::warn!("Failed to refresh native result window settings: {error}");
        }
        schedule_auto(&self.services, false);
    }

    pub(crate) fn dispatch(&self, action: Action) {
        dispatch(self.services.clone(), action);
    }
}

fn receive_payload(services: &Services, request_id: ResultWindowRequestId) -> Result<(), String> {
    let window = services
        .window
        .upgrade()
        .ok_or("Result window runtime has stopped")?;
    let Some(payload) = window
        .take_if_current(request_id)
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    let immediate = payload.auto_translate;
    let file = services
        .session
        .apply_payload(payload)
        .map_err(|error| error.to_string())?;
    if file {
        dispatch(services.clone(), Action::FileOcr);
    } else {
        schedule_auto(services, immediate);
    }
    Ok(())
}

fn schedule_auto(services: &Services, immediate: bool) {
    let session = services.session.clone();
    let Some(plan) = session.auto_plan(immediate) else {
        return;
    };
    let services = services.clone();
    services.executor.clone().spawn(async move {
        tokio::select! {
            biased;
            _ = session.plan_cancelled(&plan) => return,
            _ = tokio::time::sleep(plan.delay) => {},
        }
        translate(&services, plan).await;
    });
}

async fn translate(services: &Services, plan: AutoTranslationPlan) {
    let session = services.session.clone();
    let result = session.translate_plan(plan).await;
    match result {
        Ok(Some(copy)) if session.auto_copy_is_current(&copy) => {
            if let Err(error) = services.effects.copy_text(copy.text) {
                log::warn!("Failed to auto-copy translation result: {error}");
            }
        }
        Err(error) => session.report_error(error.to_string()),
        _ => {}
    }
}

fn dispatch(services: Services, action: Action) {
    let session = services.session.clone();
    match action {
        Action::Edit(text, composing) => {
            session.edit_source(text, composing);
            schedule_auto(&services, false);
        }
        Action::SourceLanguage(language) => {
            session.source_language(language);
            schedule_auto(&services, false);
        }
        Action::TargetLanguage(language) => {
            session.target_language(language);
            schedule_auto(&services, false);
        }
        Action::Swap => {
            session.swap_languages();
            schedule_auto(&services, false);
        }
        Action::Close => session.close(),
        Action::ClearOcr => session.clear_ocr(),
        Action::Translate => {
            if let Some(plan) = session.auto_plan(true) {
                services.executor.clone().spawn(async move {
                    translate(&services, plan).await;
                });
            }
        }
        Action::FileOcr => {
            let epoch = session.begin_file_ocr();
            services.executor.clone().spawn(async move {
                if let Err(error) = file_ocr(&services, epoch).await {
                    session.finish_file_ocr(epoch, Err(error));
                }
            });
        }
        Action::Favorite(original, ids) => {
            if !session.is_current(original.epoch) || session.snapshot().favorite_pending {
                return;
            }
            session.favorite_pending(original.epoch, true);
            services.executor.clone().spawn(async move {
                let result = favorite(&services, &original, &ids).await;
                session.favorite_pending(original.epoch, false);
                if session.is_current(original.epoch) {
                    if let Err(error) = result {
                        session.report_error(error.to_string());
                    }
                }
            });
        }
        action => {
            let epoch = session.snapshot().epoch;
            services.executor.clone().spawn(async move {
                let result = match action {
                    Action::Retry(id) => {
                        session.retry_if_current(id, epoch).await;
                        Ok(())
                    }
                    Action::Copy(text) => services.effects.copy_text(text).map_err(Into::into),
                    Action::Speak(text, language) => {
                        services
                            .tts
                            .speak(&text, language.as_deref().map(speech_locale))
                            .await
                    }
                    Action::Moved(x, y) => {
                        services.settings.update_last_result_window_position(x, y)
                    }
                    _ => unreachable!("Synchronous result actions are handled before dispatch"),
                };
                if session.is_current(epoch) {
                    if let Err(error) = result {
                        session.report_error(error.to_string());
                    }
                }
            });
        }
    }
}

async fn file_ocr(services: &Services, epoch: u64) -> crate::Result<()> {
    let session = &services.session;
    let preferences = session.snapshot().preferences.ocr;
    let Some(path) = services.effects.pick_image().await? else {
        session.cancel_file_dialog(epoch);
        return Ok(());
    };
    if !session.ocr_started(epoch) {
        return Ok(());
    }
    let language =
        (preferences.recognition_language != "auto").then_some(preferences.recognition_language);
    let result = services
        .file_ocr
        .recognize(&path.to_string_lossy(), language)
        .await;
    session.finish_file_ocr(
        epoch,
        result.map(|result| (result.result, result.image_data_url)),
    );
    Ok(())
}

async fn favorite(
    services: &Services,
    original: &ResultWindowSnapshot,
    ids: &[String],
) -> crate::Result<()> {
    if original.mode == ResultWindowMode::Ocr {
        let image = original
            .ocr_image
            .as_deref()
            .map(|text| {
                base64::engine::general_purpose::STANDARD
                    .decode(text.split_once(',').map_or(text, |(_, data)| data))
            })
            .transpose()
            .map_err(|error| error.to_string())?
            .unwrap_or_default();
        let recognition_language = &original.preferences.ocr.recognition_language;
        let language = (recognition_language != "auto").then(|| recognition_language.clone());
        let provider = original
            .ocr_provider
            .clone()
            .or_else(|| {
                services
                    .ocr
                    .get_active()
                    .map(|provider| provider.read().id().to_owned())
            })
            .unwrap_or_else(|| "manual".into());
        services
            .favorites
            .add_ocr(
                None,
                image,
                language,
                provider,
                OcrResult::from_text(&original.ocr_text, original.ocr_confidence),
            )
            .await?;
        services.session.mark_favorite(original, &[]);
    } else {
        let request = TranslationRequest {
            text: original.source_text.clone(),
            source_lang: original.source_lang.clone(),
            target_lang: resolved_target(
                &original.source_text,
                &original.source_lang,
                &original.target_lang,
            ),
        };
        for row in &original.rows {
            if row.status == TranslationStatus::Success
                && ids.contains(&row.id)
                && !row.result.translated_text.is_empty()
            {
                services
                    .favorites
                    .add_translation(None, request.clone(), row.result.clone())
                    .await?;
                services
                    .session
                    .mark_favorite(original, std::slice::from_ref(&row.id));
            }
        }
    }
    Ok(())
}

fn speech_locale(code: &str) -> &str {
    match code {
        "zh" | "zh-CN" => "zh-CN",
        "zh-TW" => "zh-TW",
        "en" => "en-US",
        "ja" => "ja-JP",
        "ko" => "ko-KR",
        "es" => "es-ES",
        "fr" => "fr-FR",
        "de" => "de-DE",
        "ru" => "ru-RU",
        "ar" => "ar-SA",
        _ => code,
    }
}

#[cfg(test)]
mod tests;
