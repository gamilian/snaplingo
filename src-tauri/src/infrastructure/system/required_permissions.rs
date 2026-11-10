use crate::application::{
    RequiredPermissionsContext, RequiredPermissionsHost, RequiredPermissionsStatus,
    SystemPermission,
};
use std::path::Path;

pub struct SystemRequiredPermissions(pub tauri::AppHandle);

impl RequiredPermissionsHost for SystemRequiredPermissions {
    fn status(&self) -> RequiredPermissionsStatus {
        #[cfg(target_os = "macos")]
        return RequiredPermissionsStatus {
            screen_recording: unsafe { CGPreflightScreenCaptureAccess() },
            accessibility: crate::infrastructure::system::selection::macos::context::accessibility_permission_granted(false),
        };
        #[cfg(not(target_os = "macos"))]
        RequiredPermissionsStatus {
            screen_recording: true,
            accessibility: true,
        }
    }

    fn context(&self) -> RequiredPermissionsContext {
        permission_context(
            std::env::current_exe().ok().as_deref(),
            std::env::consts::OS,
        )
    }

    fn request(&self, permission: SystemPermission) -> crate::Result<()> {
        if self.context().needs_installation {
            return Err(
                "请先将 SnapLingo 拖入应用程序，关闭当前副本，再打开已安装的应用授权。".into(),
            );
        }
        #[cfg(target_os = "macos")]
        {
            match permission {
                SystemPermission::ScreenRecording => unsafe {
                    CGRequestScreenCaptureAccess();
                },
                SystemPermission::Accessibility => {
                    crate::infrastructure::system::selection::macos::context::accessibility_permission_granted(true);
                }
            }
            let pane = match permission {
                SystemPermission::ScreenRecording => "Privacy_ScreenCapture",
                SystemPermission::Accessibility => "Privacy_Accessibility",
            };
            let status = std::process::Command::new("/usr/bin/open")
                .arg(format!(
                    "x-apple.systempreferences:com.apple.preference.security?{pane}"
                ))
                .status()?;
            if !status.success() {
                return Err("无法打开系统权限设置，请在系统设置的“隐私与安全性”中打开。".into());
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = permission;
        Ok(())
    }

    fn reset(&self, permission: SystemPermission) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Only the selected service for this app; never reset all TCC records.
            let output = std::process::Command::new("/usr/bin/tccutil")
                .args(reset_arguments(permission, &self.0.config().identifier))
                .output()?;
            if !output.status.success() {
                log::warn!(
                    "Permission reset failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                return Err(
                    "未能重置授权。请在对应系统权限列表中移除旧的 SnapLingo，再添加当前应用。"
                        .into(),
                );
            }
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = permission;
            Err("当前系统不需要此权限修复。".into())
        }
    }

    fn restart(&self) -> crate::Result<()> {
        if self.context().needs_installation {
            return Err("请先安装并打开应用程序中的 SnapLingo。".into());
        }
        self.0.request_restart();
        Ok(())
    }
}

fn permission_context(executable: Option<&Path>, platform: &str) -> RequiredPermissionsContext {
    let bundle = executable.and_then(|path| {
        path.ancestors()
            .find(|ancestor| ancestor.extension().is_some_and(|ext| ext == "app"))
    });
    let app_path = bundle.or(executable).map(|path| path.display().to_string());
    let needs_installation = platform == "macos"
        && app_path.as_deref().is_some_and(|path| {
            path.starts_with("/Volumes/") || path.contains("/AppTranslocation/")
        });
    RequiredPermissionsContext {
        platform: platform.into(),
        app_path,
        needs_installation,
        can_reset: platform == "macos" && bundle.is_some() && !needs_installation,
    }
}

#[cfg(any(target_os = "macos", test))]
fn reset_arguments(permission: SystemPermission, identifier: &str) -> [&str; 3] {
    let service = match permission {
        SystemPermission::ScreenRecording => "ScreenCapture",
        SystemPermission::Accessibility => "Accessibility",
    };
    ["reset", service, identifier]
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_is_scoped_to_one_service_and_the_current_app() {
        assert_eq!(
            reset_arguments(SystemPermission::ScreenRecording, "com.snaplingo.app"),
            ["reset", "ScreenCapture", "com.snaplingo.app"]
        );
        assert_eq!(
            reset_arguments(SystemPermission::Accessibility, "com.snaplingo.app"),
            ["reset", "Accessibility", "com.snaplingo.app"]
        );
    }

    #[test]
    fn installed_app_can_repair_but_dmg_and_translocated_copies_must_be_installed_first() {
        let installed = permission_context(
            Some(Path::new(
                "/Applications/SnapLingo.app/Contents/MacOS/snaplingo",
            )),
            "macos",
        );
        assert_eq!(
            installed.app_path.as_deref(),
            Some("/Applications/SnapLingo.app")
        );
        assert!(installed.can_reset);
        assert!(!installed.needs_installation);
        for executable in [
            "/Volumes/SnapLingo/SnapLingo.app/Contents/MacOS/snaplingo",
            "/private/var/folders/example/AppTranslocation/id/d/SnapLingo.app/Contents/MacOS/snaplingo",
        ] {
            let context = permission_context(Some(Path::new(executable)), "macos");
            assert!(context.needs_installation);
            assert!(!context.can_reset);
        }
        let development =
            permission_context(Some(Path::new("/code/target/debug/snaplingo")), "macos");
        assert!(!development.can_reset);
        assert!(!permission_context(None, "windows").can_reset);
    }
}
