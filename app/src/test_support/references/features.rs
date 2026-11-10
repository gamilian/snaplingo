use crate::test_support::support::on_main;
use crate::{bootstrap, features, integrations, AppState};
struct LiveFeature {
    page: std::rc::Rc<features::settings::test_support::FeaturePage>,
    window: objc2::rc::Retained<objc2_app_kit::NSWindow>,
}
#[cfg(target_os = "macos")]
thread_local! {
    static LIVE: std::cell::RefCell<Option<LiveFeature>> = const { std::cell::RefCell::new(None) };
}
#[cfg(target_os = "macos")]
fn feature_view(
    page: &features::settings::test_support::FeaturePage,
) -> &features::settings::test_support::FeatureView {
    page.view()
        .downcast_ref::<features::settings::test_support::FeatureView>()
        .unwrap()
}

#[cfg(target_os = "macos")]
pub(in crate::test_support) fn run() {
    use features::settings::test_support::{FeatureField, FeatureKind, FeaturePage};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSApplication, NSBackingStoreType, NSEvent, NSEventModifierFlags, NSEventType, NSWindow,
        NSWindowStyleMask,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use snaplingo_core::settings::model::OcrSettings;
    use snaplingo_core::settings::model::TranslationSettings;
    use snaplingo_core::settings::{GeneralValue, OcrField, TranslationField};
    use std::sync::Arc;
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.finishLaunching();
    let crate::test_support::support::IsolatedApplication {
        directory: _directory,
        state,
    } = crate::test_support::support::IsolatedApplication::new(
        "com.snaplingo.native-features-test",
    );
    let state = Arc::new(state);
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(698.0, 650.0)),
            NSWindowStyleMask::Titled | NSWindowStyleMask::Resizable,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    let recording = snaplingo_core::hotkeys::HotkeyRecording::default();
    let page = FeaturePage::new(
        mtm,
        FeatureKind::Ocr,
        &bootstrap::settings::window::services(state.clone()),
        recording.clone(),
    )
    .unwrap();
    window.setContentView(Some(page.view()));
    let view = feature_view(&page);
    for field in [
        OcrField::GpuAcceleration,
        OcrField::PreserveFormatting,
        OcrField::RemoveChineseSpaces,
        OcrField::ShowConfidence,
        OcrField::HideSilentStatus,
    ] {
        let before = field.value(&state.settings.configuration.snapshot().unwrap().ocr);
        view.test_toggle(FeatureField::Ocr(field));
        let GeneralValue::Toggle(before) = before else {
            unreachable!()
        };
        assert_eq!(
            field.value(&state.settings.configuration.snapshot().unwrap().ocr),
            GeneralValue::Toggle(!before)
        );
    }
    view.test_choose(FeatureField::Ocr(OcrField::RecognitionLanguage), "ja");
    view.test_choose(FeatureField::Ocr(OcrField::WindowPosition), "last-position");
    assert_eq!(
        state
            .settings
            .configuration
            .snapshot()
            .unwrap()
            .ocr
            .recognition_language,
        "ja"
    );
    assert_eq!(
        state
            .settings
            .configuration
            .snapshot()
            .unwrap()
            .ocr
            .window_position,
        "last-position"
    );
    // Retain fixture defaults for visual comparison; preference actions above used real services.
    state
        .settings
        .configuration
        .update_ocr(OcrSettings::default())
        .unwrap();
    page.refresh().unwrap();
    let artifacts =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/native-features");
    std::fs::create_dir_all(&artifacts).unwrap();
    capture(&page, &window, &state, &artifacts, "ocr");
    page.dispose();
    let before = state.settings.configuration.snapshot().unwrap();
    view.test_toggle(FeatureField::Ocr(OcrField::GpuAcceleration));
    view.test_choose(FeatureField::Ocr(OcrField::RecognitionLanguage), "ja");
    assert_eq!(
        state.settings.configuration.snapshot().unwrap(),
        before,
        "A retained control from a disposed page must not change settings"
    );
    window.setContentView(None);
    drop(page);
    let page = FeaturePage::new(
        mtm,
        FeatureKind::Translation,
        &bootstrap::settings::window::services(state.clone()),
        recording.clone(),
    )
    .unwrap();
    window.setContentView(Some(page.view()));
    let view = feature_view(&page);
    for field in [
        TranslationField::AutoTranslate,
        TranslationField::IncrementalTranslation,
        TranslationField::AutoCopy,
        TranslationField::PreserveLineBreaks,
        TranslationField::WindowAlwaysOnTop,
        TranslationField::HideOnBlur,
    ] {
        let GeneralValue::Toggle(before) =
            field.value(&state.settings.configuration.snapshot().unwrap().translation)
        else {
            unreachable!()
        };
        view.test_toggle(FeatureField::Translation(field));
        assert_eq!(
            field.value(&state.settings.configuration.snapshot().unwrap().translation),
            GeneralValue::Toggle(!before)
        );
    }
    for (field, value) in [
        (TranslationField::DefaultSourceLang, "ja"),
        (TranslationField::DefaultTargetLang, "en"),
        (TranslationField::SelectionTextMode, "speed"),
        (TranslationField::SelectionWindowPosition, "last-position"),
        (TranslationField::InputWindowPosition, "cursor"),
        (TranslationField::SelectionInputState, "collapsed"),
        (TranslationField::ScreenshotInputState, "expanded"),
    ] {
        view.test_choose(FeatureField::Translation(field), value);
        assert_eq!(
            field.value(&state.settings.configuration.snapshot().unwrap().translation),
            GeneralValue::Text(value.into())
        );
    }
    view.test_range(
        FeatureField::Translation(TranslationField::WindowWidth),
        810.0,
    );
    view.test_range(
        FeatureField::Translation(TranslationField::MaxWindowHeightRatio),
        83.0,
    );
    assert_eq!(
        state
            .settings
            .configuration
            .snapshot()
            .unwrap()
            .translation
            .window_width,
        810
    );
    assert_eq!(
        state
            .settings
            .configuration
            .snapshot()
            .unwrap()
            .translation
            .max_window_height_ratio,
        83
    );
    state
        .settings
        .configuration
        .update_translation(TranslationSettings::default())
        .unwrap();
    page.refresh().unwrap();
    capture(&page, &window, &state, &artifacts, "translation");
    assert!(page.jump("unknown").is_err());
    LIVE.with_borrow_mut(|live| *live = Some(LiveFeature { page, window }));
    let configuration = state.settings.configuration.clone();
    let lifetime = Arc::downgrade(&configuration);
    let worker = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.block_on(async {
                state
                    .settings
                    .application
                    .edit_translation(TranslationField::WindowWidth, GeneralValue::Number(880))
                    .unwrap();
                // Drain notifications through AppKit; the latest edit must win over earlier queued snapshots.
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        feature_view(&live.as_ref().unwrap().page).test_choose(
                            FeatureField::Translation(TranslationField::DefaultSourceLang),
                            "ko",
                        )
                    })
                })
                .await;
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                loop {
                    if on_main(|| {
                        LIVE.with_borrow(|live| {
                            feature_view(&live.as_ref().unwrap().page)
                                .test_projection()
                                .preferences
                                .translation
                                .window_width
                                == 880
                        })
                    })
                    .await
                    {
                        break;
                    }
                    assert!(
                        std::time::Instant::now() < deadline,
                        "Background native feature preferences did not refresh"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
                assert_eq!(
                    configuration
                        .snapshot()
                        .unwrap()
                        .translation
                        .default_source_lang,
                    "ko"
                );
                on_main(|| {
                    LIVE.with_borrow_mut(|live| {
                        let live = live.take().unwrap();
                        live.page.dispose();
                        live.window.setContentView(None);
                        live.window.close();
                    })
                })
                .await;
                drop(configuration);
                drop(state);
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while lifetime.upgrade().is_some() {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "Disposed native feature page retained settings services"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            })
        }));
        integrations::ui_thread::dispatch(|| {
            let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
            app.stop(None);
            let event=NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(NSEventType::ApplicationDefined,NSPoint::new(0.0,0.0),NSEventModifierFlags::empty(),0.0,0,None,0,0,0).unwrap();
            app.postEvent_atStart(&event, true);
        });
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    });
    app.run();
    worker.join().unwrap();
    println!("Native OCR and Translation preferences: all 22 fields, real choices/toggles/ranges, latest worker state, disposal and three-theme section snapshots passed without Tauri initialization");
}
#[cfg(target_os = "macos")]
fn capture(
    page: &features::settings::test_support::FeaturePage,
    window: &objc2_app_kit::NSWindow,
    state: &AppState,
    artifacts: &std::path::Path,
    name: &str,
) {
    use objc2_foundation::NSSize;
    use snaplingo_core::settings::GeneralField;
    use snaplingo_core::settings::GeneralValue;
    let kind = if name == "ocr" {
        features::settings::test_support::FeatureKind::Ocr
    } else {
        features::settings::test_support::FeatureKind::Translation
    };
    for appearance in ["light", "dark"] {
        for (theme, width, height, shell, compact) in [
            ("native", 698.0, 650.0, 900.0, false),
            ("studio", 832.0, 650.0, 900.0, false),
            ("paper", 900.0, 582.0, 900.0, false),
            ("native", 635.0, 650.0, 700.0, true),
            ("studio", 632.0, 650.0, 700.0, true),
            ("paper", 700.0, 582.0, 700.0, true),
        ] {
            let theme_key = format!(
                "{theme}{}{}",
                if appearance == "dark" { "-dark" } else { "" },
                if compact { "-compact" } else { "" }
            );
            state
                .settings
                .application
                .edit_general(
                    GeneralField::InterfaceTheme,
                    GeneralValue::Text(theme.into()),
                )
                .unwrap();
            state
                .settings
                .application
                .edit_general(
                    GeneralField::Appearance,
                    GeneralValue::Text(appearance.into()),
                )
                .unwrap();
            page.refresh().unwrap();
            window.setContentSize(NSSize::new(width, height));
            page.view().setFrameSize(NSSize::new(width, height));
            page.view().set_layout_widths(shell, shell);
            for &(id, _) in kind.sections() {
                page.jump(id).unwrap();
                window.displayIfNeeded();
                std::fs::write(
                    artifacts.join(format!("{name}-{theme_key}-{id}.png")),
                    integrations::system::native_view_snapshot::snapshot(page.view()),
                )
                .unwrap();
                std::fs::write(
                    artifacts.join(format!("{name}-{theme_key}-{id}.json")),
                    serde_json::to_vec_pretty(&feature_view(page).test_geometry()).unwrap(),
                )
                .unwrap();
            }
        }
    }
}
