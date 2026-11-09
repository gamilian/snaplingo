use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};
use syn::{
    Attribute, ExprBlock, Field, ForeignItem, ImplItem, Item, ItemUse, TraitItem, UseTree, Variant,
};

const FORBIDDEN_APPLICATION_DEPENDENCIES: &[&str] = &[
    "infrastructure",
    "commands",
    "composition",
    "app_actions",
    "app_shell",
    "settings_window",
    "startup_shortcuts",
    "tauri",
    "arboard",
    "enigo",
    "windows",
    "objc2",
    "rusqlite",
];

#[derive(Clone)]
struct SourceFile {
    path: String,
    source: String,
}

#[derive(Default)]
struct DependencyVisitor {
    dependencies: BTreeSet<String>,
}

macro_rules! enum_has_cfg_test {
    ($item:expr, $kind:ident: $($variant:ident),+ $(,)?) => {
        match $item {
            $($kind::$variant(item) => has_cfg_test(&item.attrs),)+
            _ => false,
        }
    };
}

impl<'ast> Visit<'ast> for DependencyVisitor {
    fn visit_item(&mut self, item: &'ast Item) {
        if !enum_has_cfg_test!(
            item,
            Item: Const, Enum, ExternCrate, Fn, ForeignMod, Impl, Macro, Mod, Static, Struct,
            Trait, TraitAlias, Type, Union, Use,
        ) {
            visit::visit_item(self, item);
        }
    }

    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        expand_use_tree(&item.tree, Vec::new(), &mut self.dependencies);
    }

    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        if !enum_has_cfg_test!(item, ImplItem: Const, Fn, Type, Macro) {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast TraitItem) {
        if !enum_has_cfg_test!(item, TraitItem: Const, Fn, Type, Macro) {
            visit::visit_trait_item(self, item);
        }
    }

    fn visit_foreign_item(&mut self, item: &'ast ForeignItem) {
        if !enum_has_cfg_test!(item, ForeignItem: Fn, Static, Type, Macro) {
            visit::visit_foreign_item(self, item);
        }
    }

    fn visit_variant(&mut self, variant: &'ast Variant) {
        if !has_cfg_test(&variant.attrs) {
            visit::visit_variant(self, variant);
        }
    }

    fn visit_field(&mut self, field: &'ast Field) {
        if !has_cfg_test(&field.attrs) {
            visit::visit_field(self, field);
        }
    }

    fn visit_expr_block(&mut self, block: &'ast ExprBlock) {
        if !has_cfg_test(&block.attrs) {
            visit::visit_expr_block(self, block);
        }
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        self.record_path(path);
        visit::visit_path(self, path);
    }
}

impl DependencyVisitor {
    fn record_path(&mut self, path: &syn::Path) {
        let segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        record_dependency(segments, &mut self.dependencies);
    }
}

fn production_application_sources() -> Vec<SourceFile> {
    let manifest_dir = core_root();
    let mut paths = Vec::new();
    collect_rust_files(&manifest_dir.join("src/application"), &mut paths);
    paths.sort();

    paths
        .into_iter()
        .filter(|path| is_production_rust(path))
        .map(|path| SourceFile {
            path: path
                .strip_prefix(&manifest_dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            source: fs::read_to_string(&path).unwrap(),
        })
        .collect()
}

fn core_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../core")
}

#[test]
fn portable_core_has_no_concrete_adapter_dependencies_or_source_mounts() {
    let root = core_root();
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    for forbidden in [
        "tauri",
        "objc2",
        "arboard",
        "enigo",
        "snaplingo-storage",
        "snaplingo-app",
        "reqwest",
        "rusqlite",
        "windows =",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "Core manifest must not depend on {forbidden}"
        );
    }
    let syntax = syn::parse_file(&fs::read_to_string(root.join("src/lib.rs")).unwrap()).unwrap();
    for item in &syntax.items {
        if let Item::Mod(module) = item {
            if module.ident == "infrastructure" {
                assert!(
                    has_cfg_test(&module.attrs),
                    "Real adapter fixtures belong exclusively to core unit tests"
                );
            }
        }
    }
    let mut visitor = DependencyVisitor::default();
    visitor.visit_file(&syntax);
    assert!(visitor
        .dependencies
        .iter()
        .all(|dependency| !dependency.contains("infrastructure")));
}

fn production_app_sources() -> Vec<SourceFile> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    collect_rust_files(&root.join("src"), &mut paths);
    paths.sort();
    paths
        .into_iter()
        .filter(|path| is_production_rust(path))
        .map(|path| SourceFile {
            path: path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            source: fs::read_to_string(path).unwrap(),
        })
        .collect()
}

fn workflow_adapter_sources() -> Vec<SourceFile> {
    let mut sources = production_app_sources()
        .into_iter()
        .filter(|source| {
            source.path == "src/infrastructure/system/result_window/controller.rs"
                || source.path == "src/app_actions.rs"
        })
        .collect::<Vec<_>>();
    let settings = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../settings/src");
    let mut paths = Vec::new();
    collect_rust_files(&settings, &mut paths);
    sources.extend(
        paths
            .into_iter()
            .filter(|path| is_production_rust(path))
            .map(|path| SourceFile {
                path: format!(
                    "settings/{}",
                    path.strip_prefix(&settings).unwrap().to_string_lossy()
                ),
                source: fs::read_to_string(path).unwrap(),
            }),
    );
    assert!(sources
        .iter()
        .any(|source| source.path.ends_with("/services.rs")));
    assert!(sources
        .iter()
        .any(|source| source.path.ends_with("/library.rs")));
    assert!(sources
        .iter()
        .any(|source| source.path.ends_with("/controller.rs")));
    sources
}

#[test]
fn native_production_sources_do_not_depend_on_a_web_runtime() {
    let sources = production_app_sources();
    assert!(sources
        .iter()
        .any(|source| source.path == "src/native_app.rs"));
    assert!(sources
        .iter()
        .any(|source| source.path == "src/native_windows.rs"));
    assert_eq!(
        dependency_inventory_with_rules(&sources, &["tauri", "objc2_web_kit", "wry"]),
        BTreeSet::new()
    );
}

fn collect_rust_files(directory: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rust_files(&path, paths);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            paths.push(path);
        }
    }
}

fn is_production_rust(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    name != "tests.rs" && !name.ends_with("_test.rs") && !name.ends_with("_tests.rs")
}

fn dependency_inventory(files: &[SourceFile]) -> BTreeSet<String> {
    dependency_inventory_with_rules(files, FORBIDDEN_APPLICATION_DEPENDENCIES)
}

fn dependency_inventory_with_rules(files: &[SourceFile], forbidden: &[&str]) -> BTreeSet<String> {
    files
        .iter()
        .flat_map(|file| {
            let syntax = syn::parse_file(&file.source)
                .unwrap_or_else(|error| panic!("failed to parse {}: {error}", file.path));
            let mut visitor = DependencyVisitor::default();
            visitor.visit_file(&syntax);
            visitor
                .dependencies
                .into_iter()
                .filter(|dependency| {
                    let dependency = dependency.strip_prefix("crate::").unwrap_or(dependency);
                    forbidden.contains(&dependency.split("::").next().unwrap_or(""))
                })
                .map(|dependency| format!("{} -> {dependency}", file.path))
                .collect::<Vec<_>>()
        })
        .collect::<BTreeSet<_>>()
}

fn expand_use_tree(tree: &UseTree, mut prefix: Vec<String>, output: &mut BTreeSet<String>) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            expand_use_tree(&path.tree, prefix, output);
        }
        UseTree::Name(name) => {
            if name.ident != "self" {
                prefix.push(name.ident.to_string());
            }
            record_dependency(prefix, output);
        }
        UseTree::Rename(rename) => {
            prefix.push(rename.ident.to_string());
            record_dependency(prefix, output);
        }
        UseTree::Glob(_) => {
            prefix.push("*".to_string());
            record_dependency(prefix, output);
        }
        UseTree::Group(group) => {
            for item in &group.items {
                expand_use_tree(item, prefix.clone(), output);
            }
        }
    }
}

fn record_dependency(path: Vec<String>, output: &mut BTreeSet<String>) {
    output.insert(path.join("::"));
}

fn has_cfg_test(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && attribute
                .parse_args::<syn::Ident>()
                .is_ok_and(|argument| argument == "test")
    })
}

#[test]
fn application_production_sources_do_not_import_outward_adapters() {
    assert_eq!(
        dependency_inventory(&production_application_sources()),
        BTreeSet::<String>::new()
    );
}

#[test]
fn domain_sources_remain_independent_of_workflows_and_native_adapters() {
    let manifest_dir = core_root();
    let mut paths = Vec::new();
    collect_rust_files(&manifest_dir.join("src/domain"), &mut paths);
    let files = paths
        .into_iter()
        .filter(|path| is_production_rust(path))
        .map(|path| SourceFile {
            path: path
                .strip_prefix(&manifest_dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            source: fs::read_to_string(path).unwrap(),
        })
        .collect::<Vec<_>>();
    let mut forbidden = FORBIDDEN_APPLICATION_DEPENDENCIES.to_vec();
    forbidden.extend(["application", "app_state"]);
    assert!(dependency_inventory_with_rules(&files, &forbidden).is_empty());
    let example = SourceFile { path: "domain/example.rs".into(), source: "use crate::application::SettingsConfiguration; use tauri::Manager; use serde::Serialize;".into() };
    assert_eq!(
        dependency_inventory_with_rules(&[example], &forbidden),
        BTreeSet::from([
            "domain/example.rs -> crate::application::SettingsConfiguration".into(),
            "domain/example.rs -> tauri::Manager".into(),
        ])
    );
}

#[test]
fn native_action_routing_does_not_publish_durable_state_change_events() {
    let sources = production_app_sources();
    let router = sources
        .iter()
        .find(|source| source.path == "src/app_actions.rs")
        .unwrap();
    let production = router.source.split("#[cfg(test)]").next().unwrap();
    for changed in [
        "SettingsChanged",
        "HotkeysChanged",
        "ProvidersChanged",
        "HistoryChanged",
        "FavoritesChanged",
        "ScreenshotFavoritesChanged",
    ] {
        assert!(
            !production.contains(changed),
            "Action routing must leave durable {changed} notifications to application services"
        );
    }
}

#[test]
fn shared_workflow_adapters_keep_os_and_database_mechanics_in_their_ports() {
    assert_eq!(
        dependency_inventory_with_rules(
            &workflow_adapter_sources(),
            &[
                "tauri",
                "objc2",
                "objc2_app_kit",
                "windows",
                "rfd",
                "arboard",
                "enigo",
                "rusqlite",
                "snaplingo_storage",
            ]
        ),
        BTreeSet::new()
    );
}

#[test]
fn native_capture_actions_delegate_startup_exclusion_and_pin_delivery_to_the_runtime() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let router = fs::read_to_string(root.join("src/app_actions.rs")).unwrap();
    let production = router.split("#[cfg(test)]").next().unwrap();
    for policy in [
        "compare_exchange",
        "CaptureSessionOutput",
        "pin_png_and_open",
        ".startup.try_lock",
    ] {
        assert!(
            !production.contains(policy),
            "Native actions must delegate {policy} to the application runtime"
        );
    }
    for request in [".open_capture_window_for_mode(", ".pin_from_shortcut("] {
        assert!(
            production.contains(request),
            "Native capture action must reach the shared runtime through {request}"
        );
    }
    let runtime =
        fs::read_to_string(core_root().join("src/application/capture/runtime.rs")).unwrap();
    let production = runtime.split("#[cfg(test)]").next().unwrap();
    assert!(
        !production.contains("Capture window is not open"),
        "Capture runtime must rely on idempotent hide behavior, not adapter error text"
    );
}

#[test]
fn capture_window_and_control_adapters_use_frozen_session_coordinates() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for path in [
        "src/infrastructure/system/capture_window/native_host.rs",
        "src/infrastructure/system/capture_window/windows.rs",
        "src/infrastructure/system/capture_window/windows/view.rs",
    ] {
        let window = fs::read_to_string(manifest_dir.join(path)).unwrap();
        for live_read in [".primary_monitor(", ".available_monitors("] {
            assert!(
                !window.contains(live_read),
                "{path} must use supplied session geometry instead of {live_read}"
            );
        }
    }

    struct RejectLiveMonitorEnumeration;
    impl<'ast> Visit<'ast> for RejectLiveMonitorEnumeration {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            for segment in &path.segments {
                assert!(
                    segment.ident != "capture_all_monitor_layouts"
                        && segment.ident != "capture_all_monitor_snapshots",
                    "Control detection must map through the supplied frozen monitors"
                );
            }
            visit::visit_path(self, path);
        }
    }
    let windows_source =
        fs::read_to_string(manifest_dir.join("src/infrastructure/system/screenshot/windows.rs"))
            .unwrap();
    let syntax = syn::parse_file(&windows_source).unwrap();
    let control_mapping = syntax
        .items
        .iter()
        .find_map(|item| match item {
            Item::Fn(function) if function.sig.ident == "capture_control_candidate_at" => {
                Some(function)
            }
            _ => None,
        })
        .expect("Windows control mapping must be covered by the frozen-coordinate guard");
    RejectLiveMonitorEnumeration.visit_item_fn(control_mapping);
    assert!(
        !windows_source.contains("normalize_windows_"),
        "The session's coordinate policy owns Windows monitor normalization"
    );
}

#[test]
fn native_adapters_and_composition_delegate_provider_and_ocr_policy_to_application_services() {
    let sources = workflow_adapter_sources();
    for path in ["settings/services.rs", "settings/library.rs"] {
        let source = &sources
            .iter()
            .find(|source| source.path == path)
            .unwrap()
            .source;
        for policy in [
            "validate_non_blank",
            ".llm_introspection",
            ".prompt_strategies",
            "load_baidu_ocr_credentials",
            "baidu_ocr_api_key",
            "baidu_ocr_secret_key",
            ".recognize_image(",
            ".read_ocr_source(",
        ] {
            assert!(
                !source.contains(policy),
                "{path} must delegate {policy} to its application service"
            );
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let composition = fs::read_to_string(root.join("src/composition/provider_runtime.rs")).unwrap();
    for policy in [
        "load_baidu_ocr_credentials",
        "baidu_ocr_api_key",
        "baidu_ocr_secret_key",
    ] {
        assert!(
            !composition.contains(policy),
            "Provider composition must not own {policy}"
        );
    }
}

#[test]
fn composition_is_the_only_platform_adapter_selector() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let read = |path: &str| fs::read_to_string(manifest_dir.join(path)).unwrap();
    let mut violations = Vec::new();

    for source in workflow_adapter_sources() {
        if source.source.contains("NativeHotkeyRegistrar") {
            violations.push(format!("{} constructs NativeHotkeyRegistrar", source.path));
        }
    }

    let lib_source = read("src/lib.rs");
    if lib_source.contains("NativeHotkeyRegistrar") {
        violations.push("src/lib.rs constructs NativeHotkeyRegistrar".to_string());
    }

    let screenshot_module = read("src/infrastructure/system/screenshot/mod.rs");
    if screenshot_module.contains("get_capture_session_source") {
        violations.push(
            "src/infrastructure/system/screenshot/mod.rs selects a capture adapter".to_string(),
        );
    }

    let selection_module = read("src/infrastructure/system/selection/mod.rs");
    if selection_module.contains("platform_selection_provider") {
        violations.push(
            "src/infrastructure/system/selection/mod.rs selects a selection adapter".to_string(),
        );
    }

    let tts_module = read("src/infrastructure/system/tts/mod.rs");
    if tts_module.contains("build_system_tts_host") {
        violations
            .push("src/infrastructure/system/tts/mod.rs selects a System TTS adapter".to_string());
    }
    for platform in ["macos", "windows", "linux"] {
        let path = format!("src/infrastructure/system/selection/{platform}/mod.rs");
        if read(&path).contains("fn platform_selection_provider") {
            violations.push(format!("{path} selects a selection adapter"));
        }
    }

    let capture_composition = read("src/composition/capture_runtime.rs");
    for adapter in ["MacOSCaptureSessionSource", "WindowsCaptureSessionSource"] {
        assert!(
            capture_composition.contains(adapter),
            "capture Composition must select {adapter}"
        );
    }

    let selection_composition = read("src/composition/selection_runtime.rs");
    for adapter in ["MacSelectionProvider", "PlatformSelectionProvider"] {
        assert!(
            selection_composition.contains(adapter),
            "selection Composition must select {adapter}"
        );
    }
    let tts_composition = read("src/composition/tts_runtime.rs");
    for adapter in [
        "MacOsSystemTtsHost",
        "WindowsSystemTtsHost",
        "UnavailableSystemTtsHost",
    ] {
        assert!(
            tts_composition.contains(adapter),
            "TTS Composition must select {adapter}"
        );
    }
    assert!(
        read("src/composition.rs").contains("NativeHotkeyRegistrar"),
        "root Composition must select NativeHotkeyRegistrar"
    );

    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn rejects_forbidden_synthetic_dependencies() {
    let files = [SourceFile {
        path: "src/application/example.rs".to_string(),
        source: r#"
            use crate::app_actions::AppAction;
            use crate::app_shell::apply_resting_activation_policy;
            use crate::commands::trigger_screenshot;
            use crate::composition::build_app_state;
            use crate::infrastructure::storage::{SqliteConfigStore, SqliteCredentialStore};
            use crate::settings_window::show_settings_window;
            use crate::startup_shortcuts::trigger_hotkey_action;
            fn call() { crate::infrastructure::http::send(); }
            // use crate::infrastructure::comments::Ignored;
            const TEXT: &str = "crate::infrastructure::strings::Ignored";
            #[cfg(test)]
            mod tests { use crate::infrastructure::tests::Ignored; }
            struct Example;
            impl Example {
                #[cfg(test)]
                fn test_only() { crate::infrastructure::tests::Ignored::call(); }
            }
            fn conditional_block() {
                #[cfg(test)]
                { crate::infrastructure::test_blocks::Ignored::call(); }
            }
            trait ExampleTrait { #[cfg(test)] fn test_only(value: crate::infrastructure::trait_items::Ignored); }
            extern "C" { #[cfg(test)] fn test_only(value: crate::infrastructure::foreign_items::Ignored); }
            enum ExampleEnum { #[cfg(test)] TestOnly(crate::infrastructure::variants::Ignored), Production }
            struct ExampleFields { #[cfg(test)] test_only: crate::infrastructure::fields::Ignored, production: u8 }
            fn struct_expression() { let _ = crate::infrastructure::Foo { value: 1 }; }
            fn pattern(value: u8) { match value { crate::infrastructure::Enum::Variant => {}, _ => {} } }
            fn trait_bound<T: crate::infrastructure::SomeTraitBound>() {}
            impl crate::infrastructure::SomeImplTrait for Example {}
        "#
        .to_string(),
    }];

    assert_eq!(
        dependency_inventory(&files).into_iter().collect::<Vec<_>>(),
        vec![
            "src/application/example.rs -> crate::app_actions::AppAction",
            "src/application/example.rs -> crate::app_shell::apply_resting_activation_policy",
            "src/application/example.rs -> crate::commands::trigger_screenshot",
            "src/application/example.rs -> crate::composition::build_app_state",
            "src/application/example.rs -> crate::infrastructure::Enum::Variant",
            "src/application/example.rs -> crate::infrastructure::Foo",
            "src/application/example.rs -> crate::infrastructure::SomeImplTrait",
            "src/application/example.rs -> crate::infrastructure::SomeTraitBound",
            "src/application/example.rs -> crate::infrastructure::http::send",
            "src/application/example.rs -> crate::infrastructure::storage::SqliteConfigStore",
            "src/application/example.rs -> crate::infrastructure::storage::SqliteCredentialStore",
            "src/application/example.rs -> crate::settings_window::show_settings_window",
            "src/application/example.rs -> crate::startup_shortcuts::trigger_hotkey_action",
        ]
    );
}
