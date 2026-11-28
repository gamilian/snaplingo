# 截图选区启动优化

本轮在 v0.1.31 的透明背景和淡入修复基础上，缩短快捷键触发后的选区准备路径，并降低 macOS 冻结底图传输成本。

## 已有计时与修改

优化前本机性能日志中，前端 `window_prepared` 中位数约 54 ms，`selection_surface_ready` 约 103 ms，`revealed` 约 128 ms；这些标记相对于前端会话启动，不能作为快捷键端到端延迟。窗口准备到选区准备之间约 49 ms，与隐藏 WebView 等待 `requestAnimationFrame` 的 48 ms 超时回退相符。

- **选区准备**：移除隐藏 WebView 的帧等待，原生几何就绪后同步绘制蒙层并刷新 DOM 样式。视觉层仍在原生显示成功后淡入；底图仍等待图片解码后淡入。选区输入不等待动画或底图。
- **窗口准备**：打开窗口时完成几何、样式和透明度设置；同一有效会话的前端准备请求直接复用。异步互斥锁覆盖打开、准备、隐藏，避免前端先收到会话事件导致重复配置。隐藏会使缓存失效，取消会话不能绕过有效性检查。
- **macOS 预览 PNG**：BGRA 转 RGBA 合并为一次遍历；对 16 条完整宽度的采样行试编码。采样压缩体积小于原始体积一半时使用 Fast/Sub，否则使用无压缩 PNG，并预分配输出空间。两条路径均无损，不改变冻结像素或最终输出格式。

## 4K 像素管线基准

使用确定性的 3840 × 2160 渐变与随机噪声图，每种编码执行 7 次并取中位数。转换与编码时间包含 CGImage 数据获取、通道转换、采样（自适应路径）及 PNG 编码；解码由 Rust `image` 库执行，不代表 WebView 解码或 IPC 开销。

| 图像 | 管线 | 转换与编码 | 解码 | PNG 字节数 |
| --- | --- | ---: | ---: | ---: |
| 渐变 | 修改前无压缩 | 27.10 ms | 11.14 ms | 33,182,358 |
| 渐变 | 修改后无压缩对照 | 20.53 ms | 11.17 ms | 33,182,358 |
| 渐变 | 修改后自适应 | 12.95 ms | 17.79 ms | 1,530,091 |
| 噪声 | 修改前无压缩 | 26.63 ms | 11.06 ms | 33,182,358 |
| 噪声 | 修改后无压缩对照 | 20.77 ms | 10.63 ms | 33,182,358 |
| 噪声 | 修改后强制 Fast/Sub | 62.09 ms | 11.07 ms | 33,182,358 |
| 噪声 | 修改后自适应 | 21.41 ms | 10.92 ms | 33,182,358 |

渐变预览传输字节减少约 95.4%；压缩也会增加部分解码成本。噪声走无压缩路径，避免花约 62 ms 编码却没有体积收益。采样是启发式选择，不能保证每种真实画面都选中最快编码。

复现命令（macOS）：

```sh
cargo test -p snaplingo --release --lib benchmark_capture_pixel_pipeline -- --ignored --nocapture
```

## 回归验证

窗口准备测试通过现有 Host seam 模拟原生打开未返回时，前端已请求准备的时序。修正锁范围前，测试报 `preparation raced native open: Ok(())`；修正后，请求等待打开完成并复用准备结果。取消测试也先复现了失效会话错误复用，修正后被拒绝。

```sh
cargo test -p snaplingo --lib prepar -- --nocapture
# 4 passed
npm test -- src/views/CaptureWorkspace/captureHostRuntime.test.ts src/views/CaptureWorkspace/CaptureWorkspaceLifecycle.test.ts src/application/capture-workspace/runtime.test.ts src/styles/captureCanvas.test.ts
# 165 passed
```

前端测试覆盖无 RAF 回调时不等待计时器、几何就绪后的绘制顺序、透明背景、淡入时机和图片未解码时的选区输入。PNG 测试验证渐变压缩、随机及透明像素逐字节一致，原有行跨度测试也通过。

最终检查：

- `npm test`：132 个文件通过，1066 项通过、1 项已有跳过。
- `npm run build`：通过。
- `cargo test --workspace --locked`：678 项库测试、9 项架构测试、3 项集成测试通过；原生拖动测试通过。3 项手动探针/基准测试默认跳过。
- `cargo check --workspace --locked --all-targets`、`cargo fmt --all -- --check`、`git diff --check`：通过。

上述优化验证阶段未替换已安装应用。以上数字为旧版日志与局部基准；新构建的真实快捷键到选区耗时、连续截图视觉效果和多屏体验仍需实机验收。
