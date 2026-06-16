# SnapLingo 构建脚本

这个目录包含用于构建和测试 SnapLingo 应用的脚本。

## 脚本列表

### 🏗️ `build-release.sh` - Release 构建脚本

完整的 release 构建流程，包括清理、构建、打包和验证。

**使用方法：**
```bash
./script/build-release.sh
```

**功能：**
- 🧹 清理旧构建产物（dist/、target/release）
- 📦 版本检查（package.json 和 Cargo.toml）
- 🔨 构建前端（TypeScript + Vite）
- 🦀 构建 Tauri Release
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
2. 清理缓存后重试：`cargo clean && ./script/build-release.sh`

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

### 发布新版本
```bash
# 1. 更新版本号
# 编辑 package.json 和 src-tauri/Cargo.toml

# 2. 构建 release 版本
./script/build-release.sh

# 3. 测试构建产物
open target/release/bundle/macos/SnapLingo.app  # macOS
# 或
./target/release/bundle/appimage/SnapLingo_*.AppImage  # Linux
```

---

## 贡献

如果你发现脚本有问题或想要改进，请提交 Issue 或 Pull Request。
