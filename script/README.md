# SnapLingo 构建脚本

这个目录包含用于构建和测试 SnapLingo 应用的脚本。

## 脚本列表

### 🧪 `npm run tauri:build:beta` - 小范围测试构建

完整的小范围测试构建流程，包括构建、打包和验证。

**使用方法：**
```bash
npm run tauri:build:beta
```

**功能：**
- 🧹 清理旧构建产物（dist/、target/release）
- 📦 版本检查（package.json 和 Cargo.toml）
- 🔨 构建前端（TypeScript + Vite）
- 🦀 构建 Tauri release profile
- ✅ 验证构建产物并显示文件大小
- ⏱️ 显示构建总耗时

**输出产物：**
- macOS: `target/release/bundle/macos/SnapLingo.app` 和 `target/release/bundle/dmg/SnapLingo_*.dmg`
- Linux: `target/release/bundle/appimage/SnapLingo_*.AppImage` 和 `target/release/bundle/deb/snaplingo_*.deb`

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

1. **Node.js** (v16+) 和 **npm**
2. **Rust** (最新稳定版)
3. **Tauri CLI**: `npm install -g @tauri-apps/cli`

### macOS 额外要求

- **Xcode Command Line Tools**: `xcode-select --install`
- **create-dmg**: `brew install create-dmg` (用于生成 DMG 安装包)

### macOS 小范围测试版

当前默认采用小范围测试分发。构建命令：

```bash
npm run tauri:build:beta
```

macOS 构建会跳过 Tauri 的 Finder AppleScript 布局步骤，避免要求 Terminal、IDE 或构建代理取得“控制 Finder”的自动化权限。签名后处理仍会生成包含 SnapLingo 和 Applications 链接的 DMG。

未配置 Developer ID 时，脚本会使用稳定的本地证书签名，以保证同一台 Mac 重建后系统权限身份保持稳定。该证书不受 Apple Gatekeeper 信任，因此测试者首次安装必须：

1. 将 SnapLingo 拖入“应用程序”。
2. 尝试打开一次，然后进入“系统设置 > 隐私与安全性”。
3. 点击“仍要打开”，输入 Mac 登录密码并确认。
4. SnapLingo 首次启动会先打开应用设置窗口；权限不足时在窗口内显示引导，依次点击“打开屏幕录制设置”和“打开辅助功能设置”完成授权。应用不会在页面显示前主动请求权限。

“仍要打开”及登录密码属于 Gatekeeper 的未知开发者放行流程；后续屏幕录制和辅助功能属于应用能力授权。此构建仅用于已知测试者，不作为公开发布包。

Provider 的 Endpoint、Base URL、API Key 和 Secret Key 统一以未加密形式保存在本机 `snaplingo.db`。正式运行时不会构造或访问系统钥匙串；遗留钥匙串记录会被忽略。Unix 平台的应用数据目录和数据库权限分别限制为 `0700` 和 `0600`。

正式发布构建会启用 hardened runtime。设置 `SNAPLINGO_NOTARIZE=1` 后，构建脚本还会提交 DMG 公证并附加票据。凭据可使用以下任一方式：

```bash
# 推荐：预先通过 xcrun notarytool store-credentials 创建
SNAPLINGO_NOTARIZE=1 \
SNAPLINGO_NOTARY_PROFILE=SnapLingoNotary \
SNAPLINGO_CODESIGN_IDENTITY="Developer ID Application: Example (TEAMID)" \
npm run tauri:build

# 或使用环境变量
SNAPLINGO_NOTARIZE=1 \
APPLE_ID="release@example.com" \
APPLE_PASSWORD="app-specific-password" \
APPLE_TEAM_ID="TEAMID" \
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

### 问题：版本号不一致警告
**说明：** `package.json` 和 `src-tauri/Cargo.toml` 中的版本号不一致，构建仍会继续，但建议同步版本号。

### 问题：构建失败，出现 Rust 编译错误
**解决：**
1. 确保 Rust 是最新稳定版：`rustup update`
2. 清理缓存后重试：`cargo clean && npm run tauri:build:beta`

---

## 平台支持

- ✅ macOS (Apple Silicon & Intel)
- ✅ Linux (Ubuntu/Debian/Fedora 等)
- ⚠️ Windows: 脚本需要在 Git Bash、WSL 或 MinGW 环境中运行

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
