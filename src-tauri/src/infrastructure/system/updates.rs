use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use tokio::io::AsyncWriteExt;

use crate::application::providers::HttpClient;
use crate::application::updates::{
    AppUpdateHost, PublishedRelease, UpdateInstaller, LATEST_RELEASE_API, RELEASES_URL,
};
use crate::infrastructure::http::ReqwestHttpClient;

pub struct SystemAppUpdateHost {
    app: tauri::AppHandle,
    http: Arc<ReqwestHttpClient>,
}

impl SystemAppUpdateHost {
    pub fn new(app: tauri::AppHandle, http: Arc<ReqwestHttpClient>) -> Self {
        Self { app, http }
    }
}

#[async_trait]
impl AppUpdateHost for SystemAppUpdateHost {
    async fn latest_release(&self) -> crate::Result<Option<PublishedRelease>> {
        let response = self
            .http
            .get(
                LATEST_RELEASE_API,
                [
                    (
                        "User-Agent".into(),
                        format!("SnapLingo/{}", self.app.package_info().version),
                    ),
                    ("Accept".into(), "application/vnd.github+json".into()),
                    ("X-GitHub-Api-Version".into(), "2022-11-28".into()),
                ]
                .into(),
            )
            .await?;
        match response.status {
            404 => Ok(None),
            403 | 429 => Err("更新检查暂时受 GitHub 限流，请稍后重试或打开发布页。".into()),
            200 => Ok(Some(
                serde_json::from_str(&response.body)
                    .map_err(|_| "无法读取发布信息，请稍后重试。")?,
            )),
            code => Err(format!("检查更新失败（HTTP {code}），请稍后重试。").into()),
        }
    }

    async fn download_and_open(&self, installer: &UpdateInstaller) -> crate::Result<String> {
        let root = self
            .app
            .path()
            .app_cache_dir()
            .map_err(|error| error.to_string())?
            .join("updates");
        tokio::fs::create_dir_all(&root).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).await?;
        }
        let (client, _) = self.http.client_for_request()?;
        let directory = download_installer(&client, installer, &root).await?;
        let path = directory.path().join(&installer.name);
        mark_internet_download(&path, &installer.url)?;
        // Keep the verified file available while the system installer is using it.
        let _ = directory.keep();
        #[allow(deprecated)]
        self.app
            .shell()
            .open(path.to_string_lossy().into_owned(), None)
            .map_err(|error| {
                format!("安装包已下载到 {}，但无法自动打开：{error}", path.display())
            })?;
        Ok(path.to_string_lossy().into_owned())
    }

    fn open_release_page(&self) -> crate::Result<()> {
        #[allow(deprecated)]
        self.app
            .shell()
            .open(RELEASES_URL, None)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

async fn download_installer(
    client: &reqwest::Client,
    installer: &UpdateInstaller,
    root: &Path,
) -> crate::Result<tempfile::TempDir> {
    let directory = tempfile::Builder::new()
        .prefix("snaplingo-")
        .tempdir_in(root)?;
    let mut response = client
        .get(&installer.url)
        .timeout(Duration::from_secs(600))
        .send()
        .await?
        .error_for_status()?;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.path().join(&installer.name))
        .await?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0_u64;
    while let Some(chunk) = response.chunk().await? {
        downloaded += chunk.len() as u64;
        if downloaded > installer.size {
            return Err("安装包大小与发布信息不符，已停止下载。请重新检查更新。".into());
        }
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    if downloaded != installer.size || format!("{:x}", hasher.finalize()) != installer.sha256 {
        return Err("安装包校验失败，未打开文件。请重新检查更新后再下载。".into());
    }
    file.sync_all().await?;
    drop(file);
    Ok(directory)
}

fn mark_internet_download(path: &Path, source: &str) -> crate::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs();
        let status = std::process::Command::new("/usr/bin/xattr")
            .args([
                "-w",
                "com.apple.quarantine",
                &format!("0083;{timestamp:x};SnapLingo;"),
            ])
            .arg(path)
            .status()?;
        if !status.success() {
            return Err("无法标记安装包的下载来源，请从发布页使用浏览器下载。".into());
        }
        let _ = source;
    }
    #[cfg(target_os = "windows")]
    std::fs::write(
        format!("{}:Zone.Identifier", path.display()),
        format!("[ZoneTransfer]\r\nZoneId=3\r\nHostUrl={source}\r\n"),
    )?;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = (path, source);
    Ok(())
}

pub fn update_architecture() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    if std::process::Command::new("/usr/sbin/sysctl")
        .args(["-in", "sysctl.proc_translated"])
        .output()
        .is_ok_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "1"
        })
    {
        return "aarch64";
    }
    std::env::consts::ARCH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn downloads_verified_bytes_and_removes_failed_or_incomplete_downloads() {
        let mut server = mockito::Server::new_async().await;
        let body = b"verified installer fixture";
        let _request = server
            .mock("GET", "/installer")
            .with_body(body)
            .create_async()
            .await;
        let root = tempfile::tempdir().unwrap();
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let installer = UpdateInstaller {
            name: "test.dmg".into(),
            url: format!("{}/installer", server.url()),
            size: body.len() as u64,
            sha256: format!("{:x}", Sha256::digest(body)),
        };
        let downloaded = download_installer(&client, &installer, root.path())
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(downloaded.path().join("test.dmg")).unwrap(),
            body
        );
        drop(downloaded);
        for invalid in [
            UpdateInstaller {
                sha256: "0".repeat(64),
                ..installer.clone()
            },
            UpdateInstaller {
                size: installer.size + 1,
                ..installer.clone()
            },
            UpdateInstaller {
                size: installer.size - 1,
                ..installer.clone()
            },
        ] {
            assert!(download_installer(&client, &invalid, root.path())
                .await
                .is_err());
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
        }
    }
}
