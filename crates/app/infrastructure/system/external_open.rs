use std::path::Path;

pub(crate) fn open_url(value: &str) -> crate::Result<()> {
    let url = external_url(value)?;
    open::that_detached(url.as_str()).map_err(Into::into)
}

pub(crate) fn open_file(path: &Path) -> crate::Result<()> {
    if !path.is_file() {
        return Err("要打开的文件不存在。".into());
    }
    open::that_detached(std::path::absolute(path)?).map_err(Into::into)
}

fn external_url(value: &str) -> crate::Result<reqwest::Url> {
    if value.chars().any(char::is_control) {
        return Err("外部链接包含无效字符。".into());
    }
    let url = reqwest::Url::parse(value).map_err(|_| "外部链接无效。")?;
    let allowed = match url.scheme() {
        "http" | "https" => url.host_str().is_some(),
        "mailto" | "tel" => !url.path().is_empty(),
        _ => false,
    };
    if !allowed {
        return Err("仅支持 HTTP、HTTPS、邮件和电话链接。".into());
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_existing_browser_mail_and_phone_schemes() {
        for value in [
            "https://github.com/gamilian/snaplingo",
            "http://localhost:8080/source?q=截图&name=a%20b#license",
            "mailto:user@example.com?subject=License",
            "tel:+8613800138000",
        ] {
            assert!(external_url(value).is_ok(), "{value}");
        }
    }

    #[test]
    fn rejects_files_executable_protocols_and_control_characters_from_url_ipc() {
        for value in [
            "file:///tmp/installer.exe",
            "/tmp/installer.exe",
            "javascript:alert(1)",
            "data:text/html,test",
            "custom-app://run",
            "--args",
            "https://",
            "mailto:",
            "tel:",
            "https://example.com/\0hidden",
            "https://example.com/\nnext",
        ] {
            assert!(external_url(value).is_err(), "{value:?}");
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn passes_urls_and_unicode_paths_as_one_literal_argument() {
        for target in [
            "https://example.com/?q=$(touch%20/tmp/no)&value=\"quoted\"",
            "/tmp/截图 ; $(touch nope)/installer.dmg",
        ] {
            let commands = open::commands(target);
            assert!(!commands.is_empty());
            for command in commands {
                assert_ne!(command.get_program(), "sh");
                assert_ne!(command.get_program(), "bash");
                let argument = command.get_args().last().unwrap();
                if command.get_program() == "wslview" && target.starts_with('/') {
                    assert!(argument
                        .to_string_lossy()
                        .ends_with(target.trim_start_matches('/')));
                } else {
                    assert_eq!(argument, target);
                }
            }
        }
    }

    #[test]
    fn does_not_launch_missing_installer_files() {
        let root = tempfile::tempdir().unwrap();
        assert!(open_file(&root.path().join("missing.dmg")).is_err());
        assert!(open_file(root.path()).is_err());
    }
}
