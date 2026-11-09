use crate::window::{SettingsSessionDependencies, SettingsWindowSession};
use snaplingo_ui::{Action, GeneralField, Page, Value, WindowIntent, WindowSnapshot};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[tokio::test]
async fn seven_pages_use_one_session_and_persist_edits_without_an_os_application() {
    let fixture = crate::general_controller::tests::fixture();
    let preferences = fixture.preferences();
    let configuration = preferences.configuration.clone();
    let capacity = Arc::new(AtomicUsize::new(0));
    let calls = capacity.clone();
    let mut session = SettingsWindowSession::new(
        SettingsSessionDependencies {
            preferences,
            services: Arc::new(crate::services::tests::session),
            library: Arc::new(crate::library::tests::session),
            capacity: Arc::new(move || {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
            open_url: Arc::new(|_| Ok(())),
        },
        Default::default(),
        tokio::runtime::Handle::current(),
    );
    session
        .dispatch(
            Page::General,
            WindowIntent::Settings(Action::EditGeneral {
                field: GeneralField::ProxyUrl,
                value: Value::Text("http://localhost:8123".into()),
            }),
        )
        .unwrap();
    assert_eq!(
        configuration.snapshot().unwrap().general.proxy_url,
        "http://localhost:8123"
    );
    for page in [
        Page::General,
        Page::Screenshot,
        Page::Translation,
        Page::Ocr,
        Page::Services,
        Page::History,
        Page::Favorites,
    ] {
        session.navigate(page).unwrap();
        assert_eq!(session.page(), page);
        let snapshot = session.snapshot().unwrap();
        assert!(match page {
            Page::Services => matches!(snapshot, WindowSnapshot::Services(_)),
            Page::History | Page::Favorites => matches!(snapshot, WindowSnapshot::Library(_)),
            _ => matches!(snapshot, WindowSnapshot::Settings(_)),
        });
    }
    session
        .dispatch(
            Page::Favorites,
            WindowIntent::Settings(Action::ConfigureCapacity),
        )
        .unwrap();
    assert_eq!(capacity.load(Ordering::SeqCst), 1);
    session.navigate(Page::Services).unwrap();
    session
        .dispatch(
            Page::Services,
            WindowIntent::Services(snaplingo_ui::services::ServicesIntent::Route(
                snaplingo_ui::services::ServicesRoute::Tts,
            )),
        )
        .unwrap();
    let WindowSnapshot::Services(snapshot) = session.snapshot().unwrap() else {
        panic!()
    };
    assert_eq!(snapshot.route, snaplingo_ui::services::ServicesRoute::Tts);
}

#[tokio::test]
async fn a_recreated_page_rejects_the_previous_recording_and_releases_notifications() {
    let fixture = crate::general_controller::tests::fixture();
    let recording = snaplingo_core::HotkeyRecording::default();
    let make = || {
        SettingsWindowSession::new(
            SettingsSessionDependencies {
                preferences: fixture.preferences(),
                services: Arc::new(crate::services::tests::session),
                library: Arc::new(crate::library::tests::session),
                capacity: Arc::new(|| Ok(())),
                open_url: Arc::new(|_| Ok(())),
            },
            recording.clone(),
            tokio::runtime::Handle::current(),
        )
    };
    let mut first = make();
    first.navigate(Page::Ocr).unwrap();
    first
        .dispatch(
            Page::Ocr,
            WindowIntent::Settings(Action::BeginHotkeyRecording { index: 0 }),
        )
        .unwrap();
    let old = first.recording_id().unwrap();
    let invalidate = first.invalidator();
    let wakes = Arc::new(AtomicUsize::new(0));
    let counter = wakes.clone();
    first.set_notifier(Some(Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    })));
    drop(first);
    invalidate(false);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
    let mut next = make();
    next.navigate(Page::Ocr).unwrap();
    next.dispatch(
        Page::Ocr,
        WindowIntent::Settings(Action::BeginHotkeyRecording { index: 0 }),
    )
    .unwrap();
    assert_ne!(next.recording_id().unwrap(), old);
    next.navigate(Page::History).unwrap();
    assert!(recording.session().is_none());
}
