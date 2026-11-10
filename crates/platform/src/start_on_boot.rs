use crate::Result;
use snaplingo_core::settings::StartOnBoot;
use std::path::{Path, PathBuf};

pub struct NativeStartOnBoot {
    launch: std::result::Result<auto_launch::AutoLaunch, String>,
}

impl NativeStartOnBoot {
    pub fn new() -> Self {
        let launch = std::env::current_exe()
            .map_err(|error| error.to_string())
            .and_then(|executable| {
                launch_target(
                    &executable,
                    std::env::var_os("APPIMAGE").map(PathBuf::from).as_deref(),
                )
            })
            .and_then(|path| {
                auto_launch::AutoLaunchBuilder::new()
                    .set_app_name("SnapLingo")
                    .set_app_path(&path)
                    .set_use_launch_agent(true)
                    .set_args(&[] as &[&str])
                    .build()
                    .map_err(|error| error.to_string())
            });
        Self { launch }
    }
}

fn launch_target(
    executable: &Path,
    appimage: Option<&Path>,
) -> std::result::Result<String, String> {
    let path = if cfg!(target_os = "linux") {
        appimage.unwrap_or(executable).to_path_buf()
    } else if cfg!(target_os = "macos") {
        executable
            .canonicalize()
            .map_err(|error| error.to_string())?
    } else {
        executable.to_path_buf()
    };
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "Application launch path is not valid Unicode".into())
}

impl StartOnBoot for NativeStartOnBoot {
    fn set_enabled(&self, enabled: bool) -> Result<()> {
        let launch = self.launch.as_ref().map_err(|error| error.clone())?;
        let result = if enabled {
            launch.enable()
        } else {
            launch.disable()
        };
        result.map_err(|error| error.to_string().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_login_entry_preserves_the_previous_name_launch_agent_and_empty_arguments() {
        let executable = std::env::current_exe().unwrap();
        let path = launch_target(&executable, None).unwrap();
        let expected = auto_launch::AutoLaunchBuilder::new()
            .set_app_name("SnapLingo")
            .set_app_path(&path)
            .set_use_launch_agent(true)
            .set_args(&[] as &[&str])
            .build()
            .unwrap();
        assert_eq!(NativeStartOnBoot::new().launch.unwrap(), expected);
    }

    #[test]
    fn launch_target_keeps_spaces_and_unicode_as_one_path() {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("SnapLingo 截图.app");
        std::fs::write(&executable, []).unwrap();
        let appimage = directory.path().join("SnapLingo 截图.AppImage");
        let actual = launch_target(&executable, Some(&appimage)).unwrap();
        let expected = if cfg!(target_os = "linux") {
            appimage
        } else if cfg!(target_os = "macos") {
            executable.canonicalize().unwrap()
        } else {
            executable
        };
        assert_eq!(actual, expected.to_str().unwrap());
    }
}
