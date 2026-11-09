fn main() {
    println!("cargo:rustc-check-cfg=cfg(mobile)");
    #[cfg(feature = "native-app")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let resource = output.join("native.rc");
        let quote = |path: std::path::PathBuf| path.display().to_string().replace('\\', "\\\\");
        let version = std::env::var("CARGO_PKG_VERSION").unwrap();
        let numeric = format!("{},0", version.replace('.', ","));
        std::fs::write(
            &resource,
            format!(
                r#"
#pragma code_page(65001)
#include <windows.h>
1 ICON "{}"
1 RT_MANIFEST "{}"
1 VERSIONINFO
 FILEVERSION {numeric}
 PRODUCTVERSION {numeric}
 FILEFLAGSMASK 0x3fL
 FILEFLAGS 0x0L
 FILEOS 0x40004L
 FILETYPE 0x1L
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904b0"
  BEGIN
   VALUE "FileDescription", "SnapLingo"
   VALUE "FileVersion", "{version}"
   VALUE "InternalName", "snaplingo-native"
   VALUE "ProductName", "SnapLingo"
   VALUE "ProductVersion", "{version}"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x409, 1200
 END
END
"#,
                quote(root.join("icons/icon.ico")),
                quote(root.join("native.manifest"))
            ),
        )
        .unwrap();
        embed_resource::compile_for(
            &resource,
            &["snaplingo-native", "snaplingo"],
            embed_resource::NONE,
        )
        .manifest_required()
        .expect("Native Windows application resources must compile");
        println!("cargo:rerun-if-changed=icons/icon.ico");
        println!("cargo:rerun-if-changed=native.manifest");
    }
    // Native integration harnesses run on the process main thread and reuse
    // composition with explicit paths instead of copying its private wiring.
    if matches!(
        std::env::var("CARGO_CFG_TARGET_OS").as_deref(),
        Ok("macos" | "windows")
    ) {
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
}
