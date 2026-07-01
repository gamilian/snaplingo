use std::path::PathBuf;

const PIN_WINDOW_MAX_WIDTH: f64 = 900.0;
const PIN_WINDOW_MAX_HEIGHT: f64 = 700.0;

#[derive(Debug, PartialEq)]
pub(super) struct PinnedWindowVisibilityChange {
    pub(super) label: String,
    pub(super) visible: bool,
}

pub(super) fn pinned_window_url(image_id: &str) -> PathBuf {
    PathBuf::from(format!("index.html?window=pin&imageId={}", image_id))
}

pub(super) fn pinned_window_label(image_id: &str) -> String {
    format!("pin-{}", image_id)
}

pub(super) fn is_pinned_window_label(label: &str) -> bool {
    label.starts_with("pin-")
}

pub(super) fn next_pinned_windows_visible_state(current_visibility: &[bool]) -> Option<bool> {
    if current_visibility.is_empty() {
        return None;
    }

    Some(current_visibility.iter().any(|is_visible| !*is_visible))
}

pub(super) fn pinned_group_window_visibility_changes(
    hide_image_ids: &[String],
    show_image_ids: &[String],
) -> Vec<PinnedWindowVisibilityChange> {
    hide_image_ids
        .iter()
        .map(|image_id| PinnedWindowVisibilityChange {
            label: pinned_window_label(image_id),
            visible: false,
        })
        .chain(
            show_image_ids
                .iter()
                .map(|image_id| PinnedWindowVisibilityChange {
                    label: pinned_window_label(image_id),
                    visible: true,
                }),
        )
        .collect()
}

pub(super) fn moved_pinned_image_window_visibility_change(
    image_id: &str,
) -> PinnedWindowVisibilityChange {
    PinnedWindowVisibilityChange {
        label: pinned_window_label(image_id),
        visible: false,
    }
}

pub(super) fn hidden_pinned_group_window_visibility_changes(
    image_ids: &[String],
) -> Vec<PinnedWindowVisibilityChange> {
    image_ids
        .iter()
        .map(|image_id| PinnedWindowVisibilityChange {
            label: pinned_window_label(image_id),
            visible: false,
        })
        .collect()
}

pub(super) fn destroyed_pinned_group_window_labels(image_ids: &[String]) -> Vec<String> {
    image_ids
        .iter()
        .map(|image_id| pinned_window_label(image_id))
        .collect()
}

pub(super) fn pinned_window_size(width: u32, height: u32) -> (f64, f64) {
    let width = width.max(1) as f64;
    let height = height.max(1) as f64;
    let scale = (PIN_WINDOW_MAX_WIDTH / width)
        .min(PIN_WINDOW_MAX_HEIGHT / height)
        .min(1.0);

    ((width * scale).max(80.0), (height * scale).max(60.0))
}

#[cfg(test)]
mod tests {
    use super::PinnedWindowVisibilityChange;

    #[test]
    fn pinned_window_url_targets_pin_route() {
        assert_eq!(
            super::pinned_window_url("pin-1").to_string_lossy(),
            "index.html?window=pin&imageId=pin-1"
        );
    }

    #[test]
    fn identifies_pinned_window_labels() {
        assert!(super::is_pinned_window_label("pin-pin-1"));
        assert!(!super::is_pinned_window_label("capture"));
        assert!(!super::is_pinned_window_label("settings"));
    }

    #[test]
    fn toggles_pinned_windows_based_on_current_visibility() {
        assert_eq!(
            super::next_pinned_windows_visible_state(&[true, false]),
            Some(true)
        );
        assert_eq!(
            super::next_pinned_windows_visible_state(&[true, true]),
            Some(false)
        );
        assert_eq!(
            super::next_pinned_windows_visible_state(&[false, false]),
            Some(true)
        );
        assert_eq!(super::next_pinned_windows_visible_state(&[]), None);
    }

    #[test]
    fn plans_pinned_group_window_visibility_changes() {
        assert_eq!(
            super::pinned_group_window_visibility_changes(
                &["pin-1".to_string()],
                &["pin-2".to_string(), "pin-3".to_string()]
            ),
            vec![
                PinnedWindowVisibilityChange {
                    label: "pin-pin-1".to_string(),
                    visible: false,
                },
                PinnedWindowVisibilityChange {
                    label: "pin-pin-2".to_string(),
                    visible: true,
                },
                PinnedWindowVisibilityChange {
                    label: "pin-pin-3".to_string(),
                    visible: true,
                },
            ]
        );
    }

    #[test]
    fn plans_moved_pinned_image_to_hide_current_window() {
        assert_eq!(
            super::moved_pinned_image_window_visibility_change("pin-1"),
            PinnedWindowVisibilityChange {
                label: "pin-pin-1".to_string(),
                visible: false,
            }
        );
    }

    #[test]
    fn plans_destroyed_pinned_group_windows_to_close() {
        assert_eq!(
            super::destroyed_pinned_group_window_labels(&[
                "pin-2".to_string(),
                "pin-1".to_string(),
            ]),
            vec!["pin-pin-2".to_string(), "pin-pin-1".to_string()]
        );
    }

    #[test]
    fn plans_hidden_pinned_group_windows_to_hide() {
        assert_eq!(
            super::hidden_pinned_group_window_visibility_changes(&[
                "pin-2".to_string(),
                "pin-1".to_string(),
            ]),
            vec![
                PinnedWindowVisibilityChange {
                    label: "pin-pin-2".to_string(),
                    visible: false,
                },
                PinnedWindowVisibilityChange {
                    label: "pin-pin-1".to_string(),
                    visible: false,
                },
            ]
        );
    }

    #[test]
    fn pinned_window_size_preserves_aspect_ratio_with_cap() {
        assert_eq!(super::pinned_window_size(300, 200), (300.0, 200.0));
        assert_eq!(super::pinned_window_size(1800, 900), (900.0, 450.0));
    }
}
