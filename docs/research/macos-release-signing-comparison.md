# macOS 发布签名、公证与 TCC 权限对比

调研基线：EasyDict [`eaff88b4`](https://github.com/tisfeng/Easydict/tree/eaff88b4e981714c127b62aa8068f8ba5587cfcf)、Snapzy [`bf8ca39f`](https://github.com/duongductrong/Snapzy/tree/bf8ca39f8e13915b203b9aa3c73066b75f94284f)，以及本工作区当前的 SnapLingo 配置。这里的“发布”指 Developer ID 分发（非 Mac App Store）。

## 结论

SnapLingo 已有比普通 Tauri 项目更完整的二次签名、原生依赖打包与可选公证流程；问题不在于缺少基础能力，而在于**公开发布的关键约束不是强制的**。发布时应显式使用固定的 `Developer ID Application` 身份、始终公证并在最终产物上做 Gatekeeper 验证。不要把 `tauri build` 的直接输出当作发行包。

| 关注点 | EasyDict | Snapzy | SnapLingo 当前状态与结论 |
| --- | --- | --- | --- |
| 发布签名 | 预检强制 `Developer ID Application`，Xcode archive/export 使用手工、固定身份和时间戳。见 [preflight](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/scripts/release/release-preflight.sh)、[build](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/scripts/release/release-build.sh)。 | CI 优先 Developer ID，其次固定自签名；默认拒绝 ad-hoc，并比较新旧 designated requirement，防止更新后 TCC 身份漂移。见 [workflow](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/.github/workflows/release-publish.yml)。 | `tauri.conf.json` 的 `signingIdentity: "-"` 先产生 ad-hoc 包；`fix-macos-release-signing.mjs` 才会重签。**必须从 `build-release.sh` 进入**，并显式设置 `SNAPLINGO_CODESIGN_IDENTITY`，否则脚本会挑选任一可用身份或回退到本地自签名。 |
| Hardened runtime / entitlements | Release 配置启用 hardened runtime；发行 entitlement 为空，符合非 sandbox 桌面应用的最小授权。见 [project](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/Easydict.xcodeproj/project.pbxproj)、[entitlements](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/Easydict/App/Easydict.entitlements)。 | 先签 Sparkle 内嵌 XPC/framework，再签 app；将 `$(PRODUCT_BUNDLE_IDENTIFIER)` 展开后再传给 `codesign`，避免更新服务 mach lookup 失效。见 [workflow 的内嵌签名段](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/.github/workflows/release-publish.yml#L196-L235)。 | hardened runtime 已启用、配置了 entitlements。发布脚本仅对本地自签名额外加入 `disable-library-validation`，所以本机可运行不等于 Developer ID 包也可加载同一原生库；应以正式身份产物做验收。 |
| 公证与 Gatekeeper | 公证 app 和 DMG，staple/validate 后 `spctl --assess`；最终验证还检查 DMG 内解包的 app。见 [package](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/scripts/release/release-package.sh)、[verify](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/scripts/release/release-verify.sh)。 | Developer ID 加 Apple 凭据时提交 DMG 到 notarytool 并 staple；失败时下载日志。见 [workflow](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/.github/workflows/release-publish.yml#L356-L396)。 | 仅在 `SNAPLINGO_NOTARIZE=1` 时公证；重签脚本会检查 `codesign --deep --strict`，但正常发布流程没有 `spctl --assess` 关卡。公开发包应把两者变成必需检查（本次不改代码）。 |
| TCC / 隐私权限连续性 | 固定 Developer ID/team，发布流程把身份作为可验证发布输入。 | 明确测试“固定自签名/Developer ID 可保留 TCC，ad-hoc 更新会丢失”；还在 CI 阻止 designated requirement 非预期变化。见 [说明](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/docs/SELF_SIGNED_CERT.md)、[本地测试](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/scripts/test-tcc-local.sh)。 | 有 `NSScreenCaptureUsageDescription`，并通过 CoreGraphics 请求屏幕录制；Accessibility 也被要求。换签名、bundle ID 或从 ad-hoc 换 Developer ID 后，macOS 会按新身份重新判定 TCC，这是预期现象而非截图实现故障。 |

## SnapLingo 的发布链路与风险

当前入口 [`script/build-release.sh`](/Users/gamilian/work/code/snaplingo/script/build-release.sh) 调用 [`release-verification.mjs`](/Users/gamilian/work/code/snaplingo/script/release-verification.mjs)：先执行 Tauri build，再运行 [`fix-macos-release-signing.mjs`](/Users/gamilian/work/code/snaplingo/script/fix-macos-release-signing.mjs)。后者会收集外部 Mach-O 依赖到 `Contents/Frameworks`、重写加载路径、重签 `.app` 和 DMG，并校验 bundle identifier、sealed resources、hardened runtime 与 designated requirement。

最可能导致“签名/权限失败”的原因，按优先级：

1. **绕过发布脚本。** [`src-tauri/tauri.conf.json`](/Users/gamilian/work/code/snaplingo/src-tauri/tauri.conf.json) 设为 `signingIdentity: "-"`；单独执行 `tauri build` 得到的是 ad-hoc 初始包。它不会获得 Developer ID/Gatekeeper 身份，也可能使升级后的 Screen Recording 授权看似“消失”。
2. **没有固定、合格的正式身份。** 当前脚本在未设置 `SNAPLINGO_CODESIGN_IDENTITY`/`MACOS_CODESIGN_IDENTITY` 时会从钥匙串找第一个可用身份，再回退到 `SnapLingo Local Code Signing`。后者只适合本机 beta，不能通过外部 Gatekeeper；前者也可能不是 `Developer ID Application`。相较 EasyDict 的预检，这是公开发行最需要补的硬约束。
3. **公证是 opt-in。** `SNAPLINGO_NOTARIZE=1` 和 `SNAPLINGO_NOTARY_PROFILE` 缺失不会阻止非公证 DMG 被构建。公开发行中，这应视为失败条件，而不是可选优化。
4. **缺少最后的 Gatekeeper 黑盒验证。** `codesign --verify` 只能确认签名结构；它不能替代从最终 DMG/ZIP 验证 `spctl`。EasyDict 的最终解包检查是可直接借用的发布门禁。
5. **Apple Events 用途说明缺口（影响选中文本，不影响截图）。** SnapLingo 通过 [`browser_applescript.rs`](/Users/gamilian/work/code/snaplingo/src-tauri/src/infrastructure/system/selection/macos/browser_applescript.rs) 与 [`context.rs`](/Users/gamilian/work/code/snaplingo/src-tauri/src/infrastructure/system/selection/macos/context.rs) 调用 `osascript` 控制浏览器/查询前台应用，但 [`Info.plist`](/Users/gamilian/work/code/snaplingo/src-tauri/Info.plist) 没有 `NSAppleEventsUsageDescription`。这很可能造成自动化隐私提示或选中文本功能被 TCC 拒绝；EasyDict 明确声明了该 key，见 [Info.plist](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/Easydict/App/Info.plist)。
6. **把 Accessibility 设为全局前置条件。** [`RequiredPermissions::all_granted`](/Users/gamilian/work/code/snaplingo/src-tauri/src/application/required_permissions.rs) 要求 Screen Recording 与 Accessibility 都已授权，而截图本身只需要前者。用户拒绝 Accessibility 时，应用操作可能整体不可用；应在验收中确认截图能否降级运行。

`com.apple.security.app-sandbox` 未出现在 SnapLingo entitlements 中，因此这是非 sandbox 分发模型；其中的 user-selected 文件访问 entitlement 不应被当作修复屏幕录制/签名问题的手段。

## 推荐的正式发布顺序

1. 在 Keychain 安装唯一的 `Developer ID Application: … (TEAMID)`；先确认 `security find-identity` 可见。创建 notarytool Keychain profile。
2. 从唯一入口构建，显式传入身份并强制公证：

   ```sh
   SNAPLINGO_CODESIGN_IDENTITY='Developer ID Application: Example (TEAMID)' \
   SNAPLINGO_NOTARIZE=1 \
   SNAPLINGO_NOTARY_PROFILE='snaplingo-notary' \
   npm run tauri:build
   ```

   脚本名以 [`script/README.md`](/Users/gamilian/work/code/snaplingo/script/README.md) 的项目命令为准；关键是不得改为裸 `tauri build`。
3. 对生成的 `.app` 和 DMG 执行下面的验收；把失败当作不发布。
4. 在干净用户环境将 DMG 中的 app 放入 `/Applications`，首次运行分别测试 Screen Recording；仅在使用智能控件/选中文本功能时再测试 Accessibility 和 Automation。更新一次同一 Developer ID 签名的包，确认原有 TCC 授权仍被识别。

## 发行诊断命令

将路径改为实际产物路径：

```sh
APP='src-tauri/target/release/bundle/macos/SnapLingo.app'
DMG='src-tauri/target/release/bundle/dmg/SnapLingo.dmg'

security find-identity -v -p codesigning
codesign -dv --verbose=4 "$APP" 2>&1
codesign -d --entitlements :- "$APP"
codesign -d -r- "$APP" 2>&1
codesign --verify --deep --strict --verbose=4 "$APP"
plutil -p "$APP/Contents/Info.plist" | rg 'CFBundleIdentifier|NSScreenCaptureUsageDescription|NSAppleEventsUsageDescription'
otool -L "$APP/Contents/MacOS/SnapLingo"
xcrun stapler validate "$DMG"
spctl --assess --type execute --verbose=4 "$APP"
```

若 notarytool 被拒绝，保留提交 ID 并取得 Apple 的具体缺陷：

```sh
xcrun notarytool log SUBMISSION_ID --keychain-profile snaplingo-notary
```

要排除旧安装/旧签名造成的 TCC 假象，可在测试机上重置后重启 app：

```sh
tccutil reset ScreenCapture com.snaplingo.app
tccutil reset Accessibility com.snaplingo.app
log stream --style compact --predicate 'subsystem == "com.apple.TCC"'
```

这两条 reset 只用于测试；不要把它们放进发布或正常运行逻辑。若 `spctl`、签名 authority 或 notary 日志失败，应先修正产物签名链；若只在 `osascript` 路径失败，再核验最终 `Info.plist` 是否含 Apple Events 使用说明及系统 Automation 授权。
