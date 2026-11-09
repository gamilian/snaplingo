use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::text::normalize_ocr_text;
use super::{ResultWindowMode, ResultWindowOcrIntent, ResultWindowOrigin, ResultWindowPayload};
use crate::application::history::History;
use crate::application::providers::translation::TranslationCoordinator;
use crate::application::settings::SettingsConfiguration;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::domain::SettingsSnapshot;
use crate::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TranslationStatus {
    Pending,
    Success,
    Error,
}

#[derive(Clone, Debug)]
pub struct TranslationRow {
    pub id: String,
    pub name: String,
    pub status: TranslationStatus,
    pub result: TranslationResult,
    pub favorite: bool,
}

#[derive(Clone, Debug)]
pub struct ResultWindowSnapshot {
    pub revision: u64,
    pub epoch: u64,
    pub presentation: u64,
    pub visible: bool,
    pub mode: ResultWindowMode,
    pub origin: ResultWindowOrigin,
    pub source_text: String,
    pub composing: bool,
    pub source_lang: String,
    pub target_lang: String,
    pub rows: Vec<TranslationRow>,
    pub translating: bool,
    pub ocr_text: String,
    pub ocr_image: Option<Arc<str>>,
    pub ocr_confidence: Option<f32>,
    pub ocr_language: Option<String>,
    pub ocr_provider: Option<String>,
    pub ocr_running: bool,
    pub dialog_open: bool,
    pub favorite_pending: bool,
    pub ocr_favorite: bool,
    pub error: Option<String>,
    pub preferences: SettingsSnapshot,
}

pub trait ResultWindowPresentationPort: Send + Sync {
    fn publish(&self, view: ResultWindowSnapshot);
}

pub struct AutoTranslationPlan {
    epoch: u64,
    pub delay: Duration,
}

pub struct TranslationAutoCopy {
    pub epoch: u64,
    pub text: String,
    provider_id: String,
    token: u64,
}

struct State {
    view: ResultWindowSnapshot,
    epoch: u64,
    requests: HashMap<String, u64>,
    last_key: Option<(String, String, String)>,
    request: Option<TranslationRequest>,
}

/// Owns result input, request validity and translation scheduling policy.
/// Native views supply presentation; provider execution and history reuse the application services.
pub struct ResultWindowSession {
    state: Mutex<State>,
    settings: Arc<SettingsConfiguration>,
    translation: Arc<TranslationCoordinator>,
    history: Arc<History>,
    presentation: Arc<dyn ResultWindowPresentationPort>,
    cancellation: tokio::sync::watch::Sender<u64>,
}

impl ResultWindowSession {
    pub fn new(
        settings: Arc<SettingsConfiguration>,
        translation: Arc<TranslationCoordinator>,
        history: Arc<History>,
        presentation: Arc<dyn ResultWindowPresentationPort>,
    ) -> Result<Self> {
        let preferences = settings.snapshot()?;
        let source_lang = preferences.translation.default_source_lang.clone();
        let target_lang = preferences.translation.default_target_lang.clone();
        Ok(Self {
            state: Mutex::new(State {
                view: ResultWindowSnapshot {
                    revision: 0,
                    epoch: 0,
                    presentation: 0,
                    visible: false,
                    mode: ResultWindowMode::Translation,
                    origin: ResultWindowOrigin::Input,
                    source_text: String::new(),
                    composing: false,
                    source_lang,
                    target_lang,
                    rows: vec![],
                    translating: false,
                    ocr_text: String::new(),
                    ocr_image: None,
                    ocr_confidence: None,
                    ocr_language: None,
                    ocr_provider: None,
                    ocr_running: false,
                    dialog_open: false,
                    favorite_pending: false,
                    ocr_favorite: false,
                    error: None,
                    preferences,
                },
                epoch: 0,
                requests: HashMap::new(),
                last_key: None,
                request: None,
            }),
            settings,
            translation,
            history,
            presentation,
            cancellation: tokio::sync::watch::channel(0).0,
        })
    }

    pub fn snapshot(&self) -> ResultWindowSnapshot {
        self.state.lock().unwrap().view.clone()
    }

    fn mutate<T>(&self, change: impl FnOnce(&mut State) -> T) -> T {
        let (result, view) = {
            let mut state = self.state.lock().unwrap();
            let previous_epoch = state.epoch;
            let result = change(&mut state);
            state.view.revision += 1;
            state.view.epoch = state.epoch;
            if state.epoch != previous_epoch {
                self.cancellation.send_replace(state.epoch);
            }
            (result, state.view.clone())
        };
        self.presentation.publish(view);
        result
    }

    pub fn apply_payload(&self, payload: ResultWindowPayload) -> Result<bool> {
        let preferences = self.settings.snapshot()?;
        let file = payload.mode == ResultWindowMode::Ocr
            && payload.ocr_intent == Some(ResultWindowOcrIntent::File);
        self.mutate(|state| {
            invalidate(state);
            state.view.preferences = preferences;
            state.view.presentation += 1;
            state.view.visible = true;
            state.view.mode = payload.mode.clone();
            state.view.origin = payload.origin.clone();
            state.view.error = None;
            if payload.mode == ResultWindowMode::Translation {
                if payload.auto_translate {
                    state.view.rows.clear();
                    state.last_key = None;
                }
                if payload.auto_translate || !payload.text.is_empty() {
                    state.view.source_text = if payload.auto_translate {
                        normalize_ocr_text(
                            &payload.text,
                            (payload.origin == ResultWindowOrigin::Screenshot)
                                .then_some(&state.view.preferences.ocr),
                        )
                    } else {
                        payload.text
                    };
                    if let Some(language) = payload.detected_language {
                        state.view.source_lang = language;
                    }
                }
                if !state.view.rows.is_empty() {
                    state.request = Some(request_for(&state.view));
                }
            } else if matches!(
                payload.ocr_intent,
                Some(ResultWindowOcrIntent::DisplayText | ResultWindowOcrIntent::File)
            ) {
                state.view.ocr_text =
                    normalize_ocr_text(&payload.text, Some(&state.view.preferences.ocr));
                state.view.ocr_image = payload.image_base64.map(Arc::from);
                state.view.ocr_confidence = payload.confidence;
                state.view.ocr_language = payload.detected_language;
                state.view.ocr_provider = None;
                state.view.ocr_favorite = false;
            }
        });
        Ok(file)
    }

    pub fn refresh_settings(&self) -> Result<()> {
        let preferences = self.settings.snapshot()?;
        self.mutate(|state| {
            let previous = &state.view.preferences.translation;
            let next = &preferences.translation;
            let defaults_changed = previous.default_source_lang != next.default_source_lang
                || previous.default_target_lang != next.default_target_lang;
            if defaults_changed
                || previous.auto_translate != next.auto_translate
                || previous.incremental_translation != next.incremental_translation
                || previous.preserve_line_breaks != next.preserve_line_breaks
            {
                invalidate(state);
            }
            if defaults_changed {
                state.view.source_lang = preferences.translation.default_source_lang.clone();
                state.view.target_lang = preferences.translation.default_target_lang.clone();
            }
            state.view.preferences = preferences;
        });
        Ok(())
    }

    pub fn edit_source(&self, text: String, composing: bool) {
        self.mutate(|state| {
            if state.view.source_text != text || state.view.composing != composing {
                invalidate(state);
                state.view.source_text = text;
                state.view.composing = composing;
                state
                    .view
                    .rows
                    .iter_mut()
                    .for_each(|row| row.favorite = false);
            }
        });
    }

    pub fn source_language(&self, language: String) {
        self.mutate(|state| {
            invalidate(state);
            if state.view.target_lang != "auto" {
                state.view.target_lang = default_target(&language).into();
            }
            state.view.source_lang = language;
            state
                .view
                .rows
                .iter_mut()
                .for_each(|row| row.favorite = false);
        });
    }

    pub fn target_language(&self, language: String) {
        self.mutate(|state| {
            invalidate(state);
            state.view.target_lang = language;
            state
                .view
                .rows
                .iter_mut()
                .for_each(|row| row.favorite = false);
        });
    }

    pub fn swap_languages(&self) {
        self.mutate(|state| {
            invalidate(state);
            let previous = state.view.source_lang.clone();
            state.view.source_lang = state.view.target_lang.clone();
            state.view.target_lang = if previous == "auto" {
                default_target(&state.view.source_lang).into()
            } else {
                previous
            };
            state
                .view
                .rows
                .iter_mut()
                .for_each(|row| row.favorite = false);
        });
    }

    pub fn auto_plan(&self, immediate: bool) -> Option<AutoTranslationPlan> {
        let state = self.state.lock().unwrap();
        let view = &state.view;
        if !view.visible
            || view.mode != ResultWindowMode::Translation
            || view.source_text.trim().is_empty()
            || view.translating
            || state.view.composing
            || (!immediate && !view.preferences.translation.auto_translate)
            || (!immediate && state.last_key.as_ref() == Some(&translation_key(view)))
        {
            return None;
        }
        Some(AutoTranslationPlan {
            epoch: state.epoch,
            delay: Duration::from_millis(if immediate {
                0
            } else if view.preferences.translation.incremental_translation {
                150
            } else {
                500
            }),
        })
    }

    pub fn plan_is_current(&self, plan: &AutoTranslationPlan) -> bool {
        let state = self.state.lock().unwrap();
        state.epoch == plan.epoch
            && state.view.visible
            && !state.view.translating
            && !state.view.composing
            && state.view.mode == ResultWindowMode::Translation
            && !state.view.source_text.trim().is_empty()
    }

    pub async fn plan_cancelled(&self, plan: &AutoTranslationPlan) {
        let mut cancellation = self.cancellation.subscribe();
        if self.plan_is_current(plan) {
            let _ = cancellation.changed().await;
        }
    }

    pub fn is_current(&self, epoch: u64) -> bool {
        self.state.lock().unwrap().epoch == epoch
    }

    pub fn auto_copy_is_current(&self, copy: &TranslationAutoCopy) -> bool {
        let state = self.state.lock().unwrap();
        state.epoch == copy.epoch && state.requests.get(&copy.provider_id) == Some(&copy.token)
    }

    #[cfg(test)]
    pub async fn translate(self: &Arc<Self>) -> Result<Option<TranslationAutoCopy>> {
        self.translate_current(None).await
    }

    pub async fn translate_plan(
        self: &Arc<Self>,
        plan: AutoTranslationPlan,
    ) -> Result<Option<TranslationAutoCopy>> {
        self.translate_current(Some(plan.epoch)).await
    }

    async fn translate_current(
        self: &Arc<Self>,
        expected_epoch: Option<u64>,
    ) -> Result<Option<TranslationAutoCopy>> {
        let providers = self
            .translation
            .get_active()
            .iter()
            .map(|provider| {
                let provider = provider.read();
                (provider.id().to_owned(), provider.name().to_owned())
            })
            .collect::<Vec<_>>();
        let Some((epoch, original, request, auto_copy)) = self.mutate(|state| {
            if state.view.source_text.trim().is_empty()
                || !state.view.visible
                || state.view.composing
                || state.view.mode != ResultWindowMode::Translation
                || expected_epoch.is_some_and(|epoch| state.epoch != epoch)
            {
                return None;
            }
            invalidate(state);
            state.last_key = Some(translation_key(&state.view));
            let request = request_for(&state.view);
            let original = TranslationRequest {
                text: state.view.source_text.clone(),
                ..request.clone()
            };
            state.request = Some(request.clone());
            state.view.error = None;
            state.view.mode = ResultWindowMode::Translation;
            state.view.rows = providers
                .iter()
                .map(|(id, name)| TranslationRow {
                    id: id.clone(),
                    name: name.clone(),
                    status: TranslationStatus::Pending,
                    result: TranslationResult::success(id, "", None, None),
                    favorite: false,
                })
                .collect();
            state.view.translating = !providers.is_empty();
            state.view.error = providers
                .is_empty()
                .then(|| "请先在设置中启用翻译服务。".into());
            Some((
                state.epoch,
                original,
                request,
                state.view.preferences.translation.auto_copy,
            ))
        }) else {
            return Ok(None);
        };
        let started = Instant::now();
        let mut tasks = tokio::task::JoinSet::new();
        for (id, _) in providers {
            let session = self.clone();
            let request = request.clone();
            tasks.spawn(async move { session.provider_request(epoch, id, request).await });
        }
        while let Some(result) = tasks.join_next().await {
            if let Err(error) = result {
                if self.is_current(epoch) {
                    self.report_error(error.to_string());
                }
            }
        }
        let results = {
            let state = self.state.lock().unwrap();
            if state.epoch != epoch {
                return Ok(None);
            }
            // A retry may have replaced an earlier response while other providers were pending.
            state
                .view
                .rows
                .iter()
                .filter(|row| row.status == TranslationStatus::Success)
                .map(|row| row.result.clone())
                .collect::<Vec<_>>()
        };
        if results.is_empty() {
            return Ok(None);
        }
        if let Err(error) = self
            .history
            .record_translation(
                original,
                results.clone(),
                started.elapsed().as_millis() as u64,
            )
            .await
        {
            log::warn!("Failed to record translation history: {error}");
        }
        let state = self.state.lock().unwrap();
        Ok(if auto_copy && state.epoch == epoch {
            results
                .iter()
                .find(|result| {
                    state.view.rows.iter().any(|row| {
                        row.status == TranslationStatus::Success && row.result == **result
                    })
                })
                .and_then(|result| {
                    state
                        .requests
                        .get(&result.provider_id)
                        .map(|token| TranslationAutoCopy {
                            epoch,
                            text: result.translated_text.clone(),
                            provider_id: result.provider_id.clone(),
                            token: *token,
                        })
                })
        } else {
            None
        })
    }

    async fn provider_request(
        &self,
        epoch: u64,
        id: String,
        request: TranslationRequest,
    ) -> Option<TranslationResult> {
        let mut cancellation = self.cancellation.subscribe();
        let token = self.mutate(|state| {
            if state.epoch != epoch {
                return None;
            }
            let token = state.requests.entry(id.clone()).or_insert(0);
            *token += 1;
            let token = *token;
            if let Some(row) = state.view.rows.iter_mut().find(|row| row.id == id) {
                row.status = TranslationStatus::Pending;
                row.favorite = false;
                state.view.translating = true;
            }
            Some(token)
        })?;
        let mut result = tokio::select! {
            biased;
            _ = cancellation.changed() => return None,
            result = self.translation.translate_with_provider(&id, &request) => result,
        }
        .unwrap_or_else(|error| {
            TranslationResult::failure(&id, "request_failed", error.to_string(), true)
        });
        result.provider_id = id.clone();
        result.translated_text = result.translated_text.trim().to_owned();
        let accepted = self.mutate(|state| {
            if state.epoch != epoch || state.requests.get(&id) != Some(&token) {
                return false;
            }
            if let Some(row) = state.view.rows.iter_mut().find(|row| row.id == id) {
                row.status = if result.error.is_some() {
                    TranslationStatus::Error
                } else {
                    TranslationStatus::Success
                };
                row.result = result.clone();
            }
            state.view.translating = state
                .view
                .rows
                .iter()
                .any(|row| row.status == TranslationStatus::Pending);
            true
        });
        (accepted && result.error.is_none()).then_some(result)
    }

    #[cfg(test)]
    pub async fn retry(&self, id: String) {
        let epoch = self.state.lock().unwrap().epoch;
        self.retry_if_current(id, epoch).await;
    }

    pub async fn retry_if_current(&self, id: String, epoch: u64) {
        let task = {
            let state = self.state.lock().unwrap();
            if state.epoch != epoch || !state.view.rows.iter().any(|row| row.id == id) {
                return;
            }
            state.request.clone().map(|request| (state.epoch, request))
        };
        if let Some((epoch, request)) = task {
            self.provider_request(epoch, id, request).await;
        }
    }

    pub fn begin_file_ocr(&self) -> u64 {
        self.mutate(|state| {
            invalidate(state);
            state.view.visible = true;
            state.view.mode = ResultWindowMode::Ocr;
            state.view.origin = ResultWindowOrigin::Ocr;
            state.view.ocr_text.clear();
            state.view.ocr_image = None;
            state.view.ocr_confidence = None;
            state.view.ocr_language = None;
            state.view.ocr_provider = None;
            state.view.ocr_favorite = false;
            state.view.error = None;
            state.view.dialog_open = true;
            state.epoch
        })
    }

    pub fn ocr_started(&self, epoch: u64) -> bool {
        self.mutate(|state| {
            if state.epoch != epoch {
                return false;
            }
            state.view.dialog_open = false;
            state.view.ocr_running = true;
            true
        })
    }

    pub fn finish_file_ocr(
        &self,
        epoch: u64,
        result: Result<(crate::domain::ocr::OcrResult, String)>,
    ) {
        self.mutate(|state| {
            if state.epoch != epoch {
                return;
            }
            state.view.dialog_open = false;
            state.view.ocr_running = false;
            match result {
                Ok((result, image)) => {
                    state.view.ocr_text =
                        normalize_ocr_text(&result.text, Some(&state.view.preferences.ocr));
                    state.view.ocr_confidence = result.confidence;
                    state.view.ocr_language = result.detected_language;
                    state.view.ocr_provider = result.provider_id;
                    state.view.ocr_image = Some(Arc::from(image));
                    state.view.error = None;
                }
                Err(error) => state.view.error = Some(error.to_string()),
            }
        });
    }

    pub fn cancel_file_dialog(&self, epoch: u64) {
        self.mutate(|state| {
            if state.epoch == epoch {
                state.view.dialog_open = false;
            }
        });
    }

    pub fn clear_ocr(&self) {
        self.mutate(|state| {
            invalidate(state);
            state.view.ocr_text.clear();
            state.view.ocr_image = None;
            state.view.ocr_confidence = None;
            state.view.ocr_language = None;
            state.view.ocr_provider = None;
            state.view.error = None;
            state.view.ocr_favorite = false;
        });
    }

    pub fn favorite_pending(&self, epoch: u64, pending: bool) {
        self.mutate(|state| {
            if state.epoch == epoch {
                state.view.favorite_pending = pending;
            }
        });
    }
    pub fn mark_favorite(&self, original: &ResultWindowSnapshot, ids: &[String]) {
        self.mutate(|state| {
            if state.epoch != original.epoch || state.view.mode != original.mode {
                return;
            }
            if state.view.mode == ResultWindowMode::Ocr {
                if state.view.ocr_text == original.ocr_text
                    && state.view.ocr_image == original.ocr_image
                {
                    state.view.ocr_favorite = true;
                }
            } else if state.view.source_text == original.source_text
                && state.view.source_lang == original.source_lang
                && state.view.target_lang == original.target_lang
            {
                for row in &mut state.view.rows {
                    if ids.contains(&row.id)
                        && original.rows.iter().any(|old| {
                            old.id == row.id
                                && old.status == TranslationStatus::Success
                                && row.status == TranslationStatus::Success
                                && old.result.translated_text == row.result.translated_text
                        })
                    {
                        row.favorite = true;
                    }
                }
            }
        });
    }
    pub fn report_error(&self, error: String) {
        self.mutate(|state| state.view.error = Some(error));
    }
    pub fn close(&self) {
        self.mutate(|state| {
            invalidate(state);
            state.view.visible = false;
        });
    }
}

fn invalidate(state: &mut State) {
    state.epoch += 1;
    state.requests.clear();
    state.request = None;
    if state.view.translating {
        state.last_key = None;
    }
    state.view.translating = false;
    state.view.ocr_running = false;
    state.view.dialog_open = false;
    state.view.favorite_pending = false;
    state.view.composing = false;
}

fn translation_key(view: &ResultWindowSnapshot) -> (String, String, String) {
    (
        view.source_lang.clone(),
        view.target_lang.clone(),
        view.source_text.clone(),
    )
}

fn chinese(language: &str) -> bool {
    matches!(language, "zh" | "zh-CN" | "zh-TW")
}
fn default_target(source: &str) -> &'static str {
    if chinese(source) {
        "en"
    } else {
        "zh-CN"
    }
}
pub fn resolved_target(text: &str, source: &str, target: &str) -> String {
    let source = if source == "auto" {
        if text
            .chars()
            .any(|c| matches!(c as u32, 0x3400..=0x9fff | 0xf900..=0xfaff))
        {
            "zh-CN"
        } else {
            "en"
        }
    } else {
        source
    };
    if target == "auto" || source == target || chinese(source) && chinese(target) {
        default_target(source).into()
    } else {
        target.into()
    }
}
fn request_for(view: &ResultWindowSnapshot) -> TranslationRequest {
    let text = if view.preferences.translation.preserve_line_breaks {
        view.source_text.clone()
    } else {
        view.source_text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    TranslationRequest {
        target_lang: resolved_target(&text, &view.source_lang, &view.target_lang),
        text,
        source_lang: view.source_lang.clone(),
    }
}
