use std::path::PathBuf;

pub(super) const RESULT_WINDOW_LABEL: &str = "capture-result";

pub(super) struct ResultWindowDefinition {
    pub(super) label: &'static str,
    pub(super) url: PathBuf,
    pub(super) title: &'static str,
    pub(super) inner_size: (f64, f64),
    pub(super) position: (f64, f64),
    pub(super) decorations: bool,
    pub(super) always_on_top: bool,
    pub(super) visible_on_all_workspaces: bool,
    pub(super) transparent: bool,
    pub(super) visible: bool,
    pub(super) skip_taskbar: bool,
    pub(super) focused: bool,
    pub(super) shadow: bool,
}

pub(super) fn result_window_definition() -> ResultWindowDefinition {
    ResultWindowDefinition {
        label: RESULT_WINDOW_LABEL,
        url: PathBuf::from("index.html?window=capture-result"),
        title: "SnapLingo Result",
        inner_size: (660.0, 660.0),
        position: (120.0, 120.0),
        decorations: false,
        always_on_top: true,
        visible_on_all_workspaces: true,
        transparent: true,
        visible: false,
        skip_taskbar: true,
        focused: false,
        shadow: true,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn result_window_definition_matches_capture_result_contract() {
        let definition = super::result_window_definition();

        assert_eq!(
            definition.url.to_string_lossy(),
            "index.html?window=capture-result"
        );
        assert_eq!(definition.label, "capture-result");
        assert_eq!(definition.title, "SnapLingo Result");
        assert_eq!(definition.inner_size, (660.0, 660.0));
        assert_eq!(definition.position, (120.0, 120.0));
        assert!(!definition.decorations);
        assert!(definition.always_on_top);
        assert!(definition.visible_on_all_workspaces);
        assert!(definition.transparent);
        assert!(!definition.visible);
        assert!(definition.skip_taskbar);
        assert!(!definition.focused);
        assert!(definition.shadow);
    }
}
