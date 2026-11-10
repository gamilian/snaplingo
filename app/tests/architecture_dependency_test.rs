use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};
use syn::{
    Attribute, ExprBlock, Field, ForeignItem, ImplItem, Item, ItemUse, TraitItem, UseTree, Variant,
};

const FORBIDDEN_APPLICATION_DEPENDENCIES: &[&str] = &[
    "integrations",
    "infrastructure",
    "bootstrap",
    "shell",
    "resident",
    "features",
    "snaplingo_platform",
    "snaplingo_providers",
    "snaplingo_storage",
    "snaplingo_ui",
    "commands",
    "composition",
    "actions",
    "lifecycle",
    "window",
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
    "dirs",
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
    collect_rust_files(&manifest_dir.join("src"), &mut paths);
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
    workspace_root().join("crates/core")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn app_source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
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
        "snaplingo-platform",
        "snaplingo-providers",
        "snaplingo-ui",
        "snaplingo-app",
        "reqwest",
        "rusqlite",
        "dirs =",
        "windows =",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "Core manifest must not depend on {forbidden}"
        );
    }
    let sources = production_application_sources();
    assert!(
        sources.len() > 40,
        "Core guard must inspect the feature implementations"
    );
    struct RejectSourceMounts {
        directory: PathBuf,
        crate_root: PathBuf,
    }
    impl<'ast> Visit<'ast> for RejectSourceMounts {
        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            assert!(
                !mac.path.is_ident("include"),
                "Core sources must own their implementation"
            );
            visit::visit_macro(self, mac);
        }
        fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
            for attribute in &module.attrs {
                if let syn::Meta::NameValue(value) = &attribute.meta {
                    if value.path.is_ident("path") {
                        if let syn::Expr::Lit(literal) = &value.value {
                            if let syn::Lit::Str(path) = &literal.lit {
                                let target =
                                    self.directory.join(path.value()).canonicalize().unwrap();
                                assert!(
                                    target.starts_with(&self.crate_root),
                                    "Core source mount leaves its owning crate: {}",
                                    target.display()
                                );
                            }
                        }
                    }
                }
            }
            visit::visit_item_mod(self, module);
        }
    }
    for source in sources {
        RejectSourceMounts {
            directory: root.join(&source.path).parent().unwrap().to_path_buf(),
            crate_root: root.canonicalize().unwrap(),
        }
        .visit_file(&syn::parse_file(&source.source).unwrap());
    }
}

fn production_app_sources() -> Vec<SourceFile> {
    let root = app_source_root();
    let mut paths = Vec::new();
    collect_rust_files(&root, &mut paths);
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
    let sources = production_app_sources()
        .into_iter()
        .filter(|source| {
            source.path == "features/result/controller.rs"
                || source.path == "features/capture/completion.rs"
                || source.path == "shell/actions/mod.rs"
                || source.path.starts_with("features/settings/presenter/")
        })
        .collect::<Vec<_>>();
    for required in [
        "features/settings/presenter/services.rs",
        "features/settings/presenter/library.rs",
        "features/result/controller.rs",
        "features/capture/completion.rs",
        "shell/actions/mod.rs",
    ] {
        assert!(
            sources.iter().any(|source| source.path == required),
            "Missing guarded workflow {required}"
        );
    }
    sources
}

#[test]
fn native_production_sources_do_not_depend_on_a_web_runtime() {
    let sources = production_app_sources();
    assert!(sources.iter().any(|source| source.path == "shell/macos.rs"));
    assert!(sources
        .iter()
        .any(|source| source.path == "shell/windows.rs"));
    assert_eq!(
        dependency_inventory_with_rules(&sources, &["tauri", "objc2_web_kit", "wry"]),
        BTreeSet::new()
    );
}

#[test]
fn app_and_storage_use_core_without_reexporting_its_module_tree() {
    let mut sources = production_app_sources();
    let storage = workspace_root().join("crates/storage/src");
    let mut paths = Vec::new();
    collect_rust_files(&storage, &mut paths);
    sources.extend(
        paths
            .into_iter()
            .filter(|path| is_production_rust(path))
            .map(|path| SourceFile {
                path: format!("storage/{}", path.strip_prefix(&storage).unwrap().display()),
                source: fs::read_to_string(path).unwrap(),
            }),
    );
    let indirect = sources
        .iter()
        .flat_map(|source| {
            let syntax = syn::parse_file(&source.source).unwrap();
            let mut visitor = DependencyVisitor::default();
            visitor.visit_file(&syntax);
            visitor
                .dependencies
                .into_iter()
                .filter(|path| {
                    path == "snaplingo_core::domain"
                        || path.starts_with("snaplingo_core::domain::")
                        || path == "snaplingo_core::application"
                        || path.starts_with("snaplingo_core::application::")
                        || path == "crate::domain"
                        || path.starts_with("crate::domain::")
                        || path == "crate::application"
                        || path.starts_with("crate::application::")
                })
                .map(|path| format!("{} -> {path}", source.path))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert!(
        indirect.is_empty(),
        "Core dependencies must name their owner: {indirect:#?}"
    );

    for name in ["lib.rs", "storage/lib.rs"] {
        let source = sources.iter().find(|source| source.path == name).unwrap();
        let syntax = syn::parse_file(&source.source).unwrap();
        for item in syntax.items {
            if let Item::Use(item) = item {
                if !matches!(item.vis, syn::Visibility::Public(_)) {
                    continue;
                }
                let mut exports = BTreeSet::new();
                expand_use_tree(&item.tree, Vec::new(), &mut exports);
                for export in exports {
                    assert!(
                        !export.starts_with("snaplingo_core::") || matches!(export.as_str(), "snaplingo_core::AppError" | "snaplingo_core::Result"),
                        "{name} must expose its own interface rather than Core's module tree: {export}"
                    );
                }
            }
        }
    }
}

#[derive(Default)]
struct ReferenceEmbeddings {
    paths: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for ReferenceEmbeddings {
    fn visit_item(&mut self, item: &'ast Item) {
        if !enum_has_cfg_test!(
            item,
            Item: Const, Enum, ExternCrate, Fn, ForeignMod, Impl, Macro, Mod, Static, Struct,
            Trait, TraitAlias, Type, Union, Use,
        ) {
            visit::visit_item(self, item);
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if matches!(
            mac.path.get_ident().map(ToString::to_string).as_deref(),
            Some("include" | "include_str" | "include_bytes")
        ) {
            let source = mac.tokens.to_string();
            if source.contains("reference/") {
                self.paths.insert(source);
            }
        }
        visit::visit_macro(self, mac);
    }
}

#[test]
fn production_resources_are_independent_of_browser_references() {
    let mut paths = Vec::new();
    collect_rust_files(&app_source_root(), &mut paths);
    for name in ["core", "ui", "platform", "storage", "assets"] {
        collect_rust_files(
            &workspace_root().join("crates").join(name).join("src"),
            &mut paths,
        );
    }
    let mut violations = Vec::new();
    for path in paths.into_iter().filter(|path| is_production_rust(path)) {
        let syntax = syn::parse_file(&fs::read_to_string(&path).unwrap()).unwrap();
        let mut visitor = ReferenceEmbeddings::default();
        visitor.visit_file(&syntax);
        for resource in visitor.paths {
            violations.push(format!("{} -> {resource}", path.display()));
        }
    }
    assert!(
        violations.is_empty(),
        "Reference resource coupling: {violations:#?}"
    );
}

fn collect_rust_files(directory: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some(
                    "target"
                        | "tests"
                        | "test_support"
                        | "examples"
                        | "assets"
                        | "icons"
                        | "resources"
                        | "gen"
                )
            ) {
                continue;
            }
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
                    if matches!(dependency.as_str(), "window" | "actions") {
                        return false;
                    }
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
    fn possible(meta: &syn::Meta) -> (bool, bool) {
        use syn::{punctuated::Punctuated, Meta, Token};
        match meta {
            Meta::Path(path) if path.is_ident("test") => (true, false),
            Meta::NameValue(value)
                if value.path.is_ident("feature")
                    && matches!(&value.value, syn::Expr::Lit(lit)
                        if matches!(&lit.lit, syn::Lit::Str(name) if name.value() == "test-support")) =>
            {
                (true, false)
            }
            Meta::List(list) => {
                let nested = list
                    .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                    .unwrap();
                let values = nested.iter().map(possible).collect::<Vec<_>>();
                if list.path.is_ident("all") {
                    (values.iter().any(|v| v.0), values.iter().all(|v| v.1))
                } else if list.path.is_ident("any") {
                    (values.iter().all(|v| v.0), values.iter().any(|v| v.1))
                } else if list.path.is_ident("not") {
                    assert_eq!(values.len(), 1);
                    (values[0].1, values[0].0)
                } else {
                    (true, true)
                }
            }
            _ => (true, true),
        }
    }
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && !possible(&attribute.parse_args::<syn::Meta>().unwrap()).1
    })
}

#[test]
fn application_production_sources_do_not_import_outward_adapters() {
    assert_eq!(
        dependency_inventory(&production_application_sources()),
        BTreeSet::<String>::new()
    );
}

fn model_workflow_imports(sources: &[SourceFile]) -> BTreeSet<String> {
    sources
        .iter()
        .flat_map(|source| {
            let mut visitor = DependencyVisitor::default();
            visitor.visit_file(&syn::parse_file(&source.source).unwrap());
            visitor
                .dependencies
                .into_iter()
                .filter(|path| {
                    let parts = path.split("::").collect::<Vec<_>>();
                    matches!(parts[0], "crate" | "super")
                        && !matches!(parts.get(1), Some(&"AppError" | &"Result" | &"error"))
                        && !parts.contains(&"model")
                })
                .map(|path| format!("{} -> {path}", source.path))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn feature_models_remain_independent_of_workflows_and_native_adapters() {
    let sources = production_application_sources()
        .into_iter()
        .filter(|source| {
            source.path.ends_with("/model.rs") || source.path == "src/providers/events.rs"
        })
        .collect::<Vec<_>>();
    assert!(
        sources.len() >= 7,
        "All feature models and provider events must remain guarded"
    );
    assert!(dependency_inventory(&sources).is_empty());
    assert!(model_workflow_imports(&sources).is_empty());
    let example = SourceFile {
        path: "capture/model.rs".into(),
        source: "use crate::capture::CaptureSessionRuntime; use crate::providers::ocr::model::OcrResult; use crate::Result; use tauri::Manager;".into(),
    };
    assert_eq!(
        model_workflow_imports(&[example.clone()]),
        BTreeSet::from(["capture/model.rs -> crate::capture::CaptureSessionRuntime".into()])
    );
    assert_eq!(
        dependency_inventory(&[example]),
        BTreeSet::from(["capture/model.rs -> tauri::Manager".into()])
    );
}

#[test]
fn native_action_routing_does_not_publish_durable_state_change_events() {
    let sources = production_app_sources();
    let router = sources
        .iter()
        .find(|source| source.path == "shell/actions/mod.rs")
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
    let root = app_source_root();
    let router = fs::read_to_string(root.join("shell/actions/mod.rs")).unwrap();
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
    let runtime = fs::read_to_string(core_root().join("src/capture/runtime.rs")).unwrap();
    let production = runtime.split("#[cfg(test)]").next().unwrap();
    assert!(
        !production.contains("Capture window is not open"),
        "Capture runtime must rely on idempotent hide behavior, not adapter error text"
    );
}

#[test]
fn capture_window_and_control_adapters_use_frozen_session_coordinates() {
    let manifest_dir = app_source_root();
    for path in [
        "features/capture/macos/host.rs",
        "features/capture/windows/mod.rs",
        "features/capture/windows/view.rs",
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
        fs::read_to_string(workspace_root().join("crates/platform/src/screenshot/windows.rs"))
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
    for path in [
        "features/settings/presenter/services.rs",
        "features/settings/presenter/library.rs",
    ] {
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
    let root = app_source_root();
    let composition = fs::read_to_string(root.join("bootstrap/provider_runtime.rs")).unwrap();
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
    let manifest_dir = app_source_root();
    let read = |path: &str| fs::read_to_string(manifest_dir.join(path)).unwrap();
    let mut violations = Vec::new();

    for source in workflow_adapter_sources() {
        if source.source.contains("NativeHotkeyRegistrar") {
            violations.push(format!("{} constructs NativeHotkeyRegistrar", source.path));
        }
    }

    let lib_source = read("lib.rs");
    if lib_source.contains("NativeHotkeyRegistrar") {
        violations.push("lib.rs constructs NativeHotkeyRegistrar".to_string());
    }

    let read_platform = |path: &str| {
        fs::read_to_string(workspace_root().join("crates/platform/src").join(path)).unwrap()
    };
    let screenshot_module = read_platform("screenshot/mod.rs");
    if screenshot_module.contains("get_capture_session_source") {
        violations
            .push("infrastructure/system/screenshot/mod.rs selects a capture adapter".to_string());
    }

    let selection_module = read_platform("selection/mod.rs");
    if selection_module.contains("platform_selection_provider") {
        violations
            .push("infrastructure/system/selection/mod.rs selects a selection adapter".to_string());
    }

    let tts_module = read_platform("tts/mod.rs");
    if tts_module.contains("build_system_tts_host") {
        violations
            .push("infrastructure/system/tts/mod.rs selects a System TTS adapter".to_string());
    }
    for platform in ["macos", "windows", "linux"] {
        let path = format!("selection/{platform}/mod.rs");
        if read_platform(&path).contains("fn platform_selection_provider") {
            violations.push(format!("{path} selects a selection adapter"));
        }
    }

    let capture_composition = read("bootstrap/capture_runtime.rs");
    for adapter in ["MacOSCaptureSessionSource", "WindowsCaptureSessionSource"] {
        assert!(
            capture_composition.contains(adapter),
            "capture Composition must select {adapter}"
        );
    }

    for (entry, adapter) in [
        ("shell/macos.rs", "MacSelectionProvider"),
        ("shell/windows.rs", "PlatformSelectionProvider"),
    ] {
        assert!(
            read(entry).contains(adapter),
            "native startup must select {adapter}"
        );
    }
    let tts_composition = read("bootstrap/tts_runtime.rs");
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
        read("bootstrap/mod.rs").contains("NativeHotkeyRegistrar"),
        "root Composition must select NativeHotkeyRegistrar"
    );

    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn rejects_forbidden_synthetic_dependencies() {
    let files = [SourceFile {
        path: "application/example.rs".to_string(),
        source: r#"
            use crate::shell::actions::AppAction;
            use crate::resident::apply_resting_activation_policy;
            use crate::commands::trigger_screenshot;
            use crate::bootstrap::build_app_state;
            use snaplingo_storage::SqliteConfigStore;
use snaplingo_storage::SqliteCredentialStore;
            use crate::settings_window::show_settings_window;
            use crate::shell::actions::shortcuts::trigger_hotkey_action;
            use crate::features::capture::NativeCaptureSessionRuntimeHost;
            fn local(window: u8) -> u8 { window }
            fn call() { crate::integrations::http::send(); }
            // use crate::integrations::comments::Ignored;
            const TEXT: &str = "crate::integrations::strings::Ignored";
            #[cfg(test)]
            mod tests { use crate::integrations::tests::Ignored; }
            struct Example;
            impl Example {
                #[cfg(test)]
                fn test_only() { crate::integrations::tests::Ignored::call(); }
            }
            fn conditional_block() {
                #[cfg(test)]
                { crate::integrations::test_blocks::Ignored::call(); }
            }
            trait ExampleTrait { #[cfg(test)] fn test_only(value: crate::integrations::trait_items::Ignored); }
            extern "C" { #[cfg(test)] fn test_only(value: crate::integrations::foreign_items::Ignored); }
            enum ExampleEnum { #[cfg(test)] TestOnly(crate::integrations::variants::Ignored), Production }
            struct ExampleFields { #[cfg(test)] test_only: crate::integrations::fields::Ignored, production: u8 }
            fn struct_expression() { let _ = crate::integrations::Foo { value: 1 }; }
            fn pattern(value: u8) { match value { crate::integrations::Enum::Variant => {}, _ => {} } }
            fn trait_bound<T: crate::integrations::SomeTraitBound>() {}
            impl crate::integrations::SomeImplTrait for Example {}
        "#
        .to_string(),
    }];

    assert_eq!(
        dependency_inventory(&files),
        [
            "application/example.rs -> crate::shell::actions::AppAction",
            "application/example.rs -> crate::resident::apply_resting_activation_policy",
            "application/example.rs -> crate::commands::trigger_screenshot",
            "application/example.rs -> crate::bootstrap::build_app_state",
            "application/example.rs -> crate::integrations::Enum::Variant",
            "application/example.rs -> crate::integrations::Foo",
            "application/example.rs -> crate::integrations::SomeImplTrait",
            "application/example.rs -> crate::integrations::SomeTraitBound",
            "application/example.rs -> crate::integrations::http::send",
            "application/example.rs -> snaplingo_storage::SqliteConfigStore",
            "application/example.rs -> snaplingo_storage::SqliteCredentialStore",
            "application/example.rs -> crate::settings_window::show_settings_window",
            "application/example.rs -> crate::shell::actions::shortcuts::trigger_hotkey_action",
            "application/example.rs -> crate::features::capture::NativeCaptureSessionRuntimeHost",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
    );
}

fn feature_dependency_violations(sources: &[SourceFile]) -> BTreeSet<String> {
    sources
        .iter()
        .filter(|source| source.path.starts_with("features/"))
        .flat_map(|source| {
            let mut visitor = DependencyVisitor::default();
            visitor.visit_file(&syn::parse_file(&source.source).unwrap());
            visitor
                .dependencies
                .into_iter()
                .filter(|path| {
                    path != "shell"
                        && path.split("::").any(|part| {
                            matches!(
                                part,
                                "AppState"
                                    | "bootstrap"
                                    | "composition"
                                    | "shell"
                                    | "lifecycle"
                                    | "app_state"
                            )
                        })
                })
                .map(|path| format!("{} -> {path}", source.path))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn feature_hosts_receive_dependencies_without_resolving_application_state() {
    let sources = production_app_sources();
    for feature in ["settings", "capture", "result", "pinned", "scrolling"] {
        assert!(sources
            .iter()
            .any(|source| source.path == format!("features/{feature}/mod.rs")));
        for platform in ["macos", "windows"] {
            assert!(sources.iter().any(|source| source
                .path
                .starts_with(&format!("features/{feature}/{platform}/"))));
        }
    }
    assert_eq!(feature_dependency_violations(&sources), BTreeSet::new());
    let examples = [SourceFile {
        path: "features/capture/macos/host.rs".into(),
        source: "use crate::bootstrap as assembly; use crate::AppState as Global; type State = super::super::bootstrap::State; #[cfg(test)] mod tests { use crate::AppState; }".into(),
    }];
    assert_eq!(
        feature_dependency_violations(&examples),
        BTreeSet::from([
            "features/capture/macos/host.rs -> crate::bootstrap".into(),
            "features/capture/macos/host.rs -> crate::AppState".into(),
            "features/capture/macos/host.rs -> super::super::bootstrap::State".into(),
        ])
    );
}

#[test]
fn adapters_are_imported_from_their_owning_crates() {
    let moved = [
        "annotation_fonts",
        "capture_cursor",
        "capture_output",
        "clipboard_text",
        "hotkeys",
        "hotkey_registrar",
        "hotkey_recording",
        "ocr",
        "paths",
        "required_permissions",
        "screenshot",
        "screenshot_favorites",
        "selection",
        "shortcut_native",
        "single_instance",
        "start_on_boot",
        "tts",
    ];
    let mut violations = BTreeSet::new();
    for source in production_app_sources() {
        let syntax = syn::parse_file(&source.source).unwrap();
        let mut visitor = DependencyVisitor::default();
        visitor.visit_file(&syntax);
        for path in visitor.dependencies {
            let old_system = path
                .strip_prefix("crate::integrations::system::")
                .is_some_and(|tail| moved.contains(&tail.split("::").next().unwrap()));
            if old_system
                || ["storage", "http", "llm", "ocr"]
                    .iter()
                    .any(|module| path.starts_with(&format!("crate::integrations::{module}")))
            {
                violations.insert(format!("{} -> {path}", source.path));
            }
        }
        // An alias at the integration root would recreate an indirect ownership path.
        if source.path == "integrations/mod.rs" {
            for item in syntax.items {
                if let Item::Use(item) = item {
                    let mut paths = BTreeSet::new();
                    expand_use_tree(&item.tree, Vec::new(), &mut paths);
                    if matches!(item.vis, syn::Visibility::Public(_)) {
                        for path in paths {
                            if path.starts_with("snaplingo_platform")
                                || path.starts_with("snaplingo_storage")
                                || path.starts_with("snaplingo_providers")
                            {
                                violations.insert(format!("{} re-exports {path}", source.path));
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "Adapter ownership must remain explicit: {violations:#?}"
    );
}

#[test]
fn cfg_filter_checks_all_production_targets_without_loading_test_fixtures() {
    let source = SourceFile {
        path: "capture/example.rs".into(),
        source: r#"
            #[cfg(all(test, target_os = "macos"))]
            use objc2::Ignored;
            #[cfg(feature = "test-support")]
            use crate::bootstrap::OnlyForNativeAcceptance;
            #[cfg(any(test, target_os = "windows"))]
            use windows::Required;
            #[cfg(not(test))]
            use snaplingo_storage::Database;
        "#
        .into(),
    };
    assert_eq!(
        dependency_inventory(&[source]),
        BTreeSet::from([
            "capture/example.rs -> windows::Required".into(),
            "capture/example.rs -> snaplingo_storage::Database".into(),
        ])
    );
}

#[derive(Default)]
struct IntegrationSourceMounts {
    violations: BTreeSet<String>,
}
impl<'ast> Visit<'ast> for IntegrationSourceMounts {
    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        if attribute.path().is_ident("path") {
            if let syn::Meta::NameValue(value) = &attribute.meta {
                if let syn::Expr::Lit(lit) = &value.value {
                    if let syn::Lit::Str(path) = &lit.lit {
                        // Test fixtures can use paths within tests/, but cannot
                        // mount a parent source tree under another module graph.
                        let path = path.value().replace('\\', "/");
                        if Path::new(&path).is_absolute()
                            || path.split('/').any(|part| part == "..")
                        {
                            self.violations.insert(path);
                        }
                    }
                }
            }
        }
        visit::visit_attribute(self, attribute);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac.path.is_ident("include") {
            self.violations.insert("include! source mount".into());
        }
        visit::visit_macro(self, mac);
    }
}

#[test]
fn native_acceptance_links_the_application_instead_of_remounting_its_sources() {
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut sources = Vec::new();
    collect_rust_files(&tests, &mut sources);
    collect_rust_files(&app_source_root().join("test_support"), &mut sources);
    for path in sources {
        let mut mounts = IntegrationSourceMounts::default();
        mounts.visit_file(&syn::parse_file(&fs::read_to_string(&path).unwrap()).unwrap());
        assert!(
            mounts.violations.is_empty(),
            "{}: {:?}",
            path.display(),
            mounts.violations
        );
    }
    let mut mounts = IntegrationSourceMounts::default();
    mounts.visit_file(
        &syn::parse_file(
            r#"
        #[cfg(target_os = "windows")]
        #[path = "../src/shell/mod.rs"] mod copied;
        include!(concat!(env!("OUT_DIR"), "/generated_composition.rs"));
        #[path = "core_workflows/history.rs"] mod fixture;
    "#,
        )
        .unwrap(),
    );
    assert_eq!(
        mounts.violations,
        BTreeSet::from(["../src/shell/mod.rs".into(), "include! source mount".into(),])
    );
    let build =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build.rs")).unwrap();
    assert!(
        !build.contains("native_ui_composition"),
        "Build must not generate a second composition module graph"
    );

    let manifest =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    for target in [
        "native_app_smoke",
        "native_ui_workflows",
        "pinned_window_native",
        "scrolling_controls_native",
        "scrolling_overlays_native",
        "selected_text_native",
        "settings_features_native",
        "settings_general_native",
        "settings_hotkeys_native",
        "ui_thread_native",
    ] {
        let section = manifest
            .split("[[test]]")
            .find(|section| section.contains(&format!("name = \"{target}\"")))
            .unwrap_or_else(|| {
                panic!("Native acceptance target {target} must remain independently runnable")
            });
        let section = section.split("[[").next().unwrap();
        assert!(section.contains("harness = false"));
        assert!(
            section.contains("\"test-support\""),
            "{target} must explicitly select the native acceptance implementation"
        );
        let entry = fs::read_to_string(tests.join(format!("{target}.rs"))).unwrap();
        assert!(
            entry.contains("snaplingo_lib::test_support"),
            "{target} must call the linked scenario runner"
        );
    }
    let syntax =
        syn::parse_file(&fs::read_to_string(app_source_root().join("lib.rs")).unwrap()).unwrap();
    let support = syntax
        .items
        .iter()
        .find_map(|item| match item {
            Item::Mod(module) if module.ident == "test_support" => Some(module),
            _ => None,
        })
        .expect("The scenario runner must belong to the application");
    assert!(
        has_cfg_test(&support.attrs),
        "Native acceptance must be absent from ordinary product builds"
    );
    let default = manifest
        .lines()
        .find(|line| line.starts_with("default = "))
        .unwrap();
    assert!(!default.contains("test-support"));
}
