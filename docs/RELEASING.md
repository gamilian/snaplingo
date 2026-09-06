# 零付费桌面发布指南

适用仓库：`gamilian/snaplingo`。配置与官方规则核对日期：2026-09-06。

## 1. 方案与支持范围

| 平台 | 默认产物 | 构建位置 | 默认签名 |
| --- | --- | --- | --- |
| macOS 14+ / Apple Silicon | DMG | Actions `macos-14`，原生 arm64 | 固定自签名证书 |
| macOS 14+ / Intel | DMG | Actions `macos-15-intel`，原生 x64 | 与 arm64 同一张证书 |
| Windows x64 | NSIS EXE、备选 MSI | Actions `windows-2022` | 未签名，可后续申请 SignPath Foundation |

最低系统版本是构建约束，并不代替对应系统真机验收。Windows runner 是服务器环境，不足以证明所有 Windows 桌面配置都兼容；公开发布前至少验收 Windows 11 x64。此流程不要求本地 Mac 构建 Windows 包。

macOS 默认使用 Vision 系统 OCR，不再动态链接 Homebrew 的 Tesseract/Leptonica。Linux 保留 Tesseract。开发者若明确需要 macOS Tesseract，可自行安装 Tesseract 和语言数据，并运行 `npm run tauri:build -- --features tesseract-ocr`；该自定义包不是自包含的官方包。脚本仍校验所有动态库的最低系统版本和架构，不合格就失败。

无需购买 Apple Developer Program、Windows 证书、Azure 签名服务，也无需登记信用卡。免费自签名不能替代 Apple 公证。Windows 免费可信签名需要申请审核；未获批准前按未签名包发布。账号开通、免费额度及 SignPath 完整申请过程见 [免费账号与签名申请](research/free-release-accounts.md)。

## 2. GitHub 配置与费用边界

1. 用你自己的 GitHub 账号管理公开仓库；开启双重验证，保管恢复码。
2. 仓库 **Settings → Actions → General** 允许运行仓库需要的 Actions。默认 token 保持只读；Release job 单独声明 `contents: write`，不需要长期 PAT。
3. 工作流只使用标准托管 runner，并且所有入口 job 都限制 `repository.private == false`。改成私有后会跳过，而不会消耗私有仓库付费资源。
4. 不启用 larger runner、付费签名服务。不要为 Actions 增加付费预算；在账号 Billing 页面检查 Actions 的预算/超额支出设置，启用到额阻断。不要把免费计算时长理解为任意数量的 artifact 存储都免费。
5. 普通 CI 不上传大文件；手动/测试分支包只保存 3 天，tag 成功后把安装包保存在 GitHub Release。下载完可主动删除旧运行的 artifacts。不要开启无上限的缓存或长期 artifact 保留。
6. 版本 tag 保护、分支保护及签名私钥访问权限由仓库维护者管理。只有可信分支才能运行带证书的 Desktop packages；PR 的 CI 不接触签名私钥。

## 3. 一次性配置 macOS 长期证书

这一步不需要 Apple 账号。自签名证书必须同时备份证书和私钥，两个架构、不同电脑和每次更新都复用它。不要把同名的新证书当成旧证书。

### 已有本机 SnapLingo 签名证书（本项目的常规情况）

安装并登录 GitHub CLI：

```sh
brew install gh
gh auth login
# 选择 GitHub.com、通过浏览器登录自己的账号；确认对目标仓库有管理权限。
gh auth status
```

在项目根目录运行：

```sh
node script/configure-macos-release.mjs gamilian/snaplingo
```

脚本会复用 `~/.snaplingo/codesign/` 中已有证书；第一次使用时才创建。它通过 stdin 把加密的 P12 和密码分别写入 GitHub Secrets，并固定证书 SHA-1 指纹。不会在日志打印私钥、P12 或密码。若仓库已固定另一张证书，脚本停止，避免无意轮换身份。

仓库 **Settings → Secrets and variables → Actions** 最终应有：

| 类型 | 名称 | 内容 |
| --- | --- | --- |
| Secret | `MACOS_CERTIFICATE_P12` | 加密 P12 文件的 Base64 |
| Secret | `MACOS_CERTIFICATE_PASSWORD` | P12 的密码 |
| Variable | `MACOS_CERTIFICATE_SHA1` | 40 位证书指纹，无冒号 |

证书指纹用于身份固定，并非安装包的 SHA-256 校验值。

**备份：** 把 `~/.snaplingo/codesign/SnapLingoLocalCodeSigning.p12` 存入加密备份；其密码在同目录 `keychain-password.txt`，应另存密码管理器。限制本地目录访问权限；不要提交这些文件、截图或粘贴到 Issue。丢失密钥后，无法通过重新生成同名证书恢复原有身份。

### 从其他电脑迁移或手工配置

1. 在原电脑“钥匙串访问”中找到用于既有发布的 Code Signing 身份，展开确认存在私钥，导出为带密码的 `.p12`。不要仅导出 `.cer`。
2. 将该 P12 转为 Base64 后填入上表 Secret；把导出密码放入独立 Secret。
3. 在原证书详情获取 SHA-1，去掉冒号后填入上表 Variable。不要用 P12 文件的哈希冒充证书指纹。
4. 在 Actions 触发一次测试构建。导入器会先核对指纹，再导入临时钥匙串；缺配置或身份不符立即失败，不会自动换证书。
5. 成功后确认两个 macOS `build-*.json` 中证书指纹一致；下载 DMG 实测更新。

临时 runner 会在最后删除导入的钥匙串及目录。构建证书只供构建器使用，不要求安装用户信任或安装你的根证书。

## 4. 工作流使用方式

### CI：每次 push / PR

运行前端测试、Rust 格式/测试/check、原生打包验证。Windows 执行 NSIS 安装、启动、同版本重装、卸载检查；macOS 从最终 DMG 复制安装、验证签名、启动两次。Linux 同样执行原生测试与打包。

macOS CI 使用临时 ad-hoc 签名，不接触发布密钥，也不把该包作为稳定发行。CI 的启动检查不等同于 Gatekeeper 信任检查，不会绕过或修改系统信任策略。

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

自动化已经检查：版本、产物非空/可执行、macOS 签名完整性/固定身份/最低系统版本/动态库路径与架构、最终 DMG 内容、Windows 安装退出码和实际安装目录、程序存活、同版本重装。`npm run release:verify` 会重新做 macOS 原生验证，不只是检查文件存在。

自动化**不能证明**真实用户能截图、所有 OCR 语言正常、TCC 跨版本保留，或浏览器下载后的系统信任提示符合预期。公开发布前在干净用户/电脑上完成：

| 场景 | 验收要求 |
| --- | --- |
| 浏览器下载、首次安装 | 按发布说明可安装；macOS 通过“隐私与安全性 → 仍要打开”手工放行；Windows 根据实际设备策略处理 |
| 拒绝所有权限 | 设置、文本翻译、文件 OCR 可用；可跳过权限向导 |
| 只授予屏幕录制 | 可截图、截图 OCR/翻译；无需先授予辅助功能 |
| 使用选中文本/元素检测 | 按需提示辅助功能；浏览器自动化有明确用途说明 |
| Windows WebView2 缺失 | 在线版在联网环境自动安装；离线版在断网且未安装运行时的测试机安装成功 |
| Windows OCR 语言缺失 | 有可理解的安装语言提示；补充系统 OCR 语言能力后重新启动验证 |
| 真实 N→N+1 更新 | 同架构、同 macOS 证书覆盖安装；数据库、API 配置、历史保留；截图/选中文本授权复测 |
| 多屏、缩放 | Windows 混合 DPI，macOS Retina/外接屏截图和选区正确 |

自签名包 `spctl` 返回拒绝是预期结果；脚本记录结果，不要求用户关闭 Gatekeeper、删除 quarantine 或修改 TCC。固定证书减少身份变化，但不承诺系统隐私授权永久保留。

## 6. 免费 Windows 签名的后续接入

当前可立即交付的路线是未签名 EXE/MSI，不需要额外账号。SignPath 是可选增强，申请过程见 [免费账号与签名申请](research/free-release-accounts.md)。项目许可证目前是 TBD；维护者需要先确定一个合格的开源许可证，本次不代替作者选择或申请。

获批后的正确次序：可信 Actions 构建 → SignPath 校验构建来源并签名（包含内嵌应用与安装器）→ 下载签名结果 → 再做安装测试 → 重新计算 SHA-256 → 上传 Release。不能只替换安装器签名却保留旧校验值，也不能使用面向测试的证书冒充可信签名。审批前不创建会阻断免费未签名发布的签名步骤；实际项目 ID、策略和 artifact 配置以审核通过的配置为准。

## 官方参考

- [GitHub 标准 runner 与架构](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [GitHub Actions 计费](https://docs.github.com/en/billing/concepts/product-billing/github-actions)
- [Tauri Windows 安装器与 WebView2](https://v2.tauri.app/distribute/windows-installer/)
- [Apple：安全打开 Mac App](https://support.apple.com/en-us/102445)
- [SignPath Foundation 条件](https://signpath.org/terms)
