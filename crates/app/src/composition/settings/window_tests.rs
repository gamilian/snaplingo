use super::window::*;
use snaplingo_ui::{
    library as library_ui, services as services_ui, Action, GeneralField, Page, Value,
    WindowIntent, WindowSnapshot,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[tokio::test]
async fn seven_page_backend_reuses_real_preferences_services_library_and_callbacks() {
    let fixture = super::library::tests::fixture();
    let capacity = Arc::new(AtomicUsize::new(0));
    let calls = capacity.clone();
    let (mut window, _subscription) = from_state(
        fixture.app.clone(),
        Default::default(),
        Arc::new(|| Ok(())),
        Arc::new(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }),
    );
    let notifications = Arc::new(AtomicUsize::new(0));
    let wakes = notifications.clone();
    window.set_notifier(Some(Arc::new(move || {
        wakes.fetch_add(1, Ordering::SeqCst);
    })));
    window
        .dispatch(
            Page::General,
            WindowIntent::Settings(Action::EditGeneral {
                field: GeneralField::ProxyUrl,
                value: Value::Text("http://127.0.0.1:8123".into()),
            }),
        )
        .unwrap();
    assert_eq!(
        fixture
            .app
            .settings
            .configuration
            .snapshot()
            .unwrap()
            .general
            .proxy_url,
        "http://127.0.0.1:8123"
    );
    assert!(notifications.load(Ordering::SeqCst) > 0);
    for page in [Page::Screenshot, Page::Ocr, Page::Translation] {
        window.navigate(page).unwrap();
        assert!(matches!(
            window.snapshot().unwrap(),
            WindowSnapshot::Settings(_)
        ));
    }
    window.navigate(Page::Services).unwrap();
    window
        .dispatch(
            Page::Services,
            WindowIntent::Services(services_ui::ServicesIntent::Route(
                services_ui::ServicesRoute::Tts,
            )),
        )
        .unwrap();
    let WindowSnapshot::Services(snapshot) = window.snapshot().unwrap() else {
        panic!()
    };
    assert_eq!(snapshot.route, services_ui::ServicesRoute::Tts);
    for page in [Page::History, Page::Favorites] {
        window.navigate(page).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let WindowSnapshot::Library(snapshot) = window.snapshot().unwrap() else {
                    panic!()
                };
                if !snapshot.loading {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let WindowSnapshot::Library(snapshot) = window.snapshot().unwrap() else {
            panic!()
        };
        assert_eq!(
            snapshot.kind,
            if page == Page::History {
                library_ui::LibraryKind::History
            } else {
                library_ui::LibraryKind::Favorites
            }
        );
        assert_eq!(snapshot.total, if page == Page::History { 2 } else { 0 });
    }
    window
        .dispatch(
            Page::Favorites,
            WindowIntent::Settings(Action::ConfigureCapacity),
        )
        .unwrap();
    assert_eq!(capacity.load(Ordering::SeqCst), 1);
    window.navigate(Page::Screenshot).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let WindowSnapshot::Settings(snapshot) = window.snapshot().unwrap() else {
                panic!()
            };
            if snapshot.capacity.history_used == Some(2)
                && snapshot.capacity.favorites_used == Some(0)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    window.set_notifier(None);
}
