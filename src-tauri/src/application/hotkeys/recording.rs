use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotkeyRecordingSession {
    pub id: String,
    pub window_label: String,
}

/// Owns the temporary destination for already registered hotkey events.
#[derive(Default)]
pub struct HotkeyRecording {
    session: Mutex<Option<HotkeyRecordingSession>>,
}

impl HotkeyRecording {
    pub fn begin(&self, window_label: &str, id: String) {
        *self.session.lock().unwrap() = Some(HotkeyRecordingSession {
            id,
            window_label: window_label.to_string(),
        });
    }

    pub fn session(&self) -> Option<HotkeyRecordingSession> {
        self.session.lock().unwrap().clone()
    }

    pub fn end(&self, window_label: &str, id: &str) {
        let mut session = self.session.lock().unwrap();
        if session
            .as_ref()
            .is_some_and(|session| session.window_label == window_label && session.id == id)
        {
            *session = None;
        }
    }

    pub fn cancel_for_window(&self, window_label: &str) {
        let mut session = self.session.lock().unwrap();
        if session
            .as_ref()
            .is_some_and(|session| session.window_label == window_label)
        {
            *session = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HotkeyRecording;

    #[test]
    fn late_cleanup_cannot_cancel_a_new_recording() {
        let recording = HotkeyRecording::default();
        recording.begin("settings", "old".into());
        recording.begin("settings", "new".into());
        recording.end("settings", "old");
        recording.end("result", "new");
        assert_eq!(recording.session().unwrap().id, "new");
        recording.end("settings", "new");
        recording.end("settings", "new");
        assert_eq!(recording.session(), None);
    }

    #[test]
    fn window_cleanup_only_cancels_its_own_recording() {
        let recording = HotkeyRecording::default();
        recording.begin("settings", "active".into());
        recording.cancel_for_window("capture");
        assert!(recording.session().is_some());
        recording.cancel_for_window("settings");
        assert_eq!(recording.session(), None);
    }
}
