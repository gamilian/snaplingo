//! Check real Cargo edges and the production module graph, including target
//! dependencies and renamed imports. Test-only adapter fixtures are excluded.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Attribute, Item, Meta, Token, UseTree};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn production_dependencies(package: &Value) -> BTreeSet<String> {
    package["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|dependency| dependency["kind"] != "dev")
        .map(|dependency| dependency["name"].as_str().unwrap().to_owned())
        .collect()
}

fn dependency_violations(packages: &BTreeMap<String, Value>) -> Vec<String> {
    let mut errors = Vec::new();
    for (name, package) in packages {
        for dependency in production_dependencies(package) {
            if dependency == "tauri"
                || dependency.starts_with("tauri-")
                || matches!(dependency.as_str(), "wry" | "tao" | "objc2-web-kit")
            {
                errors.push(format!("{name} -> {dependency}"));
            }
        }
    }
    for (name, allowed) in [
        (
            "snaplingo-storage",
            &[
                "snaplingo-core",
                "async-trait",
                "chrono",
                "image",
                "md5",
                "rusqlite",
                "serde",
                "serde_json",
            ][..],
        ),
        ("snaplingo-ui", &["unicode-segmentation"][..]),
        (
            "snaplingo-accessibility",
            &["snaplingo-ui", "accesskit"][..],
        ),
        ("snaplingo-assets", &[][..]),
        (
            "snaplingo-settings",
            &[
                "snaplingo-core",
                "snaplingo-ui",
                "tokio",
                "base64",
                "chrono",
                "image",
                "log",
            ][..],
        ),
        (
            "snaplingo-render",
            &[
                "snaplingo-ui",
                "snaplingo-assets",
                "tiny-skia",
                "cosmic-text",
                "unicode-segmentation",
            ][..],
        ),
        (
            "snaplingo-desktop",
            &[
                "snaplingo-ui",
                "snaplingo-render",
                "snaplingo-accessibility",
                "snaplingo-settings",
                "winit",
                "softbuffer",
                "accesskit",
                "accesskit_winit",
                "rfd",
                "objc2",
                "objc2-foundation",
                "objc2-app-kit",
                "windows-sys",
                "arboard",
            ][..],
        ),
    ] {
        for dependency in production_dependencies(&packages[name]) {
            if !allowed.contains(&dependency.as_str()) {
                errors.push(format!("{name} -> {dependency}"));
            }
        }
    }
    for dependency in production_dependencies(&packages["snaplingo-core"]) {
        if (packages.contains_key(&dependency) && dependency != "snaplingo-assets")
            || concrete_platform_dependency(&dependency)
        {
            errors.push(format!("snaplingo-core -> {dependency}"));
        }
    }
    errors
}

fn concrete_platform_dependency(name: &str) -> bool {
    [
        "tauri",
        "objc",
        "windows",
        "core-graphics",
        "core-foundation",
        "accesskit",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
        || [
            "wry",
            "tao",
            "winit",
            "softbuffer",
            "accesskit",
            "accesskit_winit",
            "arboard",
            "enigo",
            "xcap",
            "rfd",
            "tray-icon",
            "global-hotkey",
            "dispatch2",
            "block2",
        ]
        .contains(&name)
}

#[test]
fn cargo_edges_keep_presentation_storage_and_platforms_separate() {
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--offline",
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
    let packages = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|package| {
            (
                package["name"].as_str().unwrap().to_owned(),
                package.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let violations = dependency_violations(&packages);
    assert!(
        violations.is_empty(),
        "Forbidden production dependencies: {violations:#?}"
    );

    for dependency in ["tauri", "tauri-build", "wry", "tao", "objc2-web-kit"] {
        let mut forbidden = packages.clone();
        forbidden.get_mut("snaplingo").unwrap()["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name": dependency, "rename": "shell", "kind": "build",
                "target": "cfg(target_os = \"windows\")"
            }));
        assert!(dependency_violations(&forbidden).contains(&format!("snaplingo -> {dependency}")));
    }

    // Cargo reports the real package name even when a target dependency is
    // renamed. Do not accidentally inspect only the host's active cfg edges.
    let mut forbidden = packages.clone();
    forbidden.get_mut("snaplingo-core").unwrap()["dependencies"].as_array_mut().unwrap()
        .push(serde_json::json!({"name":"tauri", "rename":"shell", "kind":null, "target":"cfg(target_os = \"windows\")"}));
    assert!(dependency_violations(&forbidden).contains(&"snaplingo-core -> tauri".into()));
    for (name, kind, target) in [
        (
            "objc2",
            serde_json::Value::Null,
            "cfg(target_os = \"macos\")",
        ),
        (
            "windows-sys",
            serde_json::json!("build"),
            "cfg(target_os = \"windows\")",
        ),
    ] {
        let dependency =
            serde_json::json!({"name":name, "rename":"native", "kind":kind, "target":target});
        let mut forbidden = packages.clone();
        forbidden.get_mut("snaplingo-core").unwrap()["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(dependency.clone());
        assert!(dependency_violations(&forbidden).contains(&format!("snaplingo-core -> {name}")));
        let mut allowed = packages.clone();
        allowed.get_mut("snaplingo-desktop").unwrap()["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(dependency);
        assert!(dependency_violations(&allowed).is_empty());
    }
    let fixture = serde_json::json!({"dependencies":[
        {"name":"snaplingo-core","kind":"dev","target":"cfg(target_os = \"windows\")"},
        {"name":"objc2","kind":null,"target":"cfg(target_os = \"macos\")"},
        {"name":"windows-sys","kind":"build","target":"cfg(target_os = \"windows\")"}
    ]});
    assert_eq!(
        production_dependencies(&fixture),
        BTreeSet::from(["objc2".into(), "windows-sys".into()])
    );
}

// Evaluate whether a cfg can exist with `test = false`; other features/targets
// remain possible so this checks production on every supported platform.
fn cfg_possibilities(meta: &Meta) -> (bool, bool) {
    match meta {
        Meta::Path(path) if path.is_ident("test") => (true, false),
        Meta::List(list) => {
            let nested = list
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .unwrap();
            let values = nested.iter().map(cfg_possibilities).collect::<Vec<_>>();
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

fn production(attrs: &[Attribute]) -> bool {
    attrs.iter().all(|attribute| {
        !attribute.path().is_ident("cfg")
            || cfg_possibilities(&attribute.parse_args::<Meta>().unwrap()).1
    })
}

fn use_paths(tree: &UseTree, prefix: String, output: &mut Vec<String>) {
    match tree {
        UseTree::Path(path) => use_paths(&path.tree, format!("{prefix}{}::", path.ident), output),
        UseTree::Name(name) => output.push(format!("{prefix}{}", name.ident)),
        UseTree::Rename(rename) => output.push(format!("{prefix}{}", rename.ident)),
        UseTree::Glob(_) => output.push(prefix),
        UseTree::Group(group) => {
            for item in &group.items {
                use_paths(item, prefix.clone(), output);
            }
        }
    }
}

struct CoreSources {
    source_root: PathBuf,
    file: PathBuf,
    modules: PathBuf,
    seen: BTreeSet<PathBuf>,
    violations: BTreeSet<String>,
}
impl CoreSources {
    fn load(&mut self, file: &Path, modules: PathBuf) {
        let file = file.canonicalize().unwrap();
        assert!(
            file.starts_with(&self.source_root),
            "Core production source reaches outside its crate: {}",
            file.display()
        );
        if !self.seen.insert(file.clone()) {
            return;
        }
        let syntax = syn::parse_file(&fs::read_to_string(&file).unwrap()).unwrap();
        let previous = (self.file.clone(), self.modules.clone());
        self.file = file;
        self.modules = modules;
        self.visit_file(&syntax);
        (self.file, self.modules) = previous;
    }
    fn record(&mut self, path: &str) {
        let path = path.strip_prefix("crate::").unwrap_or(path);
        let first = path.split("::").next().unwrap_or("").replace('_', "-");
        if concrete_platform_dependency(&first)
            || (first.starts_with("snaplingo-") && first != "snaplingo-assets")
            || [
                "infrastructure",
                "commands",
                "composition",
                "app-state",
                "app-shell",
                "app-actions",
                "settings-window",
                "startup-shortcuts",
            ]
            .contains(&first.as_str())
        {
            self.violations.insert(format!(
                "{} -> {path}",
                self.file.strip_prefix(&self.source_root).unwrap().display()
            ));
        }
    }
}
impl<'ast> Visit<'ast> for CoreSources {
    fn visit_item(&mut self, item: &'ast Item) {
        let attrs = match item {
            Item::Const(i) => &i.attrs,
            Item::Enum(i) => &i.attrs,
            Item::ExternCrate(i) => &i.attrs,
            Item::Fn(i) => &i.attrs,
            Item::ForeignMod(i) => &i.attrs,
            Item::Impl(i) => &i.attrs,
            Item::Macro(i) => &i.attrs,
            Item::Mod(i) => &i.attrs,
            Item::Static(i) => &i.attrs,
            Item::Struct(i) => &i.attrs,
            Item::Trait(i) => &i.attrs,
            Item::TraitAlias(i) => &i.attrs,
            Item::Type(i) => &i.attrs,
            Item::Union(i) => &i.attrs,
            Item::Use(i) => &i.attrs,
            _ => return,
        };
        if production(attrs) {
            visit::visit_item(self, item);
        }
    }
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        let nested = self.modules.join(module.ident.to_string());
        if let Some((_, items)) = &module.content {
            let previous = std::mem::replace(&mut self.modules, nested);
            for item in items {
                self.visit_item(item);
            }
            self.modules = previous;
        } else {
            let attribute_path = module.attrs.iter().find_map(|attr| {
                if let Meta::NameValue(value) = &attr.meta {
                    if value.path.is_ident("path") {
                        if let syn::Expr::Lit(lit) = &value.value {
                            if let syn::Lit::Str(path) = &lit.lit {
                                return Some(self.file.parent().unwrap().join(path.value()));
                            }
                        }
                    }
                }
                None
            });
            let file = attribute_path.unwrap_or_else(|| {
                let single = nested.with_extension("rs");
                if single.exists() {
                    single
                } else {
                    nested.join("mod.rs")
                }
            });
            self.load(&file, nested);
        }
    }
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        let mut paths = Vec::new();
        use_paths(&item.tree, String::new(), &mut paths);
        for path in paths {
            self.record(&path);
        }
    }
    fn visit_path(&mut self, path: &'ast syn::Path) {
        self.record(
            &path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
        );
        visit::visit_path(self, path);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        assert!(
            !mac.path.is_ident("include"),
            "Core production must declare Rust modules inside its crate: {}",
            self.file.display()
        );
        visit::visit_macro(self, mac);
    }
}

#[test]
fn core_production_module_graph_has_no_concrete_adapters() {
    let source_root = root().join("crates/core").canonicalize().unwrap();
    let mut sources = CoreSources {
        source_root: source_root.clone(),
        file: source_root.join("lib.rs"),
        modules: source_root.clone(),
        seen: BTreeSet::new(),
        violations: BTreeSet::new(),
    };
    sources.load(&source_root.join("lib.rs"), source_root);
    assert!(
        sources.seen.len() > 40,
        "The guard must traverse the real production module graph"
    );
    assert!(
        sources.violations.is_empty(),
        "Core imports concrete adapters: {:#?}",
        sources.violations
    );
}

#[test]
fn source_guard_distinguishes_target_adapters_from_test_fixtures() {
    let source_root = root();
    let mut sources = CoreSources {
        source_root: source_root.clone(),
        file: source_root.join("fixture.rs"),
        modules: source_root,
        seen: BTreeSet::new(),
        violations: BTreeSet::new(),
    };
    let syntax = syn::parse_file(
        r#"
        #[cfg(all(test, target_os="windows"))]
        mod test_only { use tauri::Manager; }
        #[cfg(not(test))]
        use crate::{infrastructure as adapter};
        #[cfg(target_os="windows")]
        use windows::Win32 as native;
    "#,
    )
    .unwrap();
    sources.visit_file(&syntax);
    assert_eq!(
        sources.violations,
        BTreeSet::from([
            "fixture.rs -> infrastructure".into(),
            "fixture.rs -> windows::Win32".into(),
        ])
    );
}
