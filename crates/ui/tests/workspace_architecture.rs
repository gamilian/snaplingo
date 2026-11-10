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

fn production_crate(package: &Value) -> bool {
    let workspace = root().canonicalize().unwrap();
    let manifest = Path::new(package["manifest_path"].as_str().unwrap());
    manifest.starts_with(workspace.join("crates")) || manifest.starts_with(workspace.join("app"))
}

// Each independently built capability declares its allowed workspace edges.
// Adding a package requires choosing its ownership, not preserving a package count.
fn allowed_workspace_dependencies(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "snaplingo" => Some(&[
            "snaplingo-core",
            "snaplingo-ui",
            "snaplingo-platform",
            "snaplingo-providers",
            "snaplingo-storage",
            "snaplingo-assets",
        ]),
        "snaplingo-core" | "snaplingo-ui" => Some(&["snaplingo-assets"]),
        "snaplingo-platform" | "snaplingo-providers" | "snaplingo-storage" => {
            Some(&["snaplingo-core"])
        }
        "snaplingo-assets" => Some(&[]),
        _ => None,
    }
}

fn dependency_violations(packages: &BTreeMap<String, Value>) -> Vec<String> {
    let mut errors = Vec::new();
    for (name, package) in packages
        .iter()
        .filter(|(_, package)| production_crate(package))
    {
        let Some(allowed) = allowed_workspace_dependencies(name) else {
            errors.push(format!(
                "{name} has no declared workspace dependency policy"
            ));
            continue;
        };
        for dependency in production_dependencies(package) {
            if packages.get(&dependency).is_some_and(production_crate)
                && !allowed.contains(&dependency.as_str())
            {
                errors.push(format!("{name} -> {dependency}"));
            }
            if packages
                .get(&dependency)
                .is_some_and(|package| !production_crate(package))
            {
                errors.push(format!("{name} -> development tool {dependency}"));
            }
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
        (
            "snaplingo-ui",
            &[
                "unicode-segmentation",
                "snaplingo-assets",
                "tiny-skia",
                "cosmic-text",
                "accesskit",
            ][..],
        ),
        ("snaplingo-assets", &[][..]),
    ] {
        for dependency in production_dependencies(&packages[name]) {
            if !allowed.contains(&dependency.as_str()) {
                errors.push(format!("{name} -> {dependency}"));
            }
        }
    }
    for dependency in production_dependencies(&packages["snaplingo-core"]) {
        if concrete_platform_dependency(&dependency)
            || matches!(
                dependency.as_str(),
                "reqwest" | "rusqlite" | "ort" | "ocr-rs" | "oar-ocr-core" | "ndarray"
            )
        {
            errors.push(format!("snaplingo-core -> {dependency}"));
        }
    }
    for dependency in production_dependencies(&packages["snaplingo-providers"]) {
        // Local inference uses NSProcessInfo to select the supported macOS engine.
        if (concrete_platform_dependency(&dependency) && dependency != "objc2-foundation")
            || dependency == "rusqlite"
        {
            errors.push(format!("snaplingo-providers -> {dependency}"));
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

    let ui = &packages["snaplingo-ui"];
    assert_eq!(
        ui["features"]["default"],
        serde_json::json!([]),
        "Layout and interaction must build without rendering or semantic adapters"
    );
    for dependency in ui["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|dependency| {
            matches!(
                dependency["name"].as_str(),
                Some("snaplingo-assets" | "tiny-skia" | "cosmic-text" | "accesskit")
            )
        })
    {
        assert_eq!(
            dependency["optional"], true,
            "Rendering/accessibility dependencies must remain opt-in"
        );
    }
    for (owner, dependency) in [
        ("snaplingo-ui", "snaplingo-platform"),
        ("snaplingo-platform", "snaplingo"),
        ("snaplingo-platform", "snaplingo-ui"),
        ("snaplingo-core", "snaplingo-providers"),
        ("snaplingo-providers", "snaplingo"),
        ("snaplingo-providers", "snaplingo-ui"),
        ("snaplingo-providers", "snaplingo-platform"),
        ("snaplingo-providers", "snaplingo-storage"),
    ] {
        let mut forbidden = packages.clone();
        forbidden.get_mut(owner).unwrap()["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"name": dependency, "kind": null}));
        assert!(dependency_violations(&forbidden).contains(&format!("{owner} -> {dependency}")));
    }

    let reference = &packages["snaplingo-reference"];
    assert!(!production_crate(reference));
    assert!(production_dependencies(reference).contains("objc2-web-kit"));
    assert!(
        production_dependencies(reference).iter().all(|dependency| {
            packages
                .get(dependency)
                .is_none_or(|package| !production_crate(package))
        }),
        "Reference collectors must build independently of production crates"
    );
    let mut forbidden = packages.clone();
    forbidden.get_mut("snaplingo").unwrap()["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"name":"snaplingo-reference","kind":null}));
    assert!(dependency_violations(&forbidden)
        .contains(&"snaplingo -> development tool snaplingo-reference".into()));

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
        allowed.get_mut("snaplingo-platform").unwrap()["dependencies"]
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

#[test]
fn provider_protocol_build_excludes_native_inference_dependencies() {
    let output = Command::new(env!("CARGO"))
        .args([
            "tree",
            "--locked",
            "--offline",
            "-p",
            "snaplingo-providers",
            "--no-default-features",
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8(output.stdout).unwrap();
    for line in tree.lines() {
        let package = line.split_whitespace().next().unwrap_or("");
        assert!(
            !matches!(
                package,
                "ort" | "ort-sys" | "ocr-rs" | "oar-ocr-core" | "oar-ocr-derive" | "ndarray"
            ),
            "Protocol-only providers must not build local inference: {line}"
        );
    }
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
        if matches!(path, "window" | "actions") {
            return;
        }
        let path = path.strip_prefix("crate::").unwrap_or(path);
        let first = path.split("::").next().unwrap_or("").replace('_', "-");
        if concrete_platform_dependency(&first)
            || (first.starts_with("snaplingo-") && first != "snaplingo-assets")
            || [
                "infrastructure",
                "commands",
                "composition",
                "actions",
                "lifecycle",
                "window",
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
    let source_root = root().join("crates/core/src").canonicalize().unwrap();
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
        use crate::window::capture::NativeCaptureSessionRuntimeHost;
        fn local(window: u8) -> u8 { window }
    "#,
    )
    .unwrap();
    sources.visit_file(&syntax);
    assert_eq!(
        sources.violations,
        BTreeSet::from([
            "fixture.rs -> infrastructure".into(),
            "fixture.rs -> windows::Win32".into(),
            "fixture.rs -> window::capture::NativeCaptureSessionRuntimeHost".into(),
        ])
    );
}

// Lower UI modules use explicit owning-module imports so a crate-root re-export
// cannot quietly make drawing depend on a product page again.
struct UiFoundationImports {
    module: Vec<String>,
    owner: String,
    violations: BTreeSet<String>,
}

impl UiFoundationImports {
    fn record(&mut self, path: &str) {
        let parts = path.split("::").collect::<Vec<_>>();
        let mut resolved = self.module.clone();
        let mut index;
        match parts.first().copied() {
            Some("crate" | "snaplingo_ui") => {
                resolved.clear();
                index = 1;
            }
            Some("super") => {
                index = 0;
                while parts.get(index) == Some(&"super") {
                    resolved.pop();
                    index += 1;
                }
            }
            Some("self") => {
                index = 1;
            }
            _ => return,
        }
        resolved.extend(parts[index..].iter().map(|part| (*part).to_owned()));
        let dependency = resolved.first().map(String::as_str).unwrap_or("");
        if dependency != "foundation" && !(self.owner == "render" && dependency == "render") {
            self.violations.insert(path.into());
        }
    }
}

impl<'ast> Visit<'ast> for UiFoundationImports {
    // pub(crate) is visibility, not a dependency on the crate root's exports.
    fn visit_visibility(&mut self, _visibility: &'ast syn::Visibility) {}

    fn visit_item(&mut self, item: &'ast Item) {
        // Tests may construct whole product frames to exercise the renderer.
        if let Item::Mod(module) = item {
            if !production(&module.attrs) {
                return;
            }
            if module
                .attrs
                .iter()
                .any(|attribute| attribute.path().is_ident("path"))
            {
                self.violations.insert("source mount #[path]".into());
            }
            if let Some((_, items)) = &module.content {
                self.module.push(module.ident.to_string());
                for item in items {
                    self.visit_item(item);
                }
                self.module.pop();
                return;
            }
        }
        visit::visit_item(self, item);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if !production(&item.attrs) {
            return;
        }
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
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
        );
        visit::visit_path(self, path);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac.path.is_ident("include") {
            self.violations.insert("source mount include!".into());
        }
        visit::visit_macro(self, mac);
    }
}

fn ui_foundation_imports(source: &str, module: Vec<String>) -> BTreeSet<String> {
    let mut visitor = UiFoundationImports {
        owner: module[0].clone(),
        module,
        violations: BTreeSet::new(),
    };
    visitor.visit_file(&syn::parse_file(source).unwrap());
    visitor.violations
}

#[test]
fn ui_foundation_and_renderer_do_not_import_product_pages() {
    let source_root = root().join("crates/ui/src");
    let mut pending = vec![source_root.join("foundation"), source_root.join("render")];
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            pending.extend(
                fs::read_dir(path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path()),
            );
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let relative = path.strip_prefix(&source_root).unwrap();
            let mut module = relative
                .with_extension("")
                .components()
                .map(|component| component.as_os_str().to_str().unwrap().to_owned())
                .collect::<Vec<_>>();
            if module.last().is_some_and(|name| name == "mod") {
                module.pop();
            }
            let violations = ui_foundation_imports(&fs::read_to_string(&path).unwrap(), module);
            assert!(
                violations.is_empty(),
                "{} imports product UI: {violations:#?}",
                relative.display()
            );
        }
    }
    let invalid = r#"
        use crate::settings::Snapshot;
        use crate::{Scene as HiddenSettingsScene};
        use super::settings::Target;
        #[cfg(target_os = "windows")]
        fn native() { crate::result::run(); }
        #[cfg(test)]
        mod tests { use crate::settings::Snapshot; }
        use crate::foundation::Scene;
        pub(crate) fn permitted_visibility() {}
        #[path = "../settings/mod.rs"] mod page;
        include!("../settings/state.rs");
    "#;
    assert_eq!(
        ui_foundation_imports(invalid, vec!["render".into()]),
        BTreeSet::from([
            "crate::settings::Snapshot".into(),
            "crate::Scene".into(),
            "super::settings::Target".into(),
            "crate::result::run".into(),
            "source mount #[path]".into(),
            "source mount include!".into(),
        ])
    );
}
