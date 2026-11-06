use base64::Engine;
use objc2_foundation::NSUserDefaults;
use tauri::{AppHandle, Listener, Manager};
use tauri_plugin_dialog::DialogExt;

use super::native_view::{self as native, Action};
use crate::application::result_window::{
    resolved_target, AutoTranslationPlan, ResultWindowMode, ResultWindowPresentationPort,
    ResultWindowRequestId, ResultWindowSnapshot, TranslationStatus,
};
use crate::domain::ocr::OcrResult;
use crate::domain::translation::TranslationRequest;

pub(crate) struct NativeResultWindowPresentation {
    app: AppHandle,
    settings_listener: tauri::EventId,
}
impl NativeResultWindowPresentation {
    pub(crate) fn new(app: AppHandle) -> Self {
        let event_app = app.clone();
        let settings_listener = app.listen("settings-changed", move |_| {
            let event_app = event_app.clone();
            let dispatch_app = event_app.clone();
            let _ = dispatch_app.run_on_main_thread(move || {
                let Some(state) = event_app.try_state::<crate::AppState>() else {
                    return;
                };
                if let Err(error) = state.result_session.refresh_settings() {
                    log::warn!("Failed to refresh native result window settings: {error}");
                }
                schedule_auto(&event_app, false);
            });
        });
        Self {
            app,
            settings_listener,
        }
    }
}
impl Drop for NativeResultWindowPresentation {
    fn drop(&mut self) {
        self.app.unlisten(self.settings_listener);
    }
}
impl ResultWindowPresentationPort for NativeResultWindowPresentation {
    fn publish(&self, view: ResultWindowSnapshot) {
        if let Err(error) = self.app.run_on_main_thread(move || native::update(view)) {
            log::warn!("Failed to publish native result window: {error}");
        }
    }
}

pub fn show_or_create_result_window(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let event_app = app.clone();
    native::open(
        state.result_session.snapshot(),
        NSUserDefaults::standardUserDefaults(),
        move |action| {
            dispatch(event_app.clone(), action);
        },
    )
}

pub(super) fn receive_payload(
    app: &AppHandle,
    request_id: ResultWindowRequestId,
) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let Some(payload) = state
        .result_window
        .take_if_current(request_id)
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    let immediate = payload.auto_translate;
    let file = state
        .result_session
        .apply_payload(payload)
        .map_err(|error| error.to_string())?;
    if file {
        dispatch(app.clone(), Action::FileOcr);
    } else {
        schedule_auto(app, immediate);
    }
    Ok(())
}

fn schedule_auto(app: &AppHandle, immediate: bool) {
    let session = app.state::<crate::AppState>().result_session.clone();
    let Some(plan) = session.auto_plan(immediate) else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::select! {
            biased;
            _ = session.plan_cancelled(&plan) => return,
            _ = tokio::time::sleep(plan.delay) => {},
        }
        translate(&app, plan).await;
    });
}

async fn translate(app: &AppHandle, plan: AutoTranslationPlan) {
    let session = app.state::<crate::AppState>().result_session.clone();
    let result = session.translate_plan(plan).await;
    match result {
        Ok(Some(copy)) if session.auto_copy_is_current(&copy) => {
            if let Err(error) = crate::infrastructure::system::clipboard_text::copy_text(copy.text)
            {
                log::warn!("Failed to auto-copy translation result: {error}");
            }
        }
        Err(error) => session.report_error(error.to_string()),
        _ => {}
    }
}

fn dispatch(app: AppHandle, action: Action) {
    let state = app.state::<crate::AppState>();
    let session = state.result_session.clone();
    match action {
        Action::Edit(text, composing) => {
            session.edit_source(text, composing);
            schedule_auto(&app, false);
        }
        Action::SourceLanguage(language) => {
            session.source_language(language);
            schedule_auto(&app, false);
        }
        Action::TargetLanguage(language) => {
            session.target_language(language);
            schedule_auto(&app, false);
        }
        Action::Swap => {
            session.swap_languages();
            schedule_auto(&app, false);
        }
        Action::Close => session.close(),
        Action::ClearOcr => session.clear_ocr(),
        Action::Translate => {
            if let Some(plan) = session.auto_plan(true) {
                tauri::async_runtime::spawn(async move {
                    translate(&app, plan).await;
                });
            }
        }
        Action::FileOcr => {
            let epoch = session.begin_file_ocr();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = file_ocr(&app, epoch).await {
                    session.finish_file_ocr(epoch, Err(error));
                }
            });
        }
        Action::Favorite(original, ids) => {
            if !session.is_current(original.epoch) || session.snapshot().favorite_pending {
                return;
            }
            session.favorite_pending(original.epoch, true);
            tauri::async_runtime::spawn(async move {
                let result = favorite(&app, &original, &ids).await;
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
            tauri::async_runtime::spawn(async move {
                let result = match action {
                    Action::Retry(id) => {
                        session.retry_if_current(id, epoch).await;
                        Ok(())
                    }
                    Action::Copy(text) => {
                        crate::infrastructure::system::clipboard_text::copy_text(text)
                            .map_err(Into::into)
                    }
                    Action::Speak(text, language) => {
                        app.state::<crate::AppState>()
                            .tts
                            .speak(&text, language.as_deref().map(speech_locale))
                            .await
                    }
                    Action::Moved(x, y) => app
                        .state::<crate::AppState>()
                        .settings
                        .configuration
                        .update_last_result_window_position(x, y),
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

async fn file_ocr(app: &AppHandle, epoch: u64) -> crate::Result<()> {
    let state = app.state::<crate::AppState>();
    let session = &state.result_session;
    let preferences = session.snapshot().preferences.ocr;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("选择图片进行 OCR")
        .add_filter(
            "图片",
            &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tiff"],
        )
        .pick_file(move |path| {
            let _ = sender.send(path);
        });
    let Some(path) = receiver.await.map_err(|error| error.to_string())? else {
        session.cancel_file_dialog(epoch);
        return Ok(());
    };
    if !session.ocr_started(epoch) {
        return Ok(());
    }
    let path = path.into_path().map_err(|error| error.to_string())?;
    let language =
        (preferences.recognition_language != "auto").then_some(preferences.recognition_language);
    let result = state
        .providers
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
    app: &AppHandle,
    original: &ResultWindowSnapshot,
    ids: &[String],
) -> crate::Result<()> {
    let state = app.state::<crate::AppState>();
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
                state
                    .providers
                    .ocr
                    .get_active()
                    .map(|provider| provider.read().id().to_owned())
            })
            .unwrap_or_else(|| "manual".into());
        state
            .favorites
            .favorites
            .add_ocr(
                None,
                image,
                language,
                provider,
                OcrResult::from_text(&original.ocr_text, original.ocr_confidence),
            )
            .await?;
        state.result_session.mark_favorite(original, &[]);
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
                state
                    .favorites
                    .favorites
                    .add_translation(None, request.clone(), row.result.clone())
                    .await?;
                state
                    .result_session
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
