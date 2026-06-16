# 构建和测试脚本设计

**日期：** 2026-06-16
**状态：** 已批准

## 概述

为 SnapLingo 项目添加 shell 脚本，简化 release 构建和开发测试流程。

## 需求

1. 编译 release app 的脚本，包含完整流程（清理、版本检查、构建、打包、验证）
2. 运行测试 app 的脚本，支持开发模式并自动打开开发者工具
3. 支持 macOS 和 Linux 平台

## 设计方案

采用简单的两个独立脚本方案：

### 1. `script/build-release.sh` - Release 构建脚本

**功能流程：**

1. **清理旧构建产物**
   - 删除 `dist/` 目录
   - 删除 `src-tauri/target/release` 目录

2. **版本检查**
   - 读取 `package.json` 中的版本号
   - 读取 `src-tauri/Cargo.toml` 中的版本号
   - 显示版本信息
   - 检查版本是否一致，如不一致发出警告

3. **构建前端**
   - 运行 `npm run build`
   - 构建 React + Vite 前端到 `dist/` 目录

4. **构建 Tauri Release**
   - 运行 `npm run tauri build`
   - 生成平台特定的安装包

5. **验证构建产物**
   - 检查 `src-tauri/target/release/bundle/` 目录
   - 在 macOS 上验证 `.app` 和 `.dmg` 文件
   - 在 Linux 上验证 `.AppImage` 或 `.deb` 文件

6. **输出构建信息**
   - 显示构建产物的路径
   - 显示文件大小
   - 显示构建总耗时

**错误处理：**
- 使用 `set -e` 确保任何命令失败时立即退出
- 在每个关键步骤显示状态信息
- 如果依赖未安装（node_modules），提示并退出

### 2. `script/dev.sh` - 开发测试脚本

**功能流程：**

1. **环境检查**
   - 确认 `node_modules/` 目录存在
   - 如不存在，提示运行 `npm install`

2. **设置开发环境**
   - 设置 `RUST_BACKTRACE=1` 环境变量（便于调试）
   - 显示启动信息

3. **启动开发模式**
   - 运行 `npm run tauri:dev`
   - Tauri 会自动：
     - 启动 Vite 开发服务器（热重载）
     - 编译 Rust 后端（开发模式）
     - 启动应用窗口

4. **开发者工具提示**
   - 显示如何打开开发者工具的快捷键
   - macOS: `Cmd+Option+I`
   - Linux: `Ctrl+Shift+I`

**特性：**
- 支持前端热重载
- 自动重启 Rust 后端（检测到代码变更时）
- 显示控制台日志和错误

## 技术细节

### 脚本格式

- Shell 脚本（兼容 bash/zsh）
- 使用 `#!/usr/bin/env bash` shebang
- 设置 `set -e` 严格错误处理
- 使用彩色输出增强可读性

### 文件结构

```
script/
├── build-release.sh   # Release 构建脚本
└── dev.sh             # 开发测试脚本
```

### 权限

两个脚本都需要可执行权限：
```bash
chmod +x script/build-release.sh
chmod +x script/dev.sh
```

## 使用方式

### 构建 Release 版本

```bash
./script/build-release.sh
```

输出示例：
```
🧹 清理旧构建产物...
📦 版本检查...
  package.json: 0.1.0
  Cargo.toml: 0.1.0
🔨 构建前端...
🦀 构建 Tauri Release...
✅ 构建完成！
  产物位置: src-tauri/target/release/bundle/
  总耗时: 2m 34s
```

### 启动开发模式

```bash
./script/dev.sh
```

输出示例：
```
🚀 启动 SnapLingo 开发模式...
💡 提示：按 Cmd+Option+I 打开开发者工具
[Vite] 开发服务器运行在 http://localhost:5173
[Tauri] 正在编译 Rust 后端...
[Tauri] 应用已启动
```

## 实现注意事项

1. **版本号提取**
   - 使用 `grep` + `sed` 从 JSON 和 TOML 文件中提取版本号
   - 处理不同的格式和空白字符

2. **平台检测**
   - 使用 `uname` 检测操作系统
   - 根据平台调整验证逻辑和提示信息

3. **彩色输出**
   - 使用 ANSI 颜色码（绿色 ✅、红色 ❌、黄色 ⚠️）
   - 确保在不支持颜色的终端中也能正常工作

4. **构建产物路径**
   - macOS: `src-tauri/target/release/bundle/macos/SnapLingo.app`
   - macOS DMG: `src-tauri/target/release/bundle/dmg/SnapLingo_*.dmg`
   - Linux AppImage: `src-tauri/target/release/bundle/appimage/SnapLingo_*.AppImage`
   - Linux DEB: `src-tauri/target/release/bundle/deb/snaplingo_*.deb`

## 未来扩展

如果需要，可以添加：
- `script/test.sh` - 运行 Rust 单元测试和集成测试
- `script/clean.sh` - 深度清理所有构建产物和缓存
- `script/check.sh` - 运行代码检查（cargo clippy, eslint）
- 构建参数支持（如 `--target` 指定平台）
- CI/CD 集成

## 成功标准

1. ✅ `build-release.sh` 能成功构建并验证 release 产物
2. ✅ `dev.sh` 能启动开发模式并显示开发者工具提示
3. ✅ 脚本在 macOS 和 Linux 上都能正常工作
4. ✅ 错误处理清晰，失败时有明确的错误信息
5. ✅ 输出信息友好，易于理解
