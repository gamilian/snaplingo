use std::path::PathBuf;

use crate::domain::capture::{LogicalRect, MonitorSnapshotView};

pub(super) const CAPTURE_WINDOW_LABEL: &str = "capture";

pub(super) fn normalized_capture_mode(mode: &str) -> &'static str {
    match mode {
        "screenshot"
        | "screenshot-copy"
        | "screenshot-ocr"
        | "silent-screenshot-ocr"
        | "screenshot-translate" => match mode {
            "screenshot-copy" => "screenshot-copy",
            "screenshot-ocr" => "screenshot-ocr",
            "silent-screenshot-ocr" => "silent-screenshot-ocr",
            "screenshot-translate" => "screenshot-translate",
            _ => "screenshot",
        },
        _ => "screenshot",
    }
}

fn capture_window_url(mode: &str) -> PathBuf {
    PathBuf::from(format!(
        "index.html?window=capture&mode={}",
        normalized_capture_mode(mode)
    ))
}

pub(super) fn capture_window_prewarm_url() -> PathBuf {
    PathBuf::from("index.html?window=capture")
}

pub(super) fn capture_window_url_with_session(mode: &str, session_id: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}&sessionId={}",
        capture_window_url(mode).to_string_lossy(),
        session_id
    ))
}

pub fn capture_window_bounds(monitors: &[MonitorSnapshotView]) -> Option<LogicalRect> {
    if monitors.is_empty() {
        return None;
    }

    let left = monitors
        .iter()
        .map(|monitor| monitor.logical_bounds.x)
        .fold(f64::INFINITY, f64::min);
    let top = monitors
        .iter()
        .map(|monitor| monitor.logical_bounds.y)
        .fold(f64::INFINITY, f64::min);
    let right = monitors
        .iter()
        .map(|monitor| monitor.logical_bounds.x + monitor.logical_bounds.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = monitors
        .iter()
        .map(|monitor| monitor.logical_bounds.y + monitor.logical_bounds.height)
        .fold(f64::NEG_INFINITY, f64::max);

    Some(LogicalRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

pub(super) fn capture_snapshot_window_labels_to_hide(
    visible_window_labels: &[String],
) -> Vec<String> {
    visible_window_labels
        .iter()
        .filter(|label| label.as_str() == CAPTURE_WINDOW_LABEL)
        .cloned()
        .collect()
}

pub(super) fn capture_snapshot_window_labels_to_restore(
    _hidden_window_labels: &[String],
) -> Vec<String> {
    Vec::new()
}

pub fn capture_snapshot_hide_settle_delay_ms(hidden_window_labels: &[String]) -> u64 {
    if hidden_window_labels.is_empty() {
        return 0;
    }

    100
}

#[cfg(test)]
mod tests {
    use crate::domain::capture::{LogicalRect, MonitorSnapshotView, PhysicalRect};

    #[test]
    fn capture_window_url_encodes_supported_mode() {
        assert_eq!(
            super::capture_window_url("screenshot-ocr").to_string_lossy(),
            "index.html?window=capture&mode=screenshot-ocr"
        );
    }

    #[test]
    fn capture_window_url_encodes_screenshot_copy_mode() {
        assert_eq!(
            super::capture_window_url("screenshot-copy").to_string_lossy(),
            "index.html?window=capture&mode=screenshot-copy"
        );
    }

    #[test]
    fn capture_window_url_encodes_silent_screenshot_ocr_mode() {
        assert_eq!(
            super::capture_window_url("silent-screenshot-ocr").to_string_lossy(),
            "index.html?window=capture&mode=silent-screenshot-ocr"
        );
    }

    #[test]
    fn capture_window_url_falls_back_to_screenshot_for_unknown_mode() {
        assert_eq!(
            super::capture_window_url("unknown").to_string_lossy(),
            "index.html?window=capture&mode=screenshot"
        );
    }

    #[test]
    fn capture_window_url_with_session_appends_session_id() {
        assert_eq!(
            super::capture_window_url_with_session("screenshot", "session-1").to_string_lossy(),
            "index.html?window=capture&mode=screenshot&sessionId=session-1"
        );
    }

    #[test]
    fn capture_window_prewarm_url_has_no_launch_mode() {
        assert_eq!(
            super::capture_window_prewarm_url().to_string_lossy(),
            "index.html?window=capture"
        );
    }

    #[test]
    fn hides_only_existing_capture_overlay_before_capture_snapshot() {
        assert_eq!(
            super::capture_snapshot_window_labels_to_hide(&[
                "main".to_string(),
                "settings".to_string(),
                "capture".to_string(),
                "pin-pin-1".to_string(),
            ]),
            vec!["capture".to_string()]
        );
    }

    #[test]
    fn does_not_restore_business_windows_after_capture_snapshot() {
        assert!(super::capture_snapshot_window_labels_to_restore(&[
            "main".to_string(),
            "settings".to_string(),
            "capture".to_string(),
            "pin-pin-1".to_string(),
        ])
        .is_empty());
    }

    #[test]
    fn plans_capture_snapshot_settle_delay_after_hiding_windows() {
        assert_eq!(
            super::capture_snapshot_hide_settle_delay_ms(&["capture".to_string()]),
            100
        );
        assert_eq!(super::capture_snapshot_hide_settle_delay_ms(&[]), 0);
    }

    #[test]
    fn capture_window_bounds_union_monitor_logical_bounds() {
        let monitors = vec![
            MonitorSnapshotView {
                id: "left".to_string(),
                logical_bounds: LogicalRect {
                    x: -1280.0,
                    y: 0.0,
                    width: 1280.0,
                    height: 720.0,
                },
                physical_bounds: PhysicalRect {
                    x: -2560,
                    y: 0,
                    width: 2560,
                    height: 1440,
                },
                scale_factor: 2.0,
                image_base64: String::new(),
            },
            MonitorSnapshotView {
                id: "primary".to_string(),
                logical_bounds: LogicalRect {
                    x: 0.0,
                    y: 0.0,
                    width: 1440.0,
                    height: 900.0,
                },
                physical_bounds: PhysicalRect {
                    x: 0,
                    y: 0,
                    width: 2880,
                    height: 1800,
                },
                scale_factor: 2.0,
                image_base64: String::new(),
            },
            MonitorSnapshotView {
                id: "top".to_string(),
                logical_bounds: LogicalRect {
                    x: 0.0,
                    y: -600.0,
                    width: 960.0,
                    height: 600.0,
                },
                physical_bounds: PhysicalRect {
                    x: 0,
                    y: -1200,
                    width: 1920,
                    height: 1200,
                },
                scale_factor: 2.0,
                image_base64: String::new(),
            },
        ];

        assert_eq!(
            super::capture_window_bounds(&monitors),
            Some(LogicalRect {
                x: -1280.0,
                y: -600.0,
                width: 2720.0,
                height: 1500.0,
            })
        );
    }
}
