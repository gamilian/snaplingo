# SnapLingo 构建脚本

这个目录包含用于构建和测试 SnapLingo 应用的脚本。

## 脚本列表

### `npm run tauri:build` - 桌面发布构建

跨平台发布入口，负责版本检查、Tauri bundle 构建和产物验证。CI 与本地构建调用同一个 implementation。

**使用方法：**
```bash
npm run tauri:build
```

**功能：**
- 要求 `package.json`、`Cargo.toml` 和 `tauri.conf.json` 版本一致
- 通过 Cargo metadata 发现 workspace target 目录
- 构建前端和 Tauri release bundle
- 验证当前平台要求的全部产物及其可运行形态

**输出产物：**
- macOS: `target/release/bundle/macos/SnapLingo.app` 和 `target/release/bundle/dmg/SnapLingo_*.dmg`
- Linux: `target/release/bundle/appimage/SnapLingo_*.AppImage` 和 `target/release/bundle/deb/snaplingo_*.deb`
- Windows: `target/release/bundle/msi/*.msi` 和 `target/release/bundle/nsis/*.exe`

需要清理旧的前端与 release 产物时使用：

```bash
npm run tauri:build:beta
```

只验证已有 bundle、不重新构建时使用：

```bash
npm run release:verify
```

**构建时间：** 约 2-3 分钟（取决于机器性能）

---

### 🚀 `dev.sh` - 开发测试脚本

启动开发模式，支持前端热重载和 Rust 自动重启。

**使用方法：**
```bash
./script/dev.sh
```

**功能：**
- ✅ 环境检查（node_modules）
- 🔧 设置 `RUST_BACKTRACE=1` 便于调试
- 🚀 启动 Tauri 开发模式
- 💡 显示开发者工具快捷键提示
  - macOS: `Cmd+Option+I`
  - Linux: `Ctrl+Shift+I`

**特性：**
- 前端热重载（修改 React/TypeScript 代码自动刷新）
- Rust 自动重启（修改 Rust 代码自动重新编译并重启）
- 完整的控制台日志输出

---

## 前置要求

### 必须安装的工具

1. **Node.js** 22 和 **npm**
2. **Rust** (最新稳定版)
3. 项目本地安装的 Tauri CLI（由 `npm ci` 或 `npm install` 提供）

### macOS 额外要求

- **Xcode Command Line Tools**: `xcode-select --install`
- **create-dmg**: `brew install create-dmg` (用于生成 DMG 安装包)

### macOS 小范围测试版

当前默认采用小范围测试分发。构建命令：

```bash
npm run tauri:build:beta
```

macOS 构建会跳过 Tauri 的 Finder AppleScript 布局步骤，避免要求 Terminal、IDE 或构建代理取得“控制 Finder”的自动化权限。签名后处理仍会生成包含 SnapLingo 和 Applications 链接的 DMG。

未显式配置 `SNAPLINGO_CODESIGN_IDENTITY` 时，脚本会创建或复用专用的 `SnapLingo Local Code Signing` 自签名证书；它不会自动改用钥匙串中的其他证书。将这张证书及私钥安全备份并在后续构建中持续复用，同时保持 `com.snaplingo.app` 不变。这样可避免 ad-hoc 签名每次构建都改变身份；TCC 是否跨更新保留仍必须在真实升级路径中测试，Apple 未对此提供保证。

该证书不受 Apple Gatekeeper 信任，因此测试者首次安装必须：

1. 将 SnapLingo 拖入“应用程序”。
2. 尝试打开一次，然后进入“系统设置 > 隐私与安全性”。
3. 点击“仍要打开”，输入 Mac 登录密码并确认。
4. SnapLingo 首次启动会先打开应用设置窗口；权限不足时在窗口内显示引导，依次点击“打开屏幕录制设置”和“打开辅助功能设置”完成授权。应用不会在页面显示前主动请求权限。

“仍要打开”及登录密码属于 Gatekeeper 的未知开发者放行流程；后续屏幕录制和辅助功能属于应用能力授权。此构建仅用于已知测试者，不作为“直接双击即受信任”的公开发布包。不要指导用户关闭 Gatekeeper、运行 `xattr -cr`，或尝试由安装包修改 TCC 数据库。

Provider 的 Endpoint、Base URL、API Key 和 Secret Key 统一以未加密形式保存在本机 `snaplingo.db`。正式运行时不会构造或访问系统钥匙串；遗留钥匙串记录会被忽略。Unix 平台的应用数据目录和数据库权限分别限制为 `0700` 和 `0600`。

正式发布构建会启用 hardened runtime。设置 `SNAPLINGO_NOTARIZE=1` 后，构建脚本还会提交 DMG 公证并附加票据。公证必须使用 `notarytool` Keychain profile，避免 Apple ID 密码进入进程参数或构建日志：

```bash
# 预先通过 xcrun notarytool store-credentials 创建
SNAPLINGO_NOTARIZE=1 \
SNAPLINGO_NOTARY_PROFILE=SnapLingoNotary \
SNAPLINGO_CODESIGN_IDENTITY="Developer ID Application: Example (TEAMID)" \
npm run tauri:build
```

### Linux 额外要求

请参考 [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites) 安装必要的系统依赖。

---

## 首次使用

在运行脚本之前，确保安装了所有依赖：

```bash
# 安装 Node.js 依赖
npm install

# 验证 Rust 环境
rustc --version
cargo --version
```

---

## 故障排除

### 问题：`node_modules not found`
**解决：** 运行 `npm install`

### 问题：DMG 打包失败 (macOS)
**解决：** 安装 create-dmg 工具
```bash
brew install create-dmg
```

### 问题：版本号不一致
**解决：** 同步 `package.json`、`src-tauri/Cargo.toml` 和 `src-tauri/tauri.conf.json` 的版本。发布入口会在构建前失败，避免生成版本不明确的产物。

### 问题：构建失败，出现 Rust 编译错误
**解决：**
1. 确保 Rust 是最新稳定版：`rustup update`
2. 清理缓存后重试：`cargo clean && npm run tauri:build:beta`

---

## 平台支持

- macOS (Apple Silicon & Intel)
- Linux (Ubuntu/Debian/Fedora 等)
- Windows（npm 入口原生支持 PowerShell 和 cmd）

---

## 开发工作流建议

### 日常开发
```bash
./script/dev.sh
```

### 生成小范围测试包
```bash
# 1. 更新版本号
# 编辑 package.json 和 src-tauri/Cargo.toml

# 2. 构建 Beta 测试版本
npm run tauri:build:beta

# 3. 测试构建产物
open target/release/bundle/macos/SnapLingo.app  # macOS
# 或
./target/release/bundle/appimage/SnapLingo_*.AppImage  # Linux
```

---

## 贡献

如果你发现脚本有问题或想要改进，请提交 Issue 或 Pull Request。
