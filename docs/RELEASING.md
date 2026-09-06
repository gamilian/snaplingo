# 零付费桌面发布指南

适用仓库：`gamilian/snaplingo`。配置与官方规则核对日期：2026-09-06。

## 1. 方案与支持范围

| 平台 | 默认产物 | 构建位置 | 默认签名 |
| --- | --- | --- | --- |
| macOS 14+ / Apple Silicon | DMG | Actions `macos-14`，原生 arm64 | 固定自签名 |
| macOS 14+ / Intel | DMG | Actions `macos-15-intel`，原生 x64 | 固定自签名 |
| Windows x64 | NSIS EXE、备选 MSI | Actions `windows-2022` | 未签名，可后续申请 SignPath Foundation |

最低系统版本是构建约束，并不代替对应系统真机验收。Windows runner 是服务器环境，不足以证明所有 Windows 桌面配置都兼容；公开发布前至少验收 Windows 11 x64。此流程不要求本地 Mac 构建 Windows 包。

macOS 默认使用 Vision 系统 OCR，不再动态链接 Homebrew 的 Tesseract/Leptonica。Linux 保留 Tesseract。开发者若明确需要 macOS Tesseract，可自行安装 Tesseract 和语言数据，并运行 `npm run tauri:build -- --features tesseract-ocr`；该自定义包不是自包含的官方包。脚本仍校验所有动态库的最低系统版本和架构，不合格就失败。

无需购买 Apple Developer Program、Windows 证书、Azure 签名服务，也无需登记信用卡。Actions 的 macOS 发布包使用项目固定的自签名证书；它不能替代 Apple 公证，也不会通过 Gatekeeper，但固定证书能让 macOS 将后续版本识别为同一屏幕录制授权身份。Windows 免费可信签名需要申请审核；未获批准前按未签名包发布。账号开通、免费额度及 SignPath 完整申请过程见 [免费账号与签名申请](research/free-release-accounts.md)。

## 2. GitHub 配置与费用边界

1. 用你自己的 GitHub 账号管理公开仓库；开启双重验证，保管恢复码。
2. 仓库 **Settings → Actions → General** 允许运行仓库需要的 Actions。默认 token 保持只读；Release job 单独声明 `contents: write`，不需要长期 PAT。
3. 工作流只使用标准托管 runner，并且所有入口 job 都限制 `repository.private == false`。改成私有后会跳过，而不会消耗私有仓库付费资源。
4. 不启用 larger runner、付费签名服务。不要为 Actions 增加付费预算；在账号 Billing 页面检查 Actions 的预算/超额支出设置，启用到额阻断。不要把免费计算时长理解为任意数量的 artifact 存储都免费。
5. 普通 CI 不上传大文件；手动/测试分支包只保存 3 天，tag 成功后把安装包保存在 GitHub Release。下载完可主动删除旧运行的 artifacts。不要开启无上限的缓存或长期 artifact 保留。
6. 版本 tag 保护和分支保护由仓库维护者管理。仓库已保存 `SNAPLINGO_MACOS_SIGNING_CERTIFICATE_P12_BASE64` 和 `SNAPLINGO_MACOS_SIGNING_CERTIFICATE_PASSWORD` 两个 Actions Secret；它们只在 Desktop packages 的 macOS job 中导入临时钥匙串。不得轮换或删除该证书，除非接受用户需要重新授予屏幕录制权限。PR 的 CI 仍使用 ad-hoc 模式，不读取私钥。

## 3. macOS 零付费构建模式

Desktop packages 以固定自签名证书构建 macOS DMG。证书 P12 与密码保存为 GitHub Actions Secret，每次 macOS job 导入临时钥匙串，工作流校验证书指纹、app 与 DMG 的签名结构、最低系统版本、架构、动态库路径，并从最终 DMG 复制 app 后启动两次。普通 CI 使用 ad-hoc 签名，不读取发布私钥。

固定自签名不能公证，首次启动仍要在“系统设置 → 隐私与安全性 → 仍要打开”手工放行；它不能做到付费 Developer ID 的无警告体验。发布前仍须在干净的 macOS 账户中下载实际 DMG，并重新验证屏幕录制和辅助功能授权。自签名证书保持不变时，TCC 可以将同一 bundle id 的后续版本匹配为同一应用身份；更换证书、改 bundle id 或回退为 ad-hoc 都会要求重新授权。

## 4. 工作流使用方式

### CI：每次 push / PR

运行前端测试、Rust 格式/测试/check、原生打包验证。Windows 执行 NSIS 安装、启动、同版本重装、卸载检查；macOS 从最终 DMG 复制安装、验证签名、启动两次。Linux 同样执行原生测试与打包。

macOS CI 使用临时 ad-hoc 签名，Desktop packages 使用固定自签名证书。启动检查不等同于 Gatekeeper 信任检查，不会绕过或修改系统信任策略。

### Desktop packages：可下载测试包

工作流合入默认分支后，在 **Actions → Desktop packages → Run workflow** 选择分支。

- 默认构建两个 macOS DMG 和 Windows 在线 EXE/MSI。
- 勾选 `windows_offline`，增加带 WebView2 的离线安装包。离线包明显更大；普通包遇到没有运行时的电脑需要联网。
- `codex/release-*` 测试分支的 push 也会触发，便于首次搭建时验证工作流。
- 全部成功后，在运行页 **Artifacts** 下载 `SnapLingo-macos-arm64`、`SnapLingo-macos-x64`、`SnapLingo-windows-x64`；压缩包内有安装包、SHA-256 清单和构建来源 JSON。保留 3 天，下载后可删除 artifact。
- 每个平台独立产物，不要拿 Mac 本地 `.app` 当成 Windows 发布结果。

### tag：创建 Release 草稿

1. 同步 `package.json`、`package-lock.json` 的根版本、`src-tauri/Cargo.toml`、`Cargo.lock` 的根包版本、`src-tauri/tauri.conf.json`，提交变更。
2. 在准备发行的提交上创建匹配版本的 tag，例如版本 `0.2.0`：

   ```sh
   git tag v0.2.0
   git push origin v0.2.0
   ```

3. Desktop packages 校验 tag 对应版本，构建所有默认架构，执行安装验证、生成校验值。
4. 仅所有平台成功后，下载并重新校验 artifacts，再创建 GitHub Release **草稿**，附上安装包、来源文件和安装说明。
5. 完成下面的交互验收，编辑版本变更说明，再点 Publish release。失败时不要复用同一版本号发布不同内容；草稿重跑前先检查并清理旧草稿，避免混合产物。

构建用 `Cargo.lock` 和 `npm ci` 固定依赖。普通构建仅清理 bundle，不删除整个 Rust release 缓存；`npm run tauri:build:beta` 才执行完整清理。避免绕过统一入口直接运行 `npm run tauri build`。

## 5. 验收边界与必测场景

自动化已经检查：版本、产物非空/可执行、macOS 签名完整性与固定证书指纹/最低系统版本/动态库路径与架构、最终 DMG 内容、Windows 安装退出码和实际安装目录、程序存活、同版本重装。`npm run release:verify` 会重新做 macOS 原生验证，不只是检查文件存在。

自动化**不能证明**真实用户能截图、所有 OCR 语言正常、TCC 跨版本保留，或浏览器下载后的系统信任提示符合预期。公开发布前在干净用户/电脑上完成：

| 场景 | 验收要求 |
| --- | --- |
| 浏览器下载、首次安装 | 按发布说明可安装；macOS 通过“隐私与安全性 → 仍要打开”手工放行；Windows 根据实际设备策略处理 |
| 拒绝所有权限 | 设置、文本翻译、文件 OCR 可用；可跳过权限向导 |
| 只授予屏幕录制 | 可截图、截图 OCR/翻译；无需先授予辅助功能 |
| 使用选中文本/元素检测 | 按需提示辅助功能；浏览器自动化有明确用途说明 |
| Windows WebView2 缺失 | 在线版在联网环境自动安装；离线版在断网且未安装运行时的测试机安装成功 |
| Windows OCR 语言缺失 | 有可理解的安装语言提示；补充系统 OCR 语言能力后重新启动验证 |
| 真实 N→N+1 更新 | 同架构覆盖安装；数据库、API 配置、历史保留；截图/选中文本授权复测，并确认固定证书未变时屏幕录制授权仍被识别 |
| 多屏、缩放 | Windows 混合 DPI，macOS Retina/外接屏截图和选区正确 |

自签名包的 `spctl` 拒绝是预期结果；脚本记录结果，不要求用户关闭 Gatekeeper、删除 quarantine 或修改 TCC。

## 6. 免费 Windows 签名的后续接入

当前可立即交付的路线是未签名 EXE/MSI，不需要额外账号。SignPath 是可选增强，申请过程见 [免费账号与签名申请](research/free-release-accounts.md)。项目许可证目前是 TBD；维护者需要先确定一个合格的开源许可证，本次不代替作者选择或申请。

获批后的正确次序：可信 Actions 构建 → SignPath 校验构建来源并签名（包含内嵌应用与安装器）→ 下载签名结果 → 再做安装测试 → 重新计算 SHA-256 → 上传 Release。不能只替换安装器签名却保留旧校验值，也不能使用面向测试的证书冒充可信签名。审批前不创建会阻断免费未签名发布的签名步骤；实际项目 ID、策略和 artifact 配置以审核通过的配置为准。

## 官方参考

- [GitHub 标准 runner 与架构](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [GitHub Actions 计费](https://docs.github.com/en/billing/concepts/product-billing/github-actions)
- [Tauri Windows 安装器与 WebView2](https://v2.tauri.app/distribute/windows-installer/)
- [Apple：安全打开 Mac App](https://support.apple.com/en-us/102445)
- [SignPath Foundation 条件](https://signpath.org/terms)
