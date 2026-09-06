# 免费 macOS 分发策略（SnapLingo）

调研日期：2026-08-20。目标是为需要 Screen Recording，且部分功能需要 Accessibility 的开源桌面应用，选择**不购买 Apple Developer Program**时风险最低的分发方式；范围仅限直接分发，不含 Mac App Store。

## 结论

采用“**固定自签名证书 + 固定 `CFBundleIdentifier` + GitHub Release 的 DMG + 清晰的首次安装说明**”。它适合小范围 beta、技术用户和开源贡献者；不要把 ad-hoc 签名作为常规更新方案。

这不能得到 Gatekeeper 的直接放行，也不能公证。首次启动需要用户在“系统设置 → 隐私与安全性”中选择“仍要打开”，然后自行授予 Screen Recording；只有使用需要控制其他应用的功能时，才要求 Accessibility。不要让用户关闭 Gatekeeper、执行 `xattr -cr`，或让安装程序尝试修改 TCC 数据库。

Apple 的规则很明确：公证要求 Developer ID 证书，且要求 hardened runtime 和安全时间戳；ad-hoc、Apple Developer 或本地开发证书都不符合要求。[Apple 公证要求](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution.md) 也说明 Gatekeeper 用在线或 stapled 的票据识别公证产物。因此“免费且无警告”不是可实现的组合。

## 为什么不是 ad-hoc

`codesign --sign -` 中的 `-` 是 **ad-hoc signing**，不是自签名证书；它没有签名身份，只标识一个具体 code instance。每次构建的 code identity 都可能变化，尤其不适合依赖 TCC 的更新链路。

Snapzy 的可复现发布设计正好比较了这两种免费路径：它用一张长期固定的自签名 Code Signing 证书发布 beta，并提供“先授权、覆盖安装后检查授权”的脚本；对照组用 ad-hoc 重新签名。该项目把固定自签名列为 TCC 连续性的较好选择，把 ad-hoc 列为不适合常规发布的兜底。[证书说明](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/docs/SELF_SIGNED_CERT.md) · [本地对照测试](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/scripts/test-tcc-local.sh) · [CI 的三层签名选择](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/.github/workflows/release-publish.yml#L91-L180)。

这是有价值的开源项目实测，而**不是 Apple 对 TCC 跨版本保留的保证**。Apple 公开文档只保证用户可在系统设置中管理 Accessibility，并提供检查/请求 Screen Recording 的 API；每次发行都必须在真实升级路径上复测，需要时引导用户重新授权。[Accessibility 设置](https://support.apple.com/guide/mac-help/allow-accessibility-apps-to-access-your-mac-mh43185/mac) · [`CGPreflightScreenCaptureAccess`](https://developer.apple.com/documentation/coregraphics/cgpreflightscreencaptureaccess()) · [`CGRequestScreenCaptureAccess`](https://developer.apple.com/documentation/coregraphics/cgrequestscreencaptureaccess())。

## 与 CC Switch 的对照

CC Switch 的历史免费发布体验正是该策略的用户侧代价：在作者还没有 Apple Developer 账号时，其 v3.12.2 发布说明要求用户在“隐私与安全性”中点“仍要打开”。[历史说明](https://github.com/farion1231/cc-switch/blob/0b5da510168914b251481654a568c3ffacd62cf4/docs/release-notes/v3.12.2-ja.md#L110-L116)。当前 CI 则导入 `Developer ID Application`、使用 Apple 凭据构建/公证，并对 app 和 DMG stapling——这正是付费后才能消除安装摩擦的路线。[当前发布工作流](https://github.com/farion1231/cc-switch/blob/0b5da510168914b251481654a568c3ffacd62cf4/.github/workflows/release.yml#L145-L340)。

Apple 也明确把这一步留给用户：对未能验证开发者、或未公证的软件，用户可在 Privacy & Security 中选择 Open Anyway；这不是发行方能免费自动跳过的弹窗。[Apple：安全地打开 Mac App](https://support.apple.com/en-us/102445)。

## SnapLingo 的直接可执行流程

前提：把一张自签名的 **Code Signing** 证书连同私钥安全保存（建议导出加密 `.p12` 放密码管理器；CI 用仓库 secret 导入同一份）。绝不重新生成它来发布普通更新；也不要改变 `com.snaplingo.app`。若证书遗失或改了 bundle identifier，按新 app/新 TCC 身份处理并重新测试授权。

1. 首次建立专用身份时，使用带 `codeSigning` extended key usage 的自签名证书；Snapzy 的生成脚本是可审阅范例，包含 `.p12` 导出与钥匙串导入步骤：[create-signing-cert.sh](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/scripts/create-signing-cert.sh)。不要把私钥提交到 Git。
2. 使用 SnapLingo 的唯一发行入口构建，而不是裸跑 `tauri build`。当前入口会在 Tauri 初始 ad-hoc 打包后重签 `.app`、重建 DMG，并校验固定 identifier、非 ad-hoc 签名和 hardened runtime：

   ```sh
   SNAPLINGO_CODESIGN_IDENTITY='SnapLingo Local Code Signing' \
   npm run tauri:build
   ```

   `SnapLingo Local Code Signing` 必须是步骤 1 创建并持续复用的身份。当前 [`src-tauri/tauri.conf.json`](../../src-tauri/tauri.conf.json) 的 `signingIdentity` 为 `"-"`，所以跳过该入口会留下 ad-hoc 产物；重签逻辑见 [`script/fix-macos-release-signing.mjs`](../../script/fix-macos-release-signing.mjs)。
3. 只上传最终 DMG 到 GitHub Release，并同时上传 SHA-256 校验值和简短安装说明。DMG 内保留 `/Applications` 拖拽链接；不要把 app 再二次修改、压缩后重新签名，或让下载站给产物加内容。
4. 在干净的非开发用户帐户验收：下载 DMG → 拖入 `/Applications` → 首次选择“仍要打开” → 执行截图并批准 Screen Recording → 仅在对应功能中批准 Accessibility。再用**同一张证书**发布一次更高版本并覆盖安装，重复这组检查。

建议把 beta 页面直接写成：

> 本 beta 未经过 Apple 公证。将 SnapLingo 拖入“应用程序”后首次打开，如 macOS 阻止启动，请前往“系统设置 → 隐私与安全性”选择“仍要打开”。截图首次使用时需批准“屏幕录制”；仅启用自动化/选中文本等功能时需批准“辅助功能”。

## 每次发布的不可省略检查

```sh
APP='target/release/bundle/macos/SnapLingo.app'
DMG='target/release/bundle/dmg/SnapLingo_*.dmg'

codesign --verify --deep --strict --verbose=4 "$APP"
codesign -dv --verbose=4 "$APP" 2>&1
codesign -d -r- "$APP" 2>&1
spctl --assess --type execute --verbose=4 "$APP"
shasum -a 256 "$DMG"
```

`codesign --verify` 证明包的签名结构和完整性，不等同于 Gatekeeper 信任；`spctl --assess --type execute` 请求本机系统策略评估，拒绝是免费自签名 beta 的预期结果，应记录而不是用 `spctl --add` 伪造通过。不要向用户或 CI 写入 Gatekeeper 例外规则。

测试机需要重新走首次授权时，可使用以下**仅限人工测试**的重置命令，之后完全退出并重新打开 app：

```sh
tccutil reset ScreenCapture com.snaplingo.app
tccutil reset Accessibility com.snaplingo.app
```

它只能重置本机授权，不能授予权限，绝不能放入安装或应用运行流程。

## 何时停止使用免费路径

满足任一条件就应升级为 Developer ID + notarization：面向非技术公众发布、需要“下载后双击即可开”、要上 Homebrew Cask 作为主安装渠道、或无法接受用户每次因签名/路径变化而可能重新授权。届时 Apple 的命令行流程是 `xcrun notarytool submit … --wait`、`xcrun stapler staple …`，但其前提仍是 Developer ID；不应尝试把自签名产物提交公证。

## 证据与边界

- Apple 的 `codesign(1)` 将 identity 为 `-` 定义为 ad-hoc signing；`spctl(8)` 说明 `--assess --type execute` 是系统策略执行评估。这两份 Apple 命令手册随 Xcode/macOS 安装，本机可用 `man codesign`、`man spctl` 查阅。
- Apple 公证文档：要求 Developer ID、hardened runtime 和 secure timestamp，并说明公证票据与 Gatekeeper 的关系：[Notarizing macOS software before distribution](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution.md)。
- 此策略仅承诺“复用同一自签名身份可避免 ad-hoc 每构建一变”的工程做法；不承诺 Apple 未公开保证的 TCC 持久性。发布验收优先于推断。
