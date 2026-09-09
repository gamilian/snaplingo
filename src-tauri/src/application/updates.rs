use std::sync::Arc;

use async_trait::async_trait;
use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

pub const RELEASES_URL: &str = "https://github.com/gamilian/snaplingo/releases/latest";
pub const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/gamilian/snaplingo/releases/latest";
pub const MAX_INSTALLER_SIZE: u64 = 512 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct PublishedRelease {
    pub tag_name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub body: Option<String>,
    pub published_at: Option<String>,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub digest: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInstaller {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateStatus {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub available: bool,
    pub download_size: Option<u64>,
    pub notes: String,
    pub published_at: Option<String>,
    pub platform: String,
}

#[async_trait]
pub trait AppUpdateHost: Send + Sync {
    async fn latest_release(&self) -> crate::Result<Option<PublishedRelease>>;
    async fn download_and_open(&self, installer: &UpdateInstaller) -> crate::Result<String>;
    fn open_release_page(&self) -> crate::Result<()>;
}

pub struct AppUpdates {
    host: Arc<dyn AppUpdateHost>,
    version: String,
    platform: String,
    architecture: String,
    installer: Mutex<Option<UpdateInstaller>>,
}

impl AppUpdates {
    pub fn new(
        host: Arc<dyn AppUpdateHost>,
        version: String,
        platform: String,
        architecture: String,
    ) -> Self {
        Self {
            host,
            version,
            platform,
            architecture,
            installer: Mutex::new(None),
        }
    }

    pub async fn check(&self) -> crate::Result<AppUpdateStatus> {
        let mut installer = self
            .installer
            .try_lock()
            .map_err(|_| "正在处理更新，请稍后重试。")?;
        let release = self.host.latest_release().await?;
        let (status, selected) =
            select_update(&self.version, &self.platform, &self.architecture, release)?;
        *installer = selected;
        Ok(status)
    }

    pub async fn download_and_open(&self) -> crate::Result<String> {
        let installer = self
            .installer
            .try_lock()
            .map_err(|_| "正在处理更新，请稍后重试。")?;
        let installer = installer
            .as_ref()
            .ok_or("请先检查更新，确认有适配当前系统的新版安装包。")?;
        self.host.download_and_open(installer).await
    }

    pub fn open_release_page(&self) -> crate::Result<()> {
        self.host.open_release_page()
    }
}

fn select_update(
    current: &str,
    platform: &str,
    architecture: &str,
    release: Option<PublishedRelease>,
) -> crate::Result<(AppUpdateStatus, Option<UpdateInstaller>)> {
    let mut status = AppUpdateStatus {
        current_version: current.into(),
        latest_version: None,
        available: false,
        download_size: None,
        notes: String::new(),
        published_at: None,
        platform: platform.into(),
    };
    let Some(release) = release.filter(|release| !release.draft && !release.prerelease) else {
        return Ok((status, None));
    };
    let version_text = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let latest = Version::parse(version_text).map_err(|_| "发布版本号无效，请前往发布页查看。")?;
    let current = Version::parse(current).map_err(|_| "无法识别当前应用版本。")?;
    if !latest.pre.is_empty() {
        return Ok((status, None));
    }
    status.latest_version = Some(latest.to_string());
    status.available = latest.cmp_precedence(&current).is_gt();
    status.notes = release.body.unwrap_or_default();
    status.published_at = release.published_at;
    if !status.available {
        return Ok((status, None));
    }
    let suffixes = match (platform, architecture) {
        ("macos", "aarch64") => vec!["macos-aarch64.dmg"],
        ("macos", "x86_64") => vec!["macos-x86_64.dmg"],
        ("windows", "x86_64") => vec!["windows-x86_64-setup.exe", "windows-x86_64.msi"],
        _ => Vec::new(),
    };
    let installer = suffixes.into_iter().find_map(|suffix| {
        let name = format!("snaplingo-{}-{suffix}", release.tag_name);
        let url = format!(
            "https://github.com/gamilian/snaplingo/releases/download/{}/{}",
            release.tag_name, name
        );
        release.assets.iter().find_map(|asset| {
            if asset.name != name
                || asset.browser_download_url != url
                || asset.state != "uploaded"
                || asset.size == 0
                || asset.size > MAX_INSTALLER_SIZE
            {
                return None;
            }
            let digest = asset.digest.as_deref()?.strip_prefix("sha256:")?;
            if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return None;
            }
            Some(UpdateInstaller {
                name: name.clone(),
                url: url.clone(),
                size: asset.size,
                sha256: digest.to_ascii_lowercase(),
            })
        })
    });
    status.download_size = installer.as_ref().map(|installer| installer.size);
    Ok((status, installer))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> PublishedRelease {
        PublishedRelease {
            tag_name: tag.into(),
            draft: false,
            prerelease: false,
            body: Some("Release notes".into()),
            published_at: None,
            assets: [
                "macos-aarch64.dmg",
                "macos-x86_64.dmg",
                "windows-x86_64-setup.exe",
                "windows-x86_64.msi",
            ]
            .into_iter()
            .map(|suffix| {
                let name = format!("snaplingo-{tag}-{suffix}");
                ReleaseAsset {
                    browser_download_url: format!(
                        "https://github.com/gamilian/snaplingo/releases/download/{tag}/{name}"
                    ),
                    name,
                    size: 100,
                    digest: Some(format!("sha256:{}", "a".repeat(64))),
                    state: "uploaded".into(),
                }
            })
            .collect(),
        }
    }

    #[test]
    fn versions_are_compared_semantically_and_never_downgraded() {
        for (current, latest, expected) in [
            ("0.1.9", "v0.1.10", true),
            ("0.2.0", "v0.1.10", false),
            ("0.1.4", "v0.1.4", false),
            ("0.2.0-beta.1", "v0.2.0", true),
        ] {
            let (status, _) =
                select_update(current, "macos", "aarch64", Some(release(latest))).unwrap();
            assert_eq!(status.available, expected);
        }
    }

    #[test]
    fn matches_installed_platform_architecture_and_prefers_windows_nsis() {
        for (platform, architecture, suffix) in [
            ("macos", "aarch64", "macos-aarch64.dmg"),
            ("macos", "x86_64", "macos-x86_64.dmg"),
            ("windows", "x86_64", "windows-x86_64-setup.exe"),
        ] {
            let (_, installer) =
                select_update("0.1.4", platform, architecture, Some(release("v0.1.5"))).unwrap();
            assert!(installer.unwrap().name.ends_with(suffix));
        }
        let (status, installer) =
            select_update("0.1.4", "linux", "aarch64", Some(release("v0.1.5"))).unwrap();
        assert!(status.available);
        assert!(installer.is_none());
    }

    #[test]
    fn ignores_drafts_prereleases_and_missing_releases() {
        for candidate in [
            None,
            Some(PublishedRelease {
                draft: true,
                ..release("v0.1.5")
            }),
            Some(PublishedRelease {
                prerelease: true,
                ..release("v0.1.5")
            }),
            Some(release("v0.1.5-beta.1")),
        ] {
            let (status, installer) =
                select_update("0.1.4", "macos", "aarch64", candidate).unwrap();
            assert!(status.latest_version.is_none());
            assert!(installer.is_none());
        }
    }

    #[test]
    fn untrusted_urls_and_missing_checksums_are_not_downloadable() {
        for index in 0..4 {
            let mut release = release("v0.1.5");
            let asset = &mut release.assets[0];
            match index {
                0 => asset.browser_download_url = "https://example.com/installer.dmg".into(),
                1 => asset.digest = None,
                2 => asset.size = MAX_INSTALLER_SIZE + 1,
                _ => asset.name = "../installer.dmg".into(),
            }
            let (status, installer) =
                select_update("0.1.4", "macos", "aarch64", Some(release)).unwrap();
            assert!(status.available);
            assert!(installer.is_none());
        }
    }
}
