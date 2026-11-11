# small / MNN CPU 切换与性能实测

2026-11-11，在 **Apple M1 Pro、8 核 CPU、16 GiB 内存、macOS 14.8.3** 上测试。macOS、Windows、Linux 的本地 PaddleOCR 统一为 **PP-OCRv6 small / MNN CPU**，使用相同的固定版本 MNN 权重、配套字典和 CPU 配置。每次识别启动工作进程，完成后退出释放模型内存。下列耗时、内存及 CPU 数字仅为本机 Mac 实测。

## 最终版本结果

最终签名版本使用 12 张合成中英文截图，覆盖 6 种尺寸和明暗主题。每种输入、每个模型记录首次请求，再重复 10 次，共 **264 次成功请求**。表中耗时为重复请求中位数，范围表示两种主题的结果；内存列是该尺寸所有请求的最高工作进程 RSS。

| 输入 | tiny 中位耗时 | small 中位耗时 | small 最高峰值 RSS | small 请求期间平均 CPU |
| --- | ---: | ---: | ---: | ---: |
| 小选区，512×100，3 行 | 94–95 ms | 181–183 ms | 235 MB | 199–202% |
| 紧凑截图，960×320，10 行 | 157–167 ms | 402–415 ms | 317 MB | 299–304% |
| 文档，1280×960，28 行 | 330–334 ms | 1,008–1,042 ms | 438 MB | 336–342% |
| 密集文本，1920×1080，56 行 | 515–516 ms | 1,778–1,856 ms | 456 MB | 351–354% |
| 4K，3840×2160，129 行 | 1,027–1,080 ms | 3,877–3,936 ms | 584 MB | 357–359% |
| 横向选区，2400×180，4 行 | 105–114 ms | 221 ms | 287 MB | 224% |

small 比 tiny 慢约 **1.9–3.8 倍**。small 小选区首次进程请求为 **550 ms**；首次值包含程序执行、模型读取和系统缓存状态等成本，未清空文件缓存，因此不作为冷机启动结果。整轮 small 工作进程最高 RSS 为 **583.6 MB**。

CPU 以 **100% = 一个核心**计算，来自每个子进程 user+system CPU 时间除以请求墙钟时间；约 200% 表示平均使用 2 个核心，约 359% 相当于本机 8 核总容量的 45%。这是请求期间平均值，不是瞬时峰值。设置 4 个推理线程；图像和文字行并行处理也计入实际 CPU 时间。

## 空闲占用与安装验证

已备份并替换 `/Applications/SnapLingo.app`，重启后原有 PaddleOCR 选择继续生效。安装后可执行文件的 SHA-256 与本轮测速版本相同，签名身份与旧版相同。

重启 8 秒后测量 10 秒空闲窗口，再通过已安装程序运行一次 1280×960 文档 OCR，最后测量另一个 10 秒窗口：

- 空闲主进程 RSS 起初为 39.8–43.2 MB；OCR 工作进程退出后为 **39.8 MB**，未保留模型会话。
- 主进程 physical footprint 为 **13.0 MB**，与 RSS 口径不同。
- 两个空闲窗口平均 CPU 为 **0.00974% / 0.00000% 单核**，约等于空闲零占用；这是本轮短窗口观测。
- 已安装工作进程文档识别耗时 **1163 ms**，峰值 RSS **365.2 MB**，成功返回 28 行文本并退出。

主进程测量只覆盖桌面进程；工作进程峰值包含输入图片、模型、运行库和前后处理分配。工作进程退出后其地址空间由系统回收，主应用、UI 和系统文件缓存另计。主进程 CPU 使用 `proc_pid_rusage` 的 Mach ticks 经 `mach_timebase_info` 转换为纳秒，本机 timebase 为 125/3；与 `ps` 的累计 CPU 时间交叉核对一致。

## 文件大小

1 MB = 1,000,000 bytes。下列是文件字节之和，不等于文件系统分配块、压缩安装包或运行内存。

| 资源 | tiny | small |
| --- | ---: | ---: |
| 检测权重 | 0.902 MB | 4.965 MB |
| 识别权重 | 2.252 MB | 10.647 MB |
| 字典 | 0.027 MB | 0.075 MB |
| 模型＋字典合计 | **3.181 MB** | **15.687 MB** |
| 完整签名 Mac 应用包 | **58.587 MB** | **70.239 MB** |

所有平台的模型＋字典均为 **15,686,931 字节**，使用同一组 FP16 存储的 MNN 权重和同一字典，运行精度为默认 Normal。MNN 原生代码按目标平台构建并静态链接，可执行文件大小按平台有所不同；Windows 另外保留 Visual C++ 运行库。所有平台均移除 ORT/DirectML 资源。模型文件增加约 12.51 MB，完整 Mac 应用包增加约 11.65 MB。

## 方法与验证范围

- 固定 `ocr-rs` 提交 `664a945953e46186b37203fa4902a20d60a32809`，MNN 3.6.1，CPU 后端，4 个推理线程，parallel=true，recognition batch=8；每次请求均为新工作进程。
- 使用仓库已有的合成截图，逐图校验 SHA-256。交替模型顺序、串行测量，使用相同本地证书签名的真实应用可执行文件；没有在测速时编译代码。
- 耗时为工作进程启动至退出，包含图片解码、模型加载、预处理、推理、后处理、结果写入和退出；不包含截图 UI、Tauri IPC、调用方输入暂存或翻译。
- 使用 `os.wait4` 获取每个工作进程 CPU 时间和 `ru_maxrss`，macOS 的 `ru_maxrss` 单位为字节。每组 10 次重复，nearest-rank p95 等于该组最大值，不作为长期尾延迟估计。
- Rust 库测试 **654 项通过**；界面与资源/便携包/CRT 测试 **21 项通过**；small MNN CPU 实模型冒烟测试单独运行通过。TypeScript、Vite 生产构建、Tauri release 打包和严格签名验证通过。
- Windows/Linux 资源准备已实际运行，模型和字典 SHA-256 与 macOS 一致。原生程序尚未在这两个平台真机运行；CI 已加入对应构建、直接模型识别及最终工作进程验证，当前不提供其耗时或内存数字。
- 所有请求验证非空文本和文字框，保存输出文本；没有字符准确率金标准，不将本轮成功返回解释为准确率验收。

原始记录：[机器、模型、可执行文件哈希](2026-11-11-small-ocr-benchmark/metadata.json)、[输入清单](2026-11-11-small-ocr-benchmark/images-manifest.jsonl)、[逐次测量及输出](2026-11-11-small-ocr-benchmark/requests.jsonl)、[完整统计](2026-11-11-small-ocr-benchmark/summary.json)、[安装和空闲测量](2026-11-11-small-ocr-benchmark/installed-app.json)。

复现入口：[worker_bench.py](../../script/ocr-benchmark/worker_bench.py)。

```sh
python3 script/ocr-benchmark/worker_bench.py \
  --small-binary target/small-ocr/macos/SnapLingo.app/Contents/MacOS/snaplingo \
  --small-models target/small-ocr/macos/SnapLingo.app/Contents/Resources/ocr \
  --tiny-binary /path/to/tiny/SnapLingo.app/Contents/MacOS/snaplingo \
  --tiny-models /path/to/tiny/SnapLingo.app/Contents/Resources/ocr \
  --images docs/research/2026-11-11-small-ocr-benchmark/images-manifest.jsonl \
  --output /tmp/snaplingo-small-ocr-results --repeats 10
```
