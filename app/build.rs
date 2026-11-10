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
}
