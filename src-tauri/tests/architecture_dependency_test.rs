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
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
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

fn production_command_sources() -> Vec<SourceFile> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    collect_rust_files(&manifest_dir.join("src/commands"), &mut paths);
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
    name != "tests.rs" && !name.ends_with("_test.rs")
}

fn dependency_inventory(files: &[SourceFile]) -> BTreeSet<String> {
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
    let is_forbidden = path.first().map(String::as_str) == Some("crate")
        && path
            .get(1)
            .is_some_and(|module| FORBIDDEN_APPLICATION_DEPENDENCIES.contains(&module.as_str()));

    if is_forbidden {
        output.insert(path.join("::"));
    }
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
fn commands_do_not_publish_durable_state_change_events() {
    let violations = production_command_sources()
        .into_iter()
        .filter(|file| {
            file.source.contains("emit_state_changed")
                || file.source.contains("settings-changed")
                || file.source.contains("hotkeys-changed")
                || file.source.contains("providers-changed")
                || file.source.contains("history-changed")
        })
        .map(|file| file.path)
        .collect::<Vec<_>>();

    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn commands_delegate_native_effects_to_application_seams() {
    let violations = production_command_sources()
        .into_iter()
        .filter(|file| {
            file.source.contains("enigo::")
                || file.source.contains("tauri_plugin_autostart")
                || file
                    .source
                    .contains("crate::infrastructure::system::capture_window")
                || file.source.contains(".logs.repository")
                || file.source.contains("run_cleanup()")
        })
        .map(|file| file.path)
        .collect::<Vec<_>>();

    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn commands_and_composition_do_not_own_provider_or_ocr_favorite_policy() {
    let command_sources = production_command_sources();
    let forbidden_command_policy = [
        ("src/commands/provider_commands.rs", "LLMProtocol"),
        ("src/commands/provider_commands.rs", "validate_non_blank"),
        ("src/commands/provider_commands.rs", ".llm_introspection"),
        ("src/commands/provider_commands.rs", ".configuration"),
        ("src/commands/provider_commands.rs", ".prompt_strategies"),
        ("src/commands/ocr_commands.rs", ".ocr_configuration"),
        ("src/commands/ocr_commands.rs", ".list_all()"),
        ("src/commands/ocr_commands.rs", ".get_active()"),
        ("src/commands/history_commands.rs", ".get_active()"),
        ("src/commands/history_commands.rs", ".recognize_image("),
        ("src/commands/history_commands.rs", ".read_ocr_source("),
    ];
    let mut violations = Vec::new();

    for (path, policy) in forbidden_command_policy {
        if command_sources
            .iter()
            .find(|source| source.path == path)
            .is_some_and(|source| source.source.contains(policy))
        {
            violations.push(format!("{path} contains {policy}"));
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let provider_composition =
        fs::read_to_string(manifest_dir.join("src/composition/provider_runtime.rs")).unwrap();
    for policy in [
        "load_baidu_ocr_credentials",
        "baidu_ocr_api_key",
        "baidu_ocr_secret_key",
    ] {
        if provider_composition.contains(policy) {
            violations.push(format!(
                "src/composition/provider_runtime.rs contains {policy}"
            ));
        }
    }

    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn composition_is_the_only_platform_adapter_selector() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let read = |path: &str| fs::read_to_string(manifest_dir.join(path)).unwrap();
    let mut violations = Vec::new();

    for source in production_command_sources() {
        if source.source.contains("TauriHotkeyRegistrar") {
            violations.push(format!("{} constructs TauriHotkeyRegistrar", source.path));
        }
    }

    let lib_source = read("src/lib.rs");
    if lib_source.contains("TauriHotkeyRegistrar") {
        violations.push("src/lib.rs constructs TauriHotkeyRegistrar".to_string());
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
    for platform in ["macos", "windows", "linux"] {
        let path = format!("src/infrastructure/system/selection/{platform}/mod.rs");
        if read(&path).contains("fn platform_selection_provider") {
            violations.push(format!("{path} selects a selection adapter"));
        }
    }

    let capture_composition = read("src/composition/capture_runtime.rs");
    for adapter in [
        "MacOSCaptureSessionSource",
        "WindowsCaptureSessionSource",
        "LinuxCaptureSessionSource",
    ] {
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
    assert!(
        read("src/composition.rs").contains("TauriHotkeyRegistrar"),
        "root Composition must select TauriHotkeyRegistrar"
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
