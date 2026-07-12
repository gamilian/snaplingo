use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};
use syn::{
    Attribute, ExprBlock, Field, ForeignItem, ImplItem, Item, ItemUse, TraitItem, UseTree, Variant,
};

const CONFIGURATION_DEPENDENCIES: &[&str] = &[];

const CREDENTIAL_DEPENDENCIES: &[&str] = &[];

const HTTP_LLM_DEPENDENCIES: &[&str] = &[
    "src/application/providers/configuration.rs -> crate::infrastructure::llm::AnthropicLLMClient",
    "src/application/providers/configuration.rs -> crate::infrastructure::llm::GeminiLLMClient",
    "src/application/providers/configuration.rs -> crate::infrastructure::llm::LLMClient",
    "src/application/providers/configuration.rs -> crate::infrastructure::llm::LLMProtocol",
    "src/application/providers/configuration.rs -> crate::infrastructure::llm::OpenAILLMClient",
    "src/application/providers/configuration.rs -> crate::infrastructure::llm::ReasoningLevel",
    "src/application/providers/llm_introspection.rs -> crate::infrastructure::llm::AnthropicLLMClient",
    "src/application/providers/llm_introspection.rs -> crate::infrastructure::llm::GeminiLLMClient",
    "src/application/providers/llm_introspection.rs -> crate::infrastructure::llm::LLMClient",
    "src/application/providers/llm_introspection.rs -> crate::infrastructure::llm::LLMOptions",
    "src/application/providers/llm_introspection.rs -> crate::infrastructure::llm::LLMProtocol",
    "src/application/providers/llm_introspection.rs -> crate::infrastructure::llm::LLMRequest",
    "src/application/providers/llm_introspection.rs -> crate::infrastructure::llm::LlmModelLister",
    "src/application/providers/llm_introspection.rs -> crate::infrastructure::llm::ModelInfo",
    "src/application/providers/llm_introspection.rs -> crate::infrastructure::llm::OpenAILLMClient",
    "src/application/providers/translation/impls/llm.rs -> crate::infrastructure::llm::LLMClient",
    "src/application/providers/translation/impls/llm.rs -> crate::infrastructure::llm::LLMOptions",
    "src/application/providers/translation/impls/llm.rs -> crate::infrastructure::llm::LLMRequest",
    "src/application/providers/translation/impls/llm.rs -> crate::infrastructure::llm::ReasoningLevel",
];

const EVENTS_HISTORY_DEPENDENCIES: &[&str] = &[
    "src/application/history/mod.rs -> crate::infrastructure::events::EventSubscriber",
    "src/application/history/mod.rs -> crate::infrastructure::storage::HistoryDatabase",
    "src/application/history/mod.rs -> crate::infrastructure::storage::HistoryEntry",
    "src/application/history/mod.rs -> crate::infrastructure::storage::OcrHistoryEntry",
    "src/application/history/mod.rs -> crate::infrastructure::storage::TranslationHistoryEntry",
    "src/application/providers/ocr/coordinator.rs -> crate::infrastructure::events::EventBus",
    "src/application/providers/translation/coordinator.rs -> crate::infrastructure::events::EventBus",
];

const RUNTIME_HOST_DEPENDENCIES: &[&str] = &[
    "src/application/capture/runtime.rs -> crate::infrastructure::system::capture_window::begin_capture_presentation",
    "src/application/capture/runtime.rs -> crate::infrastructure::system::capture_window::capture_window_bounds",
    "src/application/capture/runtime.rs -> crate::infrastructure::system::capture_window::destroy_inactive_capture_window",
    "src/application/capture/runtime.rs -> crate::infrastructure::system::capture_window::end_capture_presentation",
    "src/application/capture/runtime.rs -> crate::infrastructure::system::capture_window::hide_capture_window",
    "src/application/capture/runtime.rs -> crate::infrastructure::system::capture_window::open_capture_window_for_session",
    "src/application/capture/runtime.rs -> crate::infrastructure::system::capture_window::restore_capture_snapshot_windows",
    "src/application/hotkeys/runtime.rs -> crate::infrastructure",
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

fn dependency_inventory_violations(
    files: &[SourceFile],
    allowlist: &BTreeSet<String>,
) -> Vec<String> {
    let actual = dependency_inventory(files);

    actual.symmetric_difference(allowlist).cloned().collect()
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
    if path.get(0).map(String::as_str) == Some("crate")
        && path.get(1).map(String::as_str) == Some("infrastructure")
    {
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

fn legacy_allowlist() -> BTreeSet<String> {
    legacy_dependency_groups()
        .into_iter()
        .flat_map(|(_, dependencies)| dependencies.iter())
        .map(|entry| (*entry).to_string())
        .collect()
}

fn legacy_dependency_groups() -> [(&'static str, &'static [&'static str]); 5] {
    [
        ("configuration", CONFIGURATION_DEPENDENCIES),
        ("credentials", CREDENTIAL_DEPENDENCIES),
        ("http_llm", HTTP_LLM_DEPENDENCIES),
        ("events_history", EVENTS_HISTORY_DEPENDENCIES),
        ("runtime_host", RUNTIME_HOST_DEPENDENCIES),
    ]
}

fn assert_dependency_group_inventory(group_name: &str, expected_dependencies: &[&str]) {
    let actual = dependency_inventory(&production_application_sources());
    let stale_entries = expected_dependencies
        .iter()
        .filter(|dependency| !actual.contains(**dependency))
        .copied()
        .collect::<Vec<_>>();

    assert!(
        stale_entries.is_empty(),
        "{group_name} dependency inventory is stale; missing dependencies: {stale_entries:#?}"
    );
}

#[test]
fn application_infrastructure_dependencies_match_the_legacy_inventory() {
    assert_eq!(
        dependency_inventory_violations(&production_application_sources(), &legacy_allowlist()),
        Vec::<String>::new()
    );
}

#[test]
fn remaining_configuration_dependencies_are_inventoried() {
    assert_dependency_group_inventory("configuration", CONFIGURATION_DEPENDENCIES);
}

#[test]
fn remaining_credential_dependencies_are_inventoried() {
    assert_dependency_group_inventory("credentials", CREDENTIAL_DEPENDENCIES);
}

#[test]
fn remaining_http_llm_dependencies_are_inventoried() {
    assert_dependency_group_inventory("http_llm", HTTP_LLM_DEPENDENCIES);
}

#[test]
fn remaining_events_history_dependencies_are_inventoried() {
    assert_dependency_group_inventory("events_history", EVENTS_HISTORY_DEPENDENCIES);
}

#[test]
fn remaining_runtime_host_dependencies_are_inventoried() {
    assert_dependency_group_inventory("runtime_host", RUNTIME_HOST_DEPENDENCIES);
}

#[test]
fn dependency_groups_partition_the_legacy_inventory() {
    let grouped_inventory = legacy_dependency_groups()
        .into_iter()
        .flat_map(|(_, dependencies)| dependencies.iter())
        .collect::<Vec<_>>();
    let grouped_set = grouped_inventory
        .iter()
        .map(|dependency| (**dependency).to_string())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        grouped_inventory.len(),
        grouped_set.len(),
        "legacy dependency groups must not contain duplicate entries"
    );
    assert_eq!(grouped_set, legacy_allowlist());
}

#[test]
fn rejects_an_unlisted_synthetic_dependency() {
    let files = [SourceFile {
        path: "src/application/example.rs".to_string(),
        source: r#"
            use crate::infrastructure::storage::{ConfigFile, Keychain};
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
        dependency_inventory_violations(&files, &BTreeSet::new()),
        vec![
            "src/application/example.rs -> crate::infrastructure::Enum::Variant",
            "src/application/example.rs -> crate::infrastructure::Foo",
            "src/application/example.rs -> crate::infrastructure::SomeImplTrait",
            "src/application/example.rs -> crate::infrastructure::SomeTraitBound",
            "src/application/example.rs -> crate::infrastructure::http::send",
            "src/application/example.rs -> crate::infrastructure::storage::ConfigFile",
            "src/application/example.rs -> crate::infrastructure::storage::Keychain",
        ]
    );
}
