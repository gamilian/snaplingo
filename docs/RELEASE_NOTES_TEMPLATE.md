# Release notes template

复制本文件为 `docs/releases/vX.Y.Z.md`，填写版本变化后再发布。该文件只是维护者模板，不会被 Release 工作流直接上传。

## 本版变化

- 在这里填写本版本面向用户的实际变化。

## 下载与安装

- macOS 14 或更新：Apple Silicon 下载 `aarch64.dmg`；Intel 下载 `x64.dmg`。拖入“应用程序”后启动。
- Windows x64：优先下载 `x64-setup.exe`（当前用户安装）；MSI 为备选安装器，不必重复安装。
- Windows 缺少 WebView2 时，普通安装器需要联网。带 `offline` 的包内含运行时；该包仅在维护者选择构建时提供。

## 免费版本的签名说明

macOS 使用 ad-hoc 签名，未经 Apple 公证。首次启动被拦截时，请在“系统设置 → 隐私与安全性”中选择“仍要打开”。截图需要屏幕录制权限；选中文本和界面元素检测需要辅助功能权限。可以稍后授权，先使用文本翻译和文件 OCR。

Windows 当前未使用公开信任的代码签名，可能显示未知发布者或 SmartScreen 提示。仅从本仓库下载并核对校验值。若设备策略或 Smart App Control 阻止运行，请联系设备管理员；不要关闭系统防护。未来获得免费签名批准后，发布说明会明确标示实际签名状态。

macOS / Windows 默认使用系统 OCR。Windows 需要安装对应的系统 OCR 语言能力；未安装时请在系统语言设置中补充。当前 macOS 官方构建不包含 Tesseract。

## 校验与更新

每个平台附 `SHA256SUMS-*.txt` 和 `build-*.json`，记录版本、源码提交和签名方式。

- macOS：`shasum -a 256 下载的文件.dmg`
- Windows PowerShell：`Get-FileHash .\下载的文件.exe -Algorithm SHA256`

更新前退出 SnapLingo，覆盖安装同一架构版本。系统隐私授权是否跨版本保留仍需实际检查。正常覆盖安装不应删除 API 配置或历史记录。
