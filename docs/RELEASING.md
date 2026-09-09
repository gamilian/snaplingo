# 零付费桌面发布指南

适用仓库：`gamilian/snaplingo`。配置与官方规则核对日期：2026-09-06。

## 1. 方案与支持范围

| 平台 | 默认产物 | Release 文件名 | 构建位置 | 默认签名 |
| --- | --- | --- | --- | --- |
| macOS 12+ / Apple Silicon | DMG | `snaplingo-vX.Y.Z-macos-aarch64.dmg` | Actions `macos-14`，原生 arm64 | ad-hoc 签名 |
| macOS 12+ / Intel | DMG | `snaplingo-vX.Y.Z-macos-x86_64.dmg` | Actions `macos-15-intel`，原生 x64 | ad-hoc 签名 |
| Windows 10 / 11 x64 | NSIS EXE、备选 MSI | `snaplingo-vX.Y.Z-windows-x86_64-setup.exe`、`snaplingo-vX.Y.Z-windows-x86_64.msi` | Actions `windows-2022` | 未签名，可后续申请 SignPath Foundation |

以上是兼容目标；最低系统版本是构建约束，并不代替对应系统真机验收。公开发布前需覆盖 macOS 12、13、14、15 及更新版本，以及 Windows 10 / 11 x64 的安装、首次授权、拖动选区、预览、复制、保存和 OCR。Windows runner 是服务器环境，不足以证明 Windows 桌面配置都兼容。此流程不要求本地 Mac 构建 Windows 包。

macOS 构建最低版本为 12.0，前端以 Safari 15 / Chromium 105 为目标；Windows 使用安装器配置的 Evergreen WebView2。前端构建拒绝目标引擎不支持的正则语法，OCR 字段提取使用捕获组，避免依赖 [Safari 16.4 才支持的正则回溯](https://webkit.org/blog/13966/webkit-features-in-safari-16-4/)。Vision 自动识别语言仅在 macOS 13+ 启用，macOS 12 使用指定的识别语言。不得只改安装包的 Info.plist 来降低最低版本，必须重新编译并通过所有内嵌二进制的 deployment target 校验。

macOS 默认使用 Vision 系统 OCR，不再动态链接 Homebrew 的 Tesseract/Leptonica。Linux 保留 Tesseract。开发者若明确需要 macOS Tesseract，可自行安装 Tesseract 和语言数据，并运行 `npm run tauri:build -- --features tesseract-ocr`；该自定义包不是自包含的官方包。脚本仍校验所有动态库的最低系统版本和架构，不合格就失败。

无需购买 Apple Developer Program、Windows 证书、Azure 签名服务，也无需登记信用卡。GitHub-hosted runner 无法可靠地信任本地自签名根证书，因此 Actions 的 macOS 包使用 ad-hoc 签名；它不能替代 Apple 公证，也不能承诺跨更新保留 TCC 授权。Windows 免费可信签名需要申请审核；未获批准前按未签名包发布。账号开通、免费额度及 SignPath 完整申请过程见 [免费账号与签名申请](research/free-release-accounts.md)。

## 2. GitHub 配置与费用边界

1. 用你自己的 GitHub 账号管理公开仓库；开启双重验证，保管恢复码。
2. 仓库 **Settings → Actions → General** 允许运行仓库需要的 Actions。默认 token 保持只读；Release job 单独声明 `contents: write`，不需要长期 PAT。
3. 工作流只使用标准托管 runner，并且所有入口 job 都限制 `repository.private == false`。改成私有后会跳过，而不会消耗私有仓库付费资源。
4. 不启用 larger runner、付费签名服务。不要为 Actions 增加付费预算；在账号 Billing 页面检查 Actions 的预算/超额支出设置，启用到额阻断。不要把免费计算时长理解为任意数量的 artifact 存储都免费。
5. 普通 CI 不上传大文件；手动/测试分支包只保存 3 天，tag 成功后把安装包保存在 GitHub Release。下载完可主动删除旧运行的 artifacts。不要开启无上限的缓存或长期 artifact 保留。
6. 版本 tag 保护和分支保护由仓库维护者管理。Desktop packages 不读取 macOS 私钥；PR 的 CI 也使用相同的 ad-hoc 模式。

## 3. macOS 零付费构建模式

Desktop packages 和 CI 都以 ad-hoc 签名构建 macOS DMG，因此不需要在 GitHub Secrets 中保存或导入 Mac 私钥。工作流仍会校验 app 与 DMG 的签名结构、最低系统版本、架构、动态库路径，并从最终 DMG 复制 app 后启动两次。

ad-hoc 签名每次构建都可能变化，不能替代稳定发行身份，也无法公证。它适合公开的测试版和首发验证；发布前必须在干净的 macOS 账户中下载实际 DMG，按“系统设置 → 隐私与安全性 → 仍要打开”完成首次启动，并重新验证屏幕录制和辅助功能授权。面向非技术公众或需要稳定更新授权时，需要改用 Apple Developer ID 和公证，这不是零付费能力。

## 4. 工作流使用方式

### CI：每次 push / PR

运行前端测试、Rust 格式/测试/check、原生打包验证。Windows 分别执行 NSIS 和 MSI 安装、启动、重装、卸载检查；macOS 从最终 DMG 复制安装、验证签名、启动两次。Linux 同样执行原生测试与打包。

macOS CI 和 Desktop packages 都使用临时 ad-hoc 签名。启动检查不等同于 Gatekeeper 信任检查，不会绕过或修改系统信任策略。

### Desktop packages：可下载测试包

工作流合入默认分支后，在 **Actions → Desktop packages → Run workflow** 选择分支。

- 默认构建两个 macOS DMG 和 Windows 在线 EXE/MSI。
- 勾选 `windows_offline`，增加带 WebView2 的离线安装包。离线包明显更大；普通包遇到没有运行时的电脑需要联网。
- `codex/release-*` 测试分支的 push 也会触发，便于首次搭建时验证工作流。
- 全部成功后，在运行页 **Artifacts** 下载 `snaplingo-macos-aarch64`、`snaplingo-macos-x86_64`、`snaplingo-windows-x86_64`；压缩包内有按 `snaplingo-vX.Y.Z-平台-架构[-变体][-安装器]` 命名的安装包、SHA-256 清单和构建来源 JSON。保留 3 天，下载后可删除 artifact。
- 每个平台独立产物，不要拿 Mac 本地 `.app` 当成 Windows 发布结果。

### tag：创建 Release 草稿

1. 同步 `package.json`、`package-lock.json` 的根版本、`src-tauri/Cargo.toml`、`Cargo.lock` 的根包版本、`src-tauri/tauri.conf.json`，提交变更。
2. 从 `docs/RELEASE_NOTES_TEMPLATE.md` 创建 `docs/releases/v0.2.0.md`，填写本版面向用户的变化、安装说明和已知限制。
3. 在准备发行的提交上创建匹配版本的 tag，例如版本 `0.2.0`：

   ```sh
   git tag v0.2.0
   git push origin v0.2.0
   ```

4. Desktop packages 在构建矩阵开始前统一校验所有版本来源和 tag，随后构建所有默认架构，执行安装验证并生成校验值。
5. 仅所有平台成功后，下载并重新校验 artifacts，再使用对应的版本化 Release notes 创建 GitHub Release **草稿**。
6. 完成下面的交互验收后点 Publish release。缺少对应版本的 notes 或包含占位内容时，工作流会失败；失败时不要复用同一版本号发布不同内容。

构建用 `Cargo.lock` 和 `npm ci` 固定依赖。普通构建仅清理 bundle，不删除整个 Rust release 缓存；`npm run tauri:build:beta` 才执行完整清理。避免绕过统一入口直接运行 `npm run tauri build`。

## 5. 验收边界与必测场景

自动化已经检查：版本、产物非空/可执行、macOS ad-hoc 签名完整性/最低系统版本/动态库路径与架构、最终 DMG 内容、Windows NSIS/MSI 安装退出码和实际安装目录、程序存活、卸载与重装后的用户数据保留。`npm run release:verify` 会重新做 macOS 原生验证，不只是检查文件存在。

自动化**不能证明**真实用户能截图、所有 OCR 语言正常、TCC 跨版本保留，或浏览器下载后的系统信任提示符合预期。公开发布前在干净用户/电脑上完成：

| 场景 | 验收要求 |
| --- | --- |
| 浏览器下载、首次安装 | 按发布说明可安装；macOS 通过“隐私与安全性 → 仍要打开”手工放行；Windows 根据实际设备策略处理 |
| 拒绝所有权限 | 设置、文本翻译、文件 OCR 可用；可跳过权限向导 |
| 只授予屏幕录制 | 可截图、截图 OCR/翻译；无需先授予辅助功能 |
| 使用选中文本/元素检测 | 按需提示辅助功能；浏览器自动化有明确用途说明 |
| Windows WebView2 缺失 | 在线版在联网环境自动安装；离线版在断网且未安装运行时的测试机安装成功 |
| Windows OCR 语言缺失 | 有可理解的安装语言提示；补充系统 OCR 语言能力后重新启动验证 |
| 真实 N→N+1 更新 | 同架构覆盖安装；数据库、API 配置、历史保留；截图/选中文本授权复测，并接受 ad-hoc 身份变化可能要求重新授权 |
| 多屏、缩放 | Windows 混合 DPI，macOS Retina/外接屏截图和选区正确 |
| 遮罩与十字光标出现后拖动 | 按下、拖动、松开应形成选区并生成预览；Esc 可退出。若失败，在设置中启用性能监控后复现，检查日志中的 `[capture-input]` 和 `selection_pointer_down` / `selection_pointer_up`，区分原生焦点与前端输入 |

ad-hoc 包的 `spctl` 拒绝是预期结果；脚本记录结果，不要求用户关闭 Gatekeeper、删除 quarantine 或修改 TCC。

### 应用内升级与权限恢复

- **设置 → 通用 → 关于** 在本次设置会话中自动检查一次 GitHub 最新正式版，也可点击“检查更新”重试。只比较正式版本号，不提示降级；每次发行必须提升版本号，替换同一 tag 下的安装包不会触发升级。
- 根据 macOS / Windows 和架构选择仓库的标准安装包；Intel 版在 Rosetta 下运行时优先下载 Apple Silicon 版。点击“下载并打开安装包”后，程序校验 GitHub 发布元数据中的 SHA-256 和大小，失败即删除临时文件，不打开安装包。未提供匹配文件或有效校验值时保留发布页入口。
- 更新使用现有网络代理设置，不需要 Apple Developer ID、额外更新服务器或签名服务账号。macOS 下载保留 quarantine 标记，Windows 保留下载来源标记；系统安全放行流程照常。此功能打开 DMG / 系统安装器，由用户完成覆盖安装，不在后台替换正在运行的应用。
- 首次授权使用简短弹窗，只说明屏幕录制用途，提供“去授权”和“稍后”；返回后自动检测并关闭。辅助功能在使用相关功能时再请求，不在首次弹窗展示。引导只出现在设置窗口，不会在翻译、文件 OCR、贴图窗口重复弹出。
- **设置 → 通用 → 系统权限** 显示实时授权状态。“授权遇到问题？”中提供重新检测、重新授权和按需重启；首次弹窗尝试授权后，也可从同名链接进入此处。
- 从 DMG 或 App Translocation 临时位置运行时，先安装到“应用程序”再授权。旧授权已开启但新版仍不可用时，在“授权遇到问题？”中确认重置具体权限。只调用 `tccutil reset ScreenCapture <当前 bundle ID>` 或 `tccutil reset Accessibility <当前 bundle ID>`；不清除其他应用或其他权限，也不在启动/检查更新时自动重置。
- 修复后仍需用户在系统设置允许当前应用。macOS 要求“退出并重新打开”时可点击“重启 SnapLingo”，无需反复手动寻找并退出进程。ad-hoc 签名仍不能保证跨版本保留授权；应用不会把本地“曾经允许”当作当前系统授权。

额外交互验收：已有旧版授权后覆盖安装；只修复屏幕录制而保留辅助功能；从 DMG 直接启动；拒绝权限后打开翻译/文件 OCR；更新检查断网后重试；下载中重复点击；实际 N→N+1 下载并安装后保留数据库。

## 6. 免费 Windows 签名的后续接入

当前可立即交付的路线是未签名 EXE/MSI，不需要额外账号。SignPath 是可选增强，申请过程见 [免费账号与签名申请](research/free-release-accounts.md)。项目许可证目前是 TBD；维护者需要先确定一个合格的开源许可证，本次不代替作者选择或申请。

获批后的正确次序：可信 Actions 构建 → SignPath 校验构建来源并签名（包含内嵌应用与安装器）→ 下载签名结果 → 再做安装测试 → 重新计算 SHA-256 → 上传 Release。不能只替换安装器签名却保留旧校验值，也不能使用面向测试的证书冒充可信签名。审批前不创建会阻断免费未签名发布的签名步骤；实际项目 ID、策略和 artifact 配置以审核通过的配置为准。

## 官方参考

- [GitHub 标准 runner 与架构](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [GitHub Actions 计费](https://docs.github.com/en/billing/concepts/product-billing/github-actions)
- [Tauri Windows 安装器与 WebView2](https://v2.tauri.app/distribute/windows-installer/)
- [Apple：安全打开 Mac App](https://support.apple.com/en-us/102445)
- [SignPath Foundation 条件](https://signpath.org/terms)
