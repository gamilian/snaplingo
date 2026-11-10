use crate::features::settings::presenter::window::SettingsSessionDependencies;
use crate::features::settings::presenter::window::SettingsWindowSession;
use snaplingo_ui::{Action, GeneralField, Page, Value, WindowIntent, WindowSnapshot};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[tokio::test]
async fn seven_pages_use_one_session_and_persist_edits_without_an_os_application() {
    let fixture = crate::features::settings::presenter::general_controller::tests::fixture();
    let preferences = fixture.preferences();
    let configuration = preferences.configuration.clone();
    let capacity = Arc::new(AtomicUsize::new(0));
    let calls = capacity.clone();
    let mut session = SettingsWindowSession::new(
        SettingsSessionDependencies {
            preferences,
            services: Arc::new(crate::features::settings::presenter::services::tests::session),
            library: Arc::new(crate::features::settings::presenter::library::tests::session),
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
    let fixture = crate::features::settings::presenter::general_controller::tests::fixture();
    let recording = snaplingo_core::hotkeys::HotkeyRecording::default();
    let make = || {
        SettingsWindowSession::new(
            SettingsSessionDependencies {
                preferences: fixture.preferences(),
                services: Arc::new(crate::features::settings::presenter::services::tests::session),
                library: Arc::new(crate::features::settings::presenter::library::tests::session),
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

fn window_session(database: Arc<snaplingo_storage::Database>) -> SettingsWindowSession {
    let fixture = crate::features::settings::presenter::general_controller::tests::fixture();
    SettingsWindowSession::new(
        SettingsSessionDependencies {
            preferences: fixture.preferences(),
            services: Arc::new(crate::features::settings::presenter::services::tests::session),
            library: Arc::new(move |kind| {
                crate::features::settings::presenter::library::tests::session_with_database(
                    database.clone(),
                    kind,
                )
            }),
            capacity: Arc::new(|| Ok(())),
            open_url: Arc::new(|_| Ok(())),
        },
        Default::default(),
        tokio::runtime::Handle::current(),
    )
}
fn history_database() -> Arc<snaplingo_storage::Database> {
    let database = Arc::new(snaplingo_storage::Database::in_memory().unwrap());
    database.with_connection(|connection| {
        for text in ["first", "second"] {
            connection.execute("INSERT INTO history_records(kind,timestamp) VALUES ('translation','2026-11-08T01:00:00+00:00')", []).map_err(snaplingo_core::AppError::database)?;
            let id = connection.last_insert_rowid();
            connection.execute("INSERT INTO translation_history(history_id,source_text,source_lang,target_lang,providers_used,results,duration_ms) VALUES (?1,?2,'en','zh-CN','[]','[]',1)", rusqlite::params![id,text]).map_err(snaplingo_core::AppError::database)?;
        }
        Ok(())
    }).unwrap();
    database
}
fn history_count(database: &snaplingo_storage::Database) -> usize {
    database
        .with_connection(|connection| {
            connection
                .query_row("SELECT COUNT(*) FROM history_records", [], |row| row.get(0))
                .map_err(snaplingo_core::AppError::database)
        })
        .unwrap()
}
async fn wait_until(mut ready: impl FnMut() -> bool) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !ready() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("Settings workflow did not finish");
}
async fn history_key(session: &SettingsWindowSession) -> snaplingo_ui::library::ItemKey {
    wait_until(|| matches!(session.snapshot().unwrap(), WindowSnapshot::Library(snapshot) if !snapshot.loading && !snapshot.items.is_empty())).await;
    let WindowSnapshot::Library(snapshot) = session.snapshot().unwrap() else {
        panic!()
    };
    snapshot.items[0].key
}

#[tokio::test]
async fn library_confirmation_cancels_or_deletes_once_through_the_shared_window_interface() {
    use snaplingo_ui::library::{LibraryFilter, LibraryIntent};
    let database = history_database();
    let mut session = window_session(database.clone());
    session.navigate(Page::History).unwrap();
    let key = history_key(&session).await;
    let request = session
        .dispatch(
            Page::History,
            WindowIntent::Library(LibraryIntent::Delete(key)),
        )
        .unwrap()
        .expect("Deleting a single record requires confirmation on every host");
    assert_eq!(history_count(&database), 2);
    session.confirm(request.clone(), false).unwrap();
    session.confirm(request, true).unwrap();
    assert_eq!(
        history_count(&database),
        2,
        "Cancelled requests cannot later execute"
    );

    let request = session
        .dispatch(
            Page::History,
            WindowIntent::Library(LibraryIntent::Delete(key)),
        )
        .unwrap()
        .unwrap();
    session.confirm(request.clone(), true).unwrap();
    session.confirm(request, true).unwrap();
    wait_until(|| matches!(session.snapshot().unwrap(), WindowSnapshot::Library(snapshot) if !snapshot.loading && snapshot.items.len() == 1)).await;
    assert_eq!(history_count(&database), 1);
    let remaining = history_key(&session).await;
    assert_ne!(remaining, key);
    let clear = session
        .dispatch(
            Page::History,
            WindowIntent::Library(LibraryIntent::ConfirmClearHistory(LibraryFilter::All)),
        )
        .unwrap()
        .unwrap();
    session.confirm(clear.clone(), false).unwrap();
    assert_eq!(history_count(&database), 1);
    let clear = session
        .dispatch(
            Page::History,
            WindowIntent::Library(LibraryIntent::ConfirmClearHistory(LibraryFilter::All)),
        )
        .unwrap()
        .unwrap();
    session.confirm(clear, true).unwrap();
    wait_until(|| history_count(&database) == 0).await;
}

#[tokio::test]
async fn closed_changed_and_replaced_settings_interactions_reject_old_confirmations() {
    use snaplingo_ui::library::LibraryIntent;
    let database = history_database();
    let mut session = window_session(database.clone());
    session.navigate(Page::History).unwrap();
    let key = history_key(&session).await;
    let request = session
        .dispatch(
            Page::History,
            WindowIntent::Library(LibraryIntent::Delete(key)),
        )
        .unwrap()
        .unwrap();
    session.navigate(Page::Favorites).unwrap();
    session.confirm(request, true).unwrap();
    assert_eq!(history_count(&database), 2);
    assert!(session
        .dispatch(
            Page::History,
            WindowIntent::Library(LibraryIntent::Delete(key))
        )
        .is_err());
    session.navigate(Page::History).unwrap();
    let key = history_key(&session).await;
    let hidden = session
        .dispatch(
            Page::History,
            WindowIntent::Library(LibraryIntent::Delete(key)),
        )
        .unwrap()
        .unwrap();
    session.cancel_confirmation();
    session.confirm(hidden, true).unwrap();
    assert_eq!(history_count(&database), 2);

    let obsolete = session
        .dispatch(
            Page::History,
            WindowIntent::Library(LibraryIntent::Delete(key)),
        )
        .unwrap()
        .unwrap();
    let current = session
        .dispatch(
            Page::History,
            WindowIntent::Library(LibraryIntent::Delete(key)),
        )
        .unwrap()
        .unwrap();
    session.confirm(obsolete.clone(), false).unwrap();
    session.confirm(obsolete, true).unwrap();
    assert_eq!(
        history_count(&database),
        2,
        "An old callback must neither execute nor erase the new request"
    );
    session.confirm(current, true).unwrap();
    wait_until(|| matches!(session.snapshot().unwrap(), WindowSnapshot::Library(snapshot) if !snapshot.loading && snapshot.items.len() == 1)).await;
    assert_eq!(history_count(&database), 1);
    let remaining = history_key(&session).await;
    let closed = session
        .dispatch(
            Page::History,
            WindowIntent::Library(LibraryIntent::Delete(remaining)),
        )
        .unwrap()
        .unwrap();
    drop(session);
    let mut reopened = window_session(database.clone());
    reopened.confirm(closed, true).unwrap();
    assert_eq!(history_count(&database), 1);
}

#[tokio::test]
async fn provider_removal_is_confirmed_before_the_shared_session_mutates_configuration() {
    use snaplingo_ui::services::{ServiceCustomField, ServicesIntent, ServicesRoute};
    let mut session = window_session(history_database());
    session.navigate(Page::Services).unwrap();
    session
        .dispatch(
            Page::Services,
            WindowIntent::Services(ServicesIntent::Route(ServicesRoute::Translation)),
        )
        .unwrap();
    session
        .dispatch(
            Page::Services,
            WindowIntent::Services(ServicesIntent::AddCustom),
        )
        .unwrap();
    wait_until(|| matches!(session.snapshot().unwrap(), WindowSnapshot::Services(snapshot) if snapshot.form.is_some())).await;
    let WindowSnapshot::Services(snapshot) = session.snapshot().unwrap() else {
        panic!()
    };
    let revision = snapshot.form_revision;
    for (field, value) in [
        (ServiceCustomField::Name, "confirmation fixture"),
        (ServiceCustomField::ApiKey, "fixture-key"),
    ] {
        session
            .dispatch(
                Page::Services,
                WindowIntent::Services(ServicesIntent::EditCustom {
                    revision,
                    field,
                    value: value.into(),
                }),
            )
            .unwrap();
    }
    session
        .dispatch(
            Page::Services,
            WindowIntent::Services(ServicesIntent::SaveCustom { revision }),
        )
        .unwrap();
    wait_until(|| matches!(session.snapshot().unwrap(), WindowSnapshot::Services(snapshot) if snapshot.providers.iter().any(|provider| !provider.builtin))).await;
    let WindowSnapshot::Services(snapshot) = session.snapshot().unwrap() else {
        panic!()
    };
    let id = snapshot
        .providers
        .iter()
        .find(|provider| !provider.builtin)
        .unwrap()
        .id
        .clone();
    let pending = session
        .dispatch(
            Page::Services,
            WindowIntent::Services(ServicesIntent::ConfirmRemove(id.clone())),
        )
        .unwrap()
        .unwrap();
    session.confirm(pending, false).unwrap();
    assert!(
        matches!(session.snapshot().unwrap(), WindowSnapshot::Services(snapshot) if snapshot.providers.iter().any(|provider| provider.id == id))
    );
    let pending = session
        .dispatch(
            Page::Services,
            WindowIntent::Services(ServicesIntent::ConfirmRemove(id.clone())),
        )
        .unwrap()
        .unwrap();
    session.confirm(pending, true).unwrap();
    wait_until(|| matches!(session.snapshot().unwrap(), WindowSnapshot::Services(snapshot) if !snapshot.providers.iter().any(|provider| provider.id == id))).await;
}

#[tokio::test]
async fn group_reset_waits_for_confirmation_and_keeps_its_original_category() {
    use snaplingo_core::hotkeys::model::{HOTKEY_UNSET, OCR_CATEGORY, SCREENSHOT_OCR_ACTION};
    let fixture = crate::features::settings::presenter::general_controller::tests::fixture();
    let preferences = fixture.preferences();
    let hotkeys = preferences.hotkeys.clone();
    hotkeys
        .update_hotkey(
            OCR_CATEGORY.into(),
            SCREENSHOT_OCR_ACTION.into(),
            HOTKEY_UNSET.into(),
        )
        .unwrap();
    let mut session = SettingsWindowSession::new(
        SettingsSessionDependencies {
            preferences,
            services: Arc::new(crate::features::settings::presenter::services::tests::session),
            library: Arc::new(crate::features::settings::presenter::library::tests::session),
            capacity: Arc::new(|| Ok(())),
            open_url: Arc::new(|_| Ok(())),
        },
        Default::default(),
        tokio::runtime::Handle::current(),
    );
    session.navigate(Page::Ocr).unwrap();
    let saved = hotkeys.snapshot().unwrap();
    let reset = session
        .dispatch(Page::Ocr, WindowIntent::Settings(Action::ResetHotkeys))
        .unwrap()
        .unwrap();
    assert_eq!(hotkeys.snapshot().unwrap(), saved);
    session.confirm(reset, false).unwrap();
    assert_eq!(hotkeys.snapshot().unwrap(), saved);
    let stale = session
        .dispatch(Page::Ocr, WindowIntent::Settings(Action::ResetHotkeys))
        .unwrap()
        .unwrap();
    session.navigate(Page::Translation).unwrap();
    session.confirm(stale, true).unwrap();
    assert_eq!(hotkeys.snapshot().unwrap(), saved);
    session.navigate(Page::Ocr).unwrap();
    let reset = session
        .dispatch(Page::Ocr, WindowIntent::Settings(Action::ResetHotkeys))
        .unwrap()
        .unwrap();
    session.confirm(reset, true).unwrap();
    wait_until(|| !session.saving_hotkeys()).await;
    assert_eq!(
        hotkeys.snapshot().unwrap().ocr,
        hotkeys.default_snapshot().ocr
    );
    assert_eq!(hotkeys.snapshot().unwrap().translation, saved.translation);
}
