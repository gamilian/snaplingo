fn main() {
    // The AppKit integration harness runs on the process main thread and reuses
    // composition with explicit paths instead of copying its private wiring.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let source = std::fs::read_to_string(root.join("src/composition.rs")).unwrap();
        let source = source
            .lines()
            .map(|line| {
                if let Some(module) = line
                    .strip_prefix("mod ")
                    .and_then(|line| line.strip_suffix(';'))
                {
                    let path = root.join("src/composition").join(format!("{module}.rs"));
                    format!("#[path = {path:?}]\n{line}")
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        std::fs::write(output.join("native_ui_composition.rs"), source).unwrap();
        println!("cargo:rerun-if-changed=src/composition.rs");
    }
    tauri_build::build()
}
