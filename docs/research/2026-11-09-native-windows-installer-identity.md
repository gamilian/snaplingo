# Native Windows installer identity

The native installers retain the installed product identity of the repository's Tauri CLI 2.11.2 configuration. This is required for upgrades to replace the existing installation instead of registering a second application.

- MSI UpgradeCode is `a6e0f6d1-4f1f-5967-aeb3-62e77a923554`. Tauri derives its default with UUIDv5 using the DNS namespace and `SnapLingo.exe.app.x64`. Running the installed CLI's `inspect wix-upgrade-code` in `crates/snaplingo-app` printed that same value. It is now fixed in `desktop.json`.
- MSI scope remains `perMachine` and its default directory remains `ProgramFiles64Folder/SnapLingo`. A different installation context would prevent a normal major upgrade from finding the old product.
- The default publisher is `snaplingo`, from the second segment of `com.snaplingo.app`.
- NSIS retains its current-user scope, `$LOCALAPPDATA/SnapLingo` default directory, `HKCU/Software/Microsoft/Windows/CurrentVersion/Uninstall/SnapLingo` uninstall identity, and `HKCU/Software/snaplingo/SnapLingo` installation-location record. Both its default value and named `InstallDir` are written.
- MSI uses major upgrades and NSIS replaces its existing installation directory. Application-data directories are not installer resources and are not removed by the generated uninstallers.

Primary sources, pinned to the installed CLI version:

- [Tauri MSI identity generation](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.2/crates/tauri-bundler/src/bundle/windows/msi/mod.rs#L581).
- [Tauri MSI scope, upgrade scheduling, and installation directory](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.2/crates/tauri-bundler/src/bundle/windows/msi/main.wxs#L24).
- [Tauri NSIS registry identity](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.2/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi#L60).
- [Tauri NSIS current-user default and location restoration](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.2/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi#L495).

Package staging and template tests run on macOS. Installer execution, upgrade, uninstall, and application startup still require the Windows smoke workflow. Changing between MSI and NSIS is separate from upgrading within one installer family.

## Compiler verification on macOS

The Homebrew NSIS 3.13 compiler was run against the generated installer script, including a checkout path containing both `$` and `'`. It produced a PE32 NSIS self-extracting archive without executing it. The generated source and placeholder payload are retained under `target/native-installer-check-$'paths/`; the placeholder application is not runnable.

The compiler initially aborted with `std::bad_alloc`, including for an empty installer. Homebrew's own formula test documents [NSIS bug 1165](https://sourceforge.net/p/nsis/bugs/1165/) and sets an explicit UTF-8 locale. Running with `LANG=en_GB.UTF-8 LC_ALL=en_GB.UTF-8` fixed the compiler. NSIS compiler input paths use literal dollar signs; runtime installer strings escape them as `$$`.

WiX compiler execution and actual Windows installation/upgrade/uninstallation were not run on macOS.
