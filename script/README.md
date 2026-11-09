# 开发与发布脚本

默认入口运行 Rust 原生应用。React/Vite 只用于隔离视觉基准和回归测试，原生安装包不包含前端资源。

## 常用命令

| 命令 | 用途 |
| --- | --- |
| `npm run dev` / `npm run native:dev` | 准备本机 OCR、构建并运行原生 debug 可执行文件 |
| `npm run native:run` | 运行已构建的原生 debug 可执行文件 |
| `npm run build` / `npm run native:build` | 构建平台原生应用并生成安装包 |
| `npm run native:bundle` | 使用已有 release 二进制重新包装 |
| `npm run release:build` | 原生构建后完整验证最终发布产物 |
| `npm run release:verify` | 重新验证已有发布产物 |
| `npm run release:collect` | 验证并收集标准文件名、SHA-256 清单和构建来源 |
| `npm run reference:dev` | 启动原 React 视觉基准 Vite 服务 |
| `npm run reference:build` | 校验许可清单、TypeScript 并编译基准页面 |
| `npm run reference:preview` | 预览已编译基准页面 |
| `npm test` | 业务、UI、架构及打包脚本回归测试 |

使用 Node.js 22、稳定 Rust 和 `npm ci`。macOS 需要 Xcode Command Line Tools；Windows 使用 x64 MSVC 与 Visual Studio C++ Build Tools，打包还需要 NSIS 和 WiX 3。完整原生应用暂不支持 Linux。

## 原生构建

```bash
npm run native:build
npm run release:verify
```

构建准备并校验目标平台 OCR 资源，拒绝缺少模型、空文件或架构不匹配。Windows staged 树包含 ONNX Runtime、DirectML、Visual C++ DLL、字体和第三方许可；安装器与 ZIP 使用同一棵树，无需 WebView2。

默认产物：

- macOS：`target/native/release/SnapLingo.app`、`target/native/release/bundle/dmg/snaplingo-vX.Y.Z-macos-{aarch64,x86_64}.dmg`。
- Windows：`target/native/release/app/`、`target/native/release/bundle/nsis/SnapLingo_X.Y.Z_x64-setup.exe`、`bundle/msi/SnapLingo_X.Y.Z_x64.msi`、`bundle/portable/SnapLingo_X.Y.Z_x64-portable.zip`。

`CARGO_TARGET_DIR` 可替换 `target`。Windows 打包资源独立保存在 `target/native/resources/win32-x64/`，不会覆盖其他平台资源。`npm run ocr:prepare` 为本机开发与测试准备源码树资源，也接受显式 `-- --target <Rust target>`。

macOS 正式发布复用固定自签名证书；CI 的临时验证显式使用 ad-hoc：

```bash
SNAPLINGO_SIGNING_MODE=adhoc npm run native:build -- --debug
```

debug 产物使用 `target/native/debug/`。开发中的裸 debug 可执行文件适合快速迭代；需要验证签名、安装位置和权限身份时使用 app bundle。签名脚本使用 `desktop.json` 的稳定应用标识和项目版本，验证 hardened runtime、架构、内嵌动态库、最低系统版本和 DMG。不会请求 Finder 自动化权限。

## 发布验证

```bash
node script/release-verification.mjs preflight
npm run release:build
npm run release:collect
```

发布前检查 `package.json`、`package-lock.json` 的根版本、`crates/app/Cargo.toml`、`Cargo.lock` 一致；tag 必须为相同版本的 `vX.Y.Z`。`desktop.json` 保存平台身份配置，不重复保存版本。

普通发布构建只移除原生 staging 和 bundle，保留 Cargo 缓存。需要重建 release 编译输出时使用 `npm run release:build -- --clean`，或执行 `./script/build-release.sh`。

`release:verify` 要求当前版本全部产物存在且非空。macOS 额外验证 app/DMG 签名、固定证书、动态库、最低系统版本，并挂载最终 DMG 检查 app 身份和 Applications 链接。Windows 复验 staged PE 架构和所有必需 OCR/CRT/许可资源。安装 smoke 在短生命周期 Actions runner 上运行：

```bash
node script/smoke-test-macos.mjs
```

```powershell
./script/smoke-test-windows.ps1 -InstallerType nsis
./script/smoke-test-windows.ps1 -InstallerType msi
./script/smoke-test-windows.ps1 -InstallerType portable
```

这些测试覆盖实际安装、启动、重装、卸载及用户数据保留；不替代真实桌面截图/OCR/权限和升级交互验收。完整矩阵和证书配置见 [发布指南](../docs/RELEASING.md)。

收集后的 `release-assets/` 使用更新检查器约定的稳定文件名：`snaplingo-vX.Y.Z-macos-架构.dmg` 和 `snaplingo-vX.Y.Z-windows-x86_64{.msi,-setup.exe,-portable.zip}`。SHA-256 基于实际发布字节，构建来源 JSON 保留提交、架构及签名模式。不会生成 WebView2 离线变体。

## 视觉基准与许可

`npm run reference:dev` 打开已有 Settings fixture；查询参数可选择页面、主题和场景。`surface=result` 和 `surface=permission` 打开对应原 React 视图。收集器 HTML 和 fixtures 统一位于 `reference/web/fixtures`，详见 [参考页面说明](../reference/web/README.md)。它们不读已安装应用设置，也不发送桌面 IPC。

依赖或 OCR 资源清单变更后运行 `npm run licenses:generate`，更新 `assets/licenses/thirdPartyNotices.json`。原生构建和 reference 构建都会检查它与锁文件一致。安装包、便携 ZIP 和关于页使用相同许可清单。
