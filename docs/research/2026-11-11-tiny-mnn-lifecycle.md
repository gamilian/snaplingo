# PP-OCRv6 tiny：MNN CPU 会话生命周期实测

2026-11-11，在 Apple M1 Pro、16 GiB 内存、macOS 14.8.3 上测量。目的为决定本地 OCR 的会话和进程生命周期，不是识别准确率评测。没有使用用户截图、历史或识别文本。

## 实验依据和口径

- `ocr-rs` 固定提交 [`664a945953e46186b37203fa4902a20d60a32809`](https://github.com/zibo-chen/ocr-rs/tree/664a945953e46186b37203fa4902a20d60a32809)，MNN runtime 0.1.0 / MNN 3.6.1，CPU backend；rec batch size 为 8。
- tiny 检测权重 901,896 字节，识别权重 2,251,616 字节，字典 27,156 字节，共 **3,180,668 字节**。逐文件校验仓库[固定权重清单](../../script/ocr-benchmark/mnn/weights-manifest.json)，本次[校验结果](2026-11-11-tiny-mnn-lifecycle/models.json)记录完整 SHA-256。
- 小图为 512×100、3 行中英混合文本；文档为 1280×960、28 行。由[现有生成器](../../script/ocr-benchmark/generate_dataset.py)使用系统 Hiragino Sans GB 字体生成。实际使用的两张图片及哈希保存在[输入目录](2026-11-11-tiny-mnn-lifecycle/images/)和[清单](2026-11-11-tiny-mnn-lifecycle/images-manifest.jsonl)。文档是规则合成排版，不代表复杂自然截图。
- 各进程串行执行。未清理操作系统文件缓存，“冷”指新进程或新引擎会话，不声称磁盘缓存完全冷。
- `load_ms` 包含模型读取、会话创建；`recognize_ms` 包含 OCR 预处理、推理、后处理。`request_ms` 是两者合计，不含图片解码、UI、Tauri IPC。输入图片在基线 RSS 测量前解码并保留。
- `process_total_ms` 从父进程启动子进程直到退出，包含解码、`ps` 采样、模型创建、识别、显式 drop 和退出。它不是生产应用最终端到端延迟。
- 当前 RSS 来自 `ps -o rss`，单位转换为字节；峰值来自 `getrusage(RUSAGE_SELF).ru_maxrss`（macOS 字节）。不会把峰值当成当前 RSS。空闲窗口为 2 秒，以进程 user+system CPU 时间增量计算单核百分比。
- 下述数据来自独立 Cargo harness 成功编译后的完整重跑。runner 每次检查子进程退出码，并核对回显的 `threads` 和 `parallel` 确实等于请求参数，避免执行旧 binary 混淆配置。

## 缓存、每次销毁、独立进程

本组都使用 4 线程、`with_parallel(true)`。同进程模式每组 5 次，后续列是第 2～5 次的中位数；独立进程为 10 次的中位数。

| 输入 | 首次加载 / 识别 / 合计 | 缓存会话后续识别 | 同进程每次 new/drop 的后续请求 | 独立进程启动至退出 |
| --- | ---: | ---: | ---: | ---: |
| 512×100 小图 | 42.65 / 32.53 / **75.18 ms** | **26.35 ms** | **31.32 ms** | **78.55 ms** |
| 1280×960 文档 | 41.10 / 259.75 / **300.85 ms** | **244.88 ms** | **262.50 ms** | **314.28 ms** |

同进程重新创建的后续 `load_ms` 约 2 ms，但这不代表引擎真正从零启动：进程、文件缓存和运行库的全局状态已经存在。独立进程模型加载中位数约 33 ms。独立进程总耗时范围分别为小图 **72.15～85.67 ms**、文档 **307.00～328.79 ms**；这十次样本不能证明首次安装后的启动上界或所有硬件的时延。

内存单位为十进制 MB：

| 输入及策略 | 解码后基线 RSS | 识别后缓存 / 峰值 | 显式 drop 再等 2 秒后的 RSS |
| --- | ---: | ---: | ---: |
| 小图，缓存会话 5 次 | 5.49 MB | 94.49 MB | **94.44 MB** |
| 小图，每次 new/drop 5 次 | 5.49 MB | 147.32 MB 峰值 | **147.28 MB** |
| 文档，缓存会话 5 次 | 9.40 MB | 254.25 MB | **254.18 MB** |
| 文档，每次 new/drop 5 次 | 9.03 MB | 289.59 MB 峰值 | **289.52 MB** |

**在该实现中，drop 会话并不能让 RSS 回到基线。** 连续 new/drop 也没有改善常驻内存，反而更高。这说明进程内仍保留了已分配内存；本实验没有进一步把它归因到具体的 allocator、MNN 或 Rayon 内部缓存，也不能据此断言内存泄漏。

缓存会话和 drop 后的空闲 CPU 都低于 **0.004% 单核**。因此本次要解决的主要问题是常驻内存，不是后台持续占用 CPU。独立工作进程退出后，工作进程地址空间由操作系统回收；主应用和文件缓存的占用不属于该“归零”表述。

## `parallel` 与线程数对比

另外以独立进程模式，每配置、每图片各 5 次。所有数字是中位数，峰值为每进程实际 `getrusage` 峰值：

| 配置 | 小图总耗时 | 小图峰值 RSS | 文档总耗时 | 文档峰值 RSS |
| --- | ---: | ---: | ---: | ---: |
| 4 线程，parallel=false | 78.29 ms | 85.25 MB | 371.58 ms | 371.46 MB |
| 2 线程，parallel=false | 89.69 ms | 84.69 MB | 490.69 ms | 352.67 MB |
| **4 线程，parallel=true** | **78.64 ms** | **84.02 MB** | **321.84 ms** | **229.62 MB** |

这批数据没有证明关闭 parallel 更快或峰值更低。文档上 parallel=true 的时间和峰值都更小，小图差距很小。对于一次请求后退出的工作进程，推荐先保持 **4 线程、parallel=true、batch=8**：进程退出已经提供资源回收，不需要再为了同进程全局线程池存留而关闭并行。没有随机化运行顺序，样本量也小，因此不把差异推广为全部图片和设备的结论。

所有识别只检查成功返回非空结果，并记录行数和字符数；**没有计算准确率、漏字率或把 tiny 与 medium 的质量排序**。性能结论必须与独立的识别质量评估一起使用。

## 签名后的实际应用工作进程

最终优化构建的 Tauri 应用完成持久本地证书签名及严格校验后，使用相同两张图片各启动 5 个新工作进程，串行测量。原始记录、二进制 SHA-256 和包体积见 [signed-app-worker.json](2026-11-11-tiny-mnn-lifecycle/signed-app-worker.json)。未预热，也未清理文件缓存。

| 输入 | 启动至退出中位数 | 范围 | 工作进程峰值 RSS 中位数 |
| --- | ---: | ---: | ---: |
| 512×100 小图 | **81.96 ms** | 80.73～989.91 ms | 95.78 MB |
| 1280×960 文档 | **313.99 ms** | 306.88～319.59 ms | 255.51 MB |

小图第一次运行是 **989.91 ms**，后续四次为 80.73～82.35 ms；保留首次样本，不把中位数当作冷启动保证。未进一步拆分首次额外开销的来源。计时包含 `/usr/bin/time` 启动、实际应用进程启动、图片读取解码、模型加载、识别、结果 JSON 写入及退出；不包含截图、父进程写入图片、UI 展示和在线翻译。峰值是工作进程本身，不含桌面主进程。

实际调用入口如下，可替换图片并重复运行；上述总耗时使用父进程 `time.perf_counter()` 包围此命令测量，峰值取 macOS `/usr/bin/time -l` 的 `maximum resident set size`（字节）：

```sh
tiny_app="$PWD/target/release/bundle/macos/SnapLingo.app"
tiny_result=$(mktemp /tmp/snaplingo-tiny-result.XXXXXX)
/usr/bin/time -l "$tiny_app/Contents/MacOS/snaplingo" \
  --ocr-tiny-worker "$tiny_app/Contents/Resources/ocr" \
  "$PWD/docs/research/2026-11-11-tiny-mnn-lifecycle/images/small-light.png" \
  "$tiny_result"
cat "$tiny_result"
rm "$tiny_result"
```

全部 10 次返回非空文本和有效的归一化行坐标，退出码为 0，测量结束无残留 OCR 工作进程。损坏图片、缺失模型和参数不全分别验证了错误响应及非零退出。复制到 `/Applications/SnapLingo.app` 后再次通过签名校验、真实 OCR 请求和普通应用启动验证。

本次签名 `.app` 的文件逻辑大小为 **58,586,580 字节**，OCR 资源含许可为 **3,234,085 字节**，扣除 OCR 为 **55,352,495 字节**。这是未压缩应用文件大小，不是 DMG 下载大小或内存占用。

## 可重复运行

研究入口：[Cargo.toml](../../script/ocr-benchmark/tiny-lifecycle/Cargo.toml)、[harness](../../script/ocr-benchmark/tiny-lifecycle/src/main.rs)、[串行 runner](../../script/ocr-benchmark/tiny-lifecycle/run.py)。独立 workspace 不参与产品构建，锁文件固定本次依赖解析。harness 的 RSS 和 rusage 单位仅针对 macOS。

以下命令从仓库根目录运行。`tiny_models` 指向包含上述三个已校验文件的目录；可复用 Cargo 的固定 ocr-rs checkout 的 `models/`，或按权重清单中的固定 URL 下载这三个文件。runner 在执行前校验所有文件哈希。不要使用用户截图作为 `--images` 输入。

```sh
tiny_work=$(mktemp -d /tmp/snaplingo-tiny-lifecycle.XXXXXX)
tiny_models=/path/to/verified/tiny/models
MNN_REQUIRE_PREBUILT=1 CARGO_TARGET_DIR="$tiny_work/target" \
  cargo build --locked --release \
  --manifest-path script/ocr-benchmark/tiny-lifecycle/Cargo.toml
python3 script/ocr-benchmark/tiny-lifecycle/run.py \
  --binary "$tiny_work/target/release/snaplingo-tiny-mnn-lifecycle" \
  --models "$tiny_models" \
  --images docs/research/2026-11-11-tiny-mnn-lifecycle/images \
  --output "$tiny_work/results"
```

本机实际构建使用已有 Cargo target 和已校验 MNN 预编译缓存，命令成功完成；随后运行仓库 runner，所有进程退出码为 0。初次干净构建仍需下载锁定依赖和 MNN 预编译包。重新生成图片可运行 `python3 script/ocr-benchmark/generate_dataset.py --output <目录>`（需要 Pillow 和指定系统字体）；精确复现本次输入应使用上面保存的 PNG，避免系统字体版本差异。

## 证据文件

- [汇总与机器/二进制来源](2026-11-11-tiny-mnn-lifecycle/summary.json)
- [小图缓存](2026-11-11-tiny-mnn-lifecycle/small-light-cached.jsonl)、[小图逐次销毁](2026-11-11-tiny-mnn-lifecycle/small-light-per-request.jsonl)
- [文档缓存](2026-11-11-tiny-mnn-lifecycle/document-light-cached.jsonl)、[文档逐次销毁](2026-11-11-tiny-mnn-lifecycle/document-light-per-request.jsonl)
- [独立进程十次原始测量](2026-11-11-tiny-mnn-lifecycle/process-per-request.json)
- [线程数及 parallel 对比原始测量](2026-11-11-tiny-mnn-lifecycle/parallel-process-comparison.json)

JSONL 末尾可能含 MNN 自身输出的设备能力行；runner 只解析以 `{` 开头的 JSON 记录。原始记录中的临时路径标识当时输入位置，保存的输入和可重复命令使证据不依赖临时目录长期存在。
