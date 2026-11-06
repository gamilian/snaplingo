use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use base64::Engine;
use objc2::AnyThread;
use objc2_foundation::{NSString, NSUserDefaults};
use tauri::{Emitter, Manager};

use crate::application::favorites::{FavoriteContent, FavoriteKind, FavoriteQuery};
use crate::application::providers::common::Provider;
use crate::application::providers::translation::TranslationProvider;
use crate::application::result_window::{
    ResultWindowMode, ResultWindowOpenRequest, ResultWindowOrigin, ResultWindowSnapshot,
    TranslationRow, TranslationStatus,
};
use crate::application::{SystemTtsHost, SystemTtsVoice, TtsRuntime};
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::fixture::{on_main, png};
use crate::infrastructure::system::result_window::native_view::{self as native, Action};
use crate::{AppState, Result};

#[derive(Default)]
pub(crate) struct Services {
    requests: Mutex<Vec<TranslationRequest>>,
    speech: Mutex<Vec<(String, String)>>,
}
struct Translator(Arc<Services>);
impl Provider for Translator {
    fn id(&self) -> &str {
        "native-result-fixture"
    }
    fn name(&self) -> &str {
        "本地测试服务"
    }
    fn is_configured(&self) -> bool {
        true
    }
    fn requires_api_key(&self) -> bool {
        false
    }
}
#[async_trait]
impl TranslationProvider for Translator {
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult> {
        self.0.requests.lock().unwrap().push(request.clone());
        tokio::time::sleep(Duration::from_millis(20)).await;
        Ok(TranslationResult::success(
            self.id(),
            format!("原生译文：{}", request.text),
            Some("en".into()),
            None,
        ))
    }
}
#[async_trait]
impl SystemTtsHost for Services {
    async fn list_voices(&self) -> Result<Vec<SystemTtsVoice>> {
        Ok(vec![SystemTtsVoice {
            id: "fixture-en".into(),
            name: "English".into(),
            locale: "en-US".into(),
        }])
    }
    async fn speak(&self, text: &str, voice: &str, _rate: u16) -> Result<()> {
        self.speech
            .lock()
            .unwrap()
            .push((text.into(), voice.into()));
        Ok(())
    }
}
pub(crate) fn configure(state: &mut AppState) -> Arc<Services> {
    let services = Arc::new(Services::default());
    let translation = &state.providers.translation;
    let ids = translation
        .get_active()
        .iter()
        .map(|provider| provider.read().id().to_owned())
        .collect::<Vec<_>>();
    for id in ids {
        translation.deactivate(&id).unwrap();
    }
    translation.register(Translator(services.clone())).unwrap();
    translation.activate("native-result-fixture").unwrap();
    let mut settings = state.settings.configuration.snapshot().unwrap().translation;
    settings.default_source_lang = "en".into();
    settings.default_target_lang = "zh-CN".into();
    settings.auto_translate = false;
    settings.auto_copy = false;
    settings.hide_on_blur = false;
    state
        .settings
        .configuration
        .update_translation(settings)
        .unwrap();
    state.result_session.refresh_settings().unwrap();
    state.tts = Arc::new(TtsRuntime::new(
        services.clone(),
        state.settings.configuration.clone(),
    ));
    services
}

struct WebviewBenchmark(tauri::AppHandle);
#[async_trait]
impl crate::application::result_window::ResultWindowWindowPort for WebviewBenchmark {
    async fn show_or_create(&self) -> Result<()> {
        Ok(())
    }
}
#[async_trait]
impl crate::application::result_window::ResultWindowNotifierPort for WebviewBenchmark {
    async fn notify_payload_ready(
        &self,
        id: crate::application::result_window::ResultWindowRequestId,
    ) -> Result<()> {
        self.0
            .emit_to(
                "capture-result",
                "capture-result-payload-ready",
                serde_json::json!({ "requestId": id.0.to_string() }),
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
pub(crate) fn prepare_benchmark(app: &tauri::AppHandle, state: &mut AppState, renderer: &str) {
    if renderer == "webview" {
        let host = Arc::new(WebviewBenchmark(app.clone()));
        state.result_window = Arc::new(
            crate::application::result_window::ResultWindowRuntime::new(host.clone(), host),
        );
    }
}
pub(crate) fn start_benchmark(app: &tauri::AppHandle, renderer: &str) {
    if renderer == "webview" {
        tauri::WebviewWindowBuilder::new(
            app,
            "capture-result",
            tauri::WebviewUrl::App("index.html?window=capture-result".into()),
        )
        .title("SnapLingo Result benchmark")
        .inner_size(660.0, 400.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .focused(true)
        .build()
        .unwrap();
    }
    let app = app.clone();
    let native = renderer == "native";
    std::thread::spawn(move || {
        tauri::async_runtime::block_on(async {
            let state = app.state::<AppState>();
            state
                .result_window
                .open(ResultWindowOpenRequest::selection_translation(
                    "native result benchmark".into(),
                ))
                .await
                .unwrap();
            if native {
                until(
                    || {
                        state
                            .result_session
                            .snapshot()
                            .rows
                            .first()
                            .is_some_and(|row| row.status == TranslationStatus::Success)
                    },
                    "Native result benchmark did not finish loading",
                )
                .await;
                let view = on_main(&app, native::test_state).await;
                println!(
                    "{}",
                    serde_json::json!({ "sourceText": view.source, "resultText": view.results[0], "resultCards": view.results.len() })
                );
            }
        });
    });
}

fn snapshot(name: &str) {
    let bytes = native::test_snapshot();
    let image = image::load_from_memory(&bytes).unwrap();
    assert!(image.width() >= 300 && image.height() >= 140);
    if let Ok(directory) = std::env::var("SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(std::path::Path::new(&directory).join(name), bytes).unwrap();
    }
}

fn controls(mut view: ResultWindowSnapshot) {
    let actions = Arc::new(Mutex::new(Vec::new()));
    let dispatch = actions.clone();
    let suite = NSString::from_str(&format!(
        "com.snaplingo.result-ui-test-{}",
        std::process::id()
    ));
    let defaults =
        NSUserDefaults::initWithSuiteName(NSUserDefaults::alloc(), Some(&suite)).unwrap();
    view.visible = true;
    view.presentation = 1;
    view.origin = ResultWindowOrigin::Selection;
    view.source_text = "Hello from the native result window.\n第二行原文".into();
    view.preferences.general.theme = "light".into();
    view.preferences.translation.selection_input_state = "expanded".into();
    view.rows = vec![
        TranslationRow {
            id: "google".into(),
            name: "Google Translate".into(),
            status: TranslationStatus::Success,
            result: TranslationResult::success(
                "google",
                "来自原生结果窗口的问候。\n第二行译文",
                Some("en".into()),
                None,
            ),
            favorite: false,
        },
        TranslationRow {
            id: "deeplx".into(),
            name: "DeepL".into(),
            status: TranslationStatus::Pending,
            result: TranslationResult::success("deeplx", "", None, None),
            favorite: false,
        },
        TranslationRow {
            id: "failed".into(),
            name: "失败的服务".into(),
            status: TranslationStatus::Error,
            result: TranslationResult::failure(
                "failed",
                "configuration",
                "测试错误：未配置翻译服务",
                false,
            ),
            favorite: false,
        },
    ];
    native::open(view.clone(), defaults.clone(), move |action| {
        dispatch.lock().unwrap().push(action)
    })
    .unwrap();
    let state = native::test_state();
    assert!(state.visible);
    assert_eq!(state.frame.size.width, 660.0);
    assert_eq!(state.level, objc2_app_kit::NSFloatingWindowLevel);
    assert!(
        state.level < objc2_app_kit::NSModalPanelWindowLevel,
        "Native menus and file dialogs must appear above the result"
    );
    assert_eq!(state.source, view.source_text);
    assert!(state.results[0].contains("第二行译文"));
    assert!(state.results[1].contains("正在请求"));
    assert!(state.results[2].contains("未配置"));
    snapshot("result-translation.png");
    view.source_text = "line one\nline two\nline three".into();
    view.revision += 1;
    native::update(view.clone());
    let source_height = native::test_state().source_height;
    view.source_text.push('\n');
    view.revision += 1;
    native::update(view.clone());
    assert!(
        native::test_state().source_height > source_height,
        "The trailing insertion line must increase the editor height"
    );
    native::test_click(0);
    assert!(native::test_state().pinned);
    native::test_click(0);
    native::test_click(3);
    assert!(
        matches!(actions.lock().unwrap().last(), Some(Action::Copy(text)) if text == &view.source_text)
    );
    native::test_click(4);
    assert!(
        matches!(actions.lock().unwrap().last(), Some(Action::Speak(text, Some(language))) if text == &view.source_text && language == "en")
    );
    native::test_click(7);
    assert!(
        matches!(actions.lock().unwrap().last(), Some(Action::Favorite(_, ids)) if ids == &vec!["google".to_string()])
    );
    native::test_click(101);
    assert!(matches!(actions.lock().unwrap().last(), Some(Action::Retry(id)) if id == "google"));
    let count = actions.lock().unwrap().len();
    native::test_click(113);
    native::test_click(121);
    assert_eq!(
        actions.lock().unwrap().len(),
        count,
        "Pending favorite and permanent error retry are disabled"
    );
    let height = native::test_state().frame.size.height;
    native::test_click(100);
    assert!(native::test_state().frame.size.height < height);
    native::test_click(100);
    native::test_click(2);
    assert!(native::test_state().collapsed);
    view.presentation += 1;
    view.revision += 1;
    view.preferences.translation.selection_input_state = "last".into();
    native::update(view.clone());
    assert!(
        native::test_state().collapsed,
        "Collapsed input is remembered per origin"
    );
    native::test_click(2);

    native::test_source("ni", true);
    assert!(native::test_state().composing);
    assert!(
        matches!(actions.lock().unwrap().last(), Some(Action::Edit(text, true)) if text == "ni")
    );
    view.source_text = "ni".into();
    view.composing = true;
    view.revision += 1;
    native::update(view.clone());
    assert!(
        native::test_state().composing,
        "A presentation update must preserve marked text"
    );
    let close_count = actions
        .lock()
        .unwrap()
        .iter()
        .filter(|action| matches!(action, Action::Close))
        .count();
    native::test_escape();
    assert_eq!(
        actions
            .lock()
            .unwrap()
            .iter()
            .filter(|action| matches!(action, Action::Close))
            .count(),
        close_count,
        "Escape during IME input must not close the window"
    );
    native::test_source("ni", true);
    native::test_source("你", false);
    assert!(!native::test_state().composing);
    assert!(
        matches!(actions.lock().unwrap().last(), Some(Action::Edit(text, false)) if text == "你")
    );
    view.composing = false;
    view.source_text = (0..80)
        .map(|index| {
            format!(
                "第 {} 行：长文本应当自动换行并且可以完整滚动、选择与复制。\n",
                index + 1
            )
        })
        .collect();
    view.preferences.translation.window_width = 300;
    view.revision += 1;
    native::update(view.clone());
    let state = native::test_state();
    assert_eq!(state.frame.size.width, 300.0);
    assert!(state.document_height > state.frame.size.height);
    snapshot("result-narrow-long-text.png");

    view.mode = ResultWindowMode::Ocr;
    view.origin = ResultWindowOrigin::Ocr;
    view.ocr_text =
        "网址 https://example.com，邮箱 support@example.com\n电话 138 0013 8000，验证码 AB1234"
            .into();
    view.ocr_confidence = Some(0.96);
    view.ocr_language = Some("zh-CN".into());
    view.ocr_image = Some(Arc::from(
        base64::engine::general_purpose::STANDARD.encode(png(600, 240, 0)),
    ));
    view.preferences.ocr.show_confidence = true;
    view.presentation += 1;
    view.revision += 1;
    native::update(view.clone());
    let state = native::test_state();
    assert!(state.decoded_image && state.fragments >= 4);
    assert_eq!(state.ocr_text, view.ocr_text);
    snapshot("result-ocr.png");
    native::test_copy_fragment(0);
    assert!(
        matches!(actions.lock().unwrap().last(), Some(Action::Copy(text)) if text == "https://example.com")
    );
    native::test_click(9);
    assert!(
        matches!(actions.lock().unwrap().last(), Some(Action::Copy(text)) if text == &view.ocr_text)
    );
    native::test_click(11);
    assert!(
        matches!(actions.lock().unwrap().last(), Some(Action::Favorite(original, ids)) if original.mode == ResultWindowMode::Ocr && ids.is_empty())
    );
    native::test_click(12);
    assert!(matches!(
        actions.lock().unwrap().last(),
        Some(Action::ClearOcr)
    ));
    native::test_click(8);
    assert!(matches!(
        actions.lock().unwrap().last(),
        Some(Action::FileOcr)
    ));

    view.preferences.translation.hide_on_blur = true;
    view.revision += 1;
    native::update(view.clone());
    let count = actions.lock().unwrap().len();
    native::test_blur(Duration::from_millis(100), false);
    native::test_blur(Duration::from_millis(400), true);
    assert_eq!(actions.lock().unwrap().len(), count);
    native::test_click(0);
    native::test_blur(Duration::from_millis(400), false);
    assert_eq!(actions.lock().unwrap().len(), count);
    native::test_click(0);
    view.dialog_open = true;
    view.revision += 1;
    native::update(view.clone());
    native::test_blur(Duration::from_millis(400), false);
    assert_eq!(actions.lock().unwrap().len(), count);
    view.dialog_open = false;
    view.revision += 1;
    native::update(view.clone());
    native::test_blur(Duration::from_millis(400), false);
    assert!(matches!(
        actions.lock().unwrap().last(),
        Some(Action::Close)
    ));

    view.visible = false;
    view.revision += 1;
    native::update(view.clone());
    assert!(!native::test_state().decoded_image);
    assert!(!native::test_state().visible);
    view.visible = true;
    view.preferences.general.theme = "dark".into();
    view.preferences.translation.hide_on_blur = false;
    view.revision += 1;
    native::update(view.clone());
    assert!(native::test_state().decoded_image);
    assert!(!native::test_state().pinned);
    snapshot("result-ocr-dark.png");
    view.ocr_image = None;
    view.ocr_text.clear();
    view.ocr_language = None;
    view.ocr_confidence = None;
    view.revision += 1;
    native::update(view.clone());
    assert!(!native::test_state().decoded_image);
    assert_eq!(native::test_state().fragments, 0);
    snapshot("result-file-ocr-empty.png");
    native::test_destroy();
    defaults.removePersistentDomainForName(&suite);
}

async fn until(mut condition: impl FnMut() -> bool, message: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "{message}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

pub(crate) async fn run(app: &tauri::AppHandle, services: Arc<Services>) {
    let state = app.state::<AppState>();
    let view = state.result_session.snapshot();
    on_main(app, move || controls(view)).await;
    state
        .result_window
        .open(ResultWindowOpenRequest::selection_translation(
            "native source".into(),
        ))
        .await
        .unwrap();
    until(
        || {
            state
                .result_session
                .snapshot()
                .rows
                .first()
                .is_some_and(|row| row.status == TranslationStatus::Success)
        },
        "Native automatic payload translation did not finish",
    )
    .await;
    assert_eq!(
        services.requests.lock().unwrap().len(),
        1,
        "Immediate payload bypasses disabled automatic editing translation"
    );
    assert!(
        app.webview_windows().is_empty(),
        "Result window must not create a WebView"
    );
    until(
        || !state.result_session.snapshot().translating,
        "Result remained busy",
    )
    .await;
    let deadline = Instant::now() + Duration::from_secs(5);
    let history = loop {
        let history = state
            .history
            .history
            .get_translation_history(10, 0)
            .await
            .unwrap();
        if history
            .first()
            .is_some_and(|entry| entry.source_text == "native source")
        {
            break history;
        }
        assert!(
            Instant::now() < deadline,
            "Native translation history was not committed"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    assert_eq!(history[0].source_text, "native source");
    state
        .result_window
        .open(ResultWindowOpenRequest::show_translation())
        .await
        .unwrap();
    assert_eq!(state.result_session.snapshot().source_text, "native source");
    assert_eq!(
        state.result_session.snapshot().rows[0]
            .result
            .translated_text,
        "原生译文：native source"
    );
    on_main(app, || native::test_click(5)).await;
    until(
        || {
            services.requests.lock().unwrap().len() == 2
                && !state.result_session.snapshot().translating
        },
        "Native translate button did not run",
    )
    .await;
    on_main(app, || native::test_click(103)).await;
    until(
        || state.result_session.snapshot().rows[0].favorite,
        "Native provider favorite did not finish",
    )
    .await;
    let favorites = state
        .favorites
        .favorites
        .query(FavoriteQuery {
            kind: Some(FavoriteKind::Translation),
            limit: 10,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(favorites.total, 1);
    assert!(
        matches!(&favorites.items[0].content, FavoriteContent::Translation(result) if result.source_text == "native source" && result.result.translated_text == "原生译文：native source")
    );
    on_main(app, || native::test_click(4)).await;
    until(
        || !services.speech.lock().unwrap().is_empty(),
        "Native source speech did not reach the TTS service",
    )
    .await;
    assert_eq!(
        services.speech.lock().unwrap()[0],
        ("native source".into(), "fixture-en".into())
    );
    let image = base64::engine::general_purpose::STANDARD.encode(png(400, 200, 0));
    state
        .result_window
        .open(ResultWindowOpenRequest::capture_ocr(
            "你 好".into(),
            Some(image),
            Some(0.93),
            Some("zh-CN".into()),
        ))
        .await
        .unwrap();
    on_main(app, || {
        assert!(native::test_state().decoded_image);
        native::test_click(11);
    })
    .await;
    until(
        || state.result_session.snapshot().ocr_favorite,
        "Native OCR favorite did not finish",
    )
    .await;
    let favorites = state
        .favorites
        .favorites
        .query(FavoriteQuery {
            kind: Some(FavoriteKind::Ocr),
            limit: 10,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(favorites.total, 1);
    assert!(
        matches!(&favorites.items[0].content, FavoriteContent::Ocr(result) if result.recognized_text == "你好" && result.confidence == Some(0.93) && result.source_asset_path.is_some())
    );
    on_main(app, || native::test_click(12)).await;
    until(
        || state.result_session.snapshot().ocr_text.is_empty(),
        "Native OCR clear did not finish",
    )
    .await;
    on_main(app, || {
        assert!(!native::test_state().decoded_image);
        native::test_escape();
    })
    .await;
    until(
        || !state.result_session.snapshot().visible,
        "Escape did not hide the native result window",
    )
    .await;
    assert!(app.webview_windows().is_empty());
    on_main(app, native::test_destroy).await;
    println!("Native result controls, IME, sizing, OCR rendering, copy actions, favorites, speech and lifecycle passed");
}
