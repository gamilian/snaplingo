pub mod common;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub use linux::platform_selection_provider;
#[cfg(target_os = "macos")]
pub use macos::platform_selection_provider;
#[cfg(target_os = "windows")]
pub use windows::platform_selection_provider;

#[cfg(test)]
mod cross_platform_adapter_tests {
    // Compile platform adapters into the host test binary so their contract
    // tests run even when the host OS excludes them from production builds.
    mod linux_provider {
        #[allow(dead_code)]
        pub mod module {
            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/infrastructure/system/selection/linux/mod.rs"
            ));
        }
    }

    mod windows_provider {
        #[allow(dead_code)]
        pub mod module {
            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/infrastructure/system/selection/windows/mod.rs"
            ));
        }
    }
}
