# 中英文端侧 OCR 方案调研

> 核查日期：2026-11-01。场景是 SnapLingo 桌面截图与图片文件 OCR，重点为英文、简体中文、中英混排及小字号；也记录繁体支持。按 macOS Apple Silicon / Intel 和 Windows x64、离线 CPU 推理评估。Windows ARM64 是后续可评估目标，当前官方发布矩阵未包含它。本次查阅官方文档、模型卡及项目源码，未安装推理依赖、未运行 OCR 对照基准，也未修改应用代码。

> 后续已完成本机性能与资源实测：[实测对比报告](./2026-11-01-ocr-benchmark.md)。本页是资料调研，部署选择以实测报告的范围和限制为准。

## 结论

**首选评估 PP-OCRv6 small 的检测＋识别模型，使用 ONNX Runtime 离线运行。** 核查到 PaddleOCR 最新正式发布为 v3.7.0（2026-06-11），已经发布 PP-OCRv6；RapidOCR v3.10.0（2026-10-08）默认也已使用 v6 small。不能再把 PP-OCRv5 当作当前最新方案。[S1][S2][S3]

选择依据是：v6 small 同一识别模型覆盖简体、繁体、英文；官方测试包含屏幕显示类别，相比 v5 有明显改善；官方 ONNX 检测＋识别权重合计约 **31.04 MB**，提供比大型文档视觉语言模型更适合桌面快捷截图的部署起点。但这些资料没有证明它在 SnapLingo 的实际截图上必然超过 Apple Vision 或 Windows OCR，必须通过同图对照决定是否替换默认引擎。[S3][S4]

建议同时评估 **tiny（6.24 MB）** 和 **medium（138.59 MB）**，分别测轻量方案与质量上限。`medium` 或旧版本的 `server` 是模型档位名称，模型仍然可以在本地执行。medium 在官方 M4 的 ONNX 端到端测试中平均 5.55 秒/图，因此不宜未经实测直接作为快捷截图默认引擎。[S3][S4]

工程上先用 RapidOCR 的 Python 实现做质量对照，再在现有 Rust Provider 接口中接入固定版本模型和本地推理实现。RapidOCR 是模型部署与前后处理工具，并不是与 PaddleOCR 无关的另一套模型。ONNX Runtime、MNN 是推理引擎，换引擎主要影响兼容性、耗时和部署成本，不能直接等同于换了更好的识别模型。[S2][S5]

## 候选比较

下表的 MB 使用十进制，`1 MB = 1,000,000 bytes`。v5/v6 全部按 PaddlePaddle 官方 Hugging Face 模型仓库中 `inference.onnx` 的文件字节核算，包含检测和识别两个权重文件，**不含字典、方向分类器、运行库、缓存与安装器**；不是运行内存，更不是最终安装包大小。[S4]

| 方案 | 中英文与繁体 | 检测＋识别 ONNX 权重 | 对 SnapLingo 的判断 |
| --- | --- | --- | --- |
| PP-OCRv6 small | 简体、繁体、英文统一；另含日文及多种拉丁语种 | 9.88＋21.16 ≈ **31.04 MB** | 优先评估的默认档，质量与成本比较平衡 |
| PP-OCRv6 medium | 与 small 相同的核心语言 | 62.03＋76.55 ≈ **138.59 MB** | 测质量上限；耗时和内存代价需要接受 |
| PP-OCRv6 tiny | 简体、繁体、英文；不含日文 | 1.78＋4.46 ≈ **6.24 MB** | 测轻量方案；中文和困难字符质量不能按 small 推断 |
| PP-OCRv5 mobile | 简体、繁体、英文、日文等 | 4.83＋16.53 ≈ **21.36 MB** | 作为上一代对照，检查 v6 提升是否适用于本项目 |
| PP-OCRv5 server | 与 v5 mobile 相同的核心语言 | 88.12＋84.50 ≈ **172.62 MB** | 旧代大模型对照；优先考虑更小的新代 medium |
| EasyOCR | 支持 `ch_sim + en`；提供繁体模型，语言组合需按官方规则选择 | 本次未核算完整组合 | 适合 Python 快速试验；官方标准路线依赖 PyTorch，Rust 桌面发布成本更高 |
| Tesseract | 提供英文、简体、繁体语言数据 | 随语言数据与 fast/best 选择变化 | 保留既有基线；没有本次场景中优于新 PP-OCR 的一手证据 |
| PaddleOCR-VL 系列 | 多语言文档解析，文字、表格、公式等 | 0.9B 参数；本次未核算完整安装体积 | 更适合复杂文档，CPU 快捷截图不是首选 |

语言与模型来源：[S3][S4][S6][S7][S8][S9]。以上“优先”与“适合”是针对本项目约束的判断，不是已经跑出来的准确率排名。

## PP-OCRv6 为什么值得先试

### 官方质量证据与局限

PP-OCRv6 使用检测、文字行识别的两阶段流程；small/medium 统一支持简体、繁体、英文、日文及 46 种拉丁语系语言，tiny 不含日文。v5 也支持中英繁日，因此 v6 的价值主要是新版模型能力与部署效率，不只是增加中文支持。[S3][S6]

官方 v6 算法页在**同一个内部多场景文字识别评估集**中报告下列结果：[S3]

| 识别模型 | 印刷中文 | 印刷英文 | 繁体 | 屏幕显示 |
| --- | ---: | ---: | ---: | ---: |
| v6 small | 90.5% | 93.3% | 77.0% | 79.7% |
| v6 medium | 91.5% | 94.1% | 78.6% | 82.5% |
| v6 tiny | 86.7% | 88.4% | 65.0% | 71.2% |
| v5 mobile | 86.0% | 86.0% | 72.0% | 57.6% |
| v5 server | 90.1% | 85.1% | 74.7% | 68.1% |

这些是官方文字识别子任务数字，不能当作“整张截图字符准确率”或“最终端到端成功率”。“屏幕显示”类别也没有证明其数据分布等于 SnapLingo 的小字号软件截图。该评测未包含 Apple Vision、Windows OCR、EasyOCR 或 Tesseract，不能据此对这些引擎给出排名。v5/v6 的其他模型表可能采用另一评估集，不能把表间数字混用。[S3][S6]

### 官方耗时提供的量级

v6 算法页在 200 张通用＋文档图像上测量读取图像、前后处理和推理全流程，公布：[S3]

| 同一官方测试的硬件/后端 | v6 tiny | v6 small | v6 medium |
| --- | ---: | ---: | ---: |
| Apple M4 / ONNX Runtime | 0.35 秒/图 | 1.29 秒/图 | 5.55 秒/图 |
| Intel Xeon 8350C / ONNX Runtime | 0.22 秒/图 | 0.61 秒/图 | 3.31 秒/图 |

这支持 small 作为起点、medium 作为质量上限的选择，但并不预测 Intel Mac、用户 Windows PC、小选区、冷启动或 p95 延迟。字数、输入缩放、识别批次、线程数与运行库版本都要随实际实验记录。官方数据也不能作为“31 MB 模型只占 31 MB 内存”的依据。

## 部署路线

### RapidOCR ＋ ONNX Runtime

已核查 RapidOCR **v3.10.0 tag** 的配置与模型路由，默认为 v6 small 检测和识别，具备 tiny/small/medium 的 ONNX 下载路径；方向分类器仍单独配置，默认开启。对常规正向软件截图，可在对照实验中关闭方向分类器，避免把额外模型及成本算成“两个模型”的一部分。[S2][S5]

官方的安装入口是 `pip install rapidocr onnxruntime`，适合研发阶段建立参考结果。本地发布不必捆绑 Python：可通过 Rust ONNX Runtime 绑定或 C/C++ API 执行同一组模型，但需要实现或复用正确的前后处理，并验证结果与 Python 参考一致。旧 `RapidOcrOnnx` C++ 仓库 README 示例仍使用 PP-OCRv3，不能把“支持 ONNX”理解为它已经完整适配 v6。[S2][S10][S11]

接入中真正需要验证的环节包括：[S5][S10]

- 检测输入的颜色顺序、归一化、缩放和 padding；v6 配置不能盲目沿用旧代数值。
- 检测概率图阈值、轮廓提取、框扩张、四边形排序与坐标还原。
- 文字行透视裁剪、识别输入高度、长行宽度、batch 和 padding；动态宽度必须与模型实际输入约定一致。
- CTC 解码及**配套字典顺序**。仅下载 `.onnx` 而使用错误字典，会产生系统性错误字符。
- 阅读顺序、行合并、中英文空格与标点。平均置信度也不能跨模型直接比较。

### CPU、Core ML 与打包

建议第一版先测 ONNX Runtime CPU，让 macOS 和 Windows 共享模型与大部分处理代码。Microsoft 提供 C/C++ API；Windows 文档明确要求将 `onnxruntime.dll` 随应用部署并显式绑定相应副本。[S11]

可选择动态库随包部署，或编译静态运行库。静态方式也需要各目标平台构建和链接验证；动态方式需要资源路径、库加载、签名和安装后测试。模型、字典与运行库均固定版本并保存哈希。完整包大小应以各目标构建结果实测，不能从模型文件大小估算出精确答案。[S11][S12]

ONNX Runtime **确实有 CoreML Execution Provider**，可调用 CPU/GPU/Neural Engine。官方说明 EP 最低 macOS 10.15，MLProgram 格式最低 macOS 12；与 SnapLingo 当前 macOS 12 最低版本在 API 条件上相容。当前官方 macOS arm64 C/C++ CPU 包构建包含 `--use_coreml`，但实际选用的运行库必须核查。[S12]

Core ML 不能承诺自动提速：官方提示动态 shape 可能降低性能，未支持算子可能落到其他后端，Neural Engine 并不保证执行整张计算图，首次子图编译和缓存还影响冷启动。RapidOCR v3.10.0 默认 `use_coreml: false`。应在 CPU 结果正确后，再测 Core ML 的端到端速度、数值与缓存；不能把“EP 存在”当作“v6 已在本项目全图 ANE 验证通过”。[S5][S12]

### Rust 接入候选与版本约束

质量对照通过后，可以优先检查现成 Rust OCR 流程，再决定是否自己实现前后处理。以下是源码能力核查，尚未在 SnapLingo 编译或验证输出一致性。[S17][S18]

| 路线 | 已确认能力 | 需要核验的接入成本 |
| --- | --- | --- |
| `oar-ocr` ＋ ONNX Runtime | 当前源码提供 `pp_ocrv6(Tiny/Small/Medium)` 预设、配套字典、内存模型加载与离线路径；可复用已有检测/识别流程 | 当前 0.10.2 声明 Rust 1.95，固定依赖 `ort = 2.0.0-rc.13`；不要另加最新 `ort` 造成运行库/API 版本不一致；只选择经典 OCR 能力 |
| `ort` ＋项目内前后处理 | 可直接在 Rust 加载 ONNX，静态链接或运行时显式加载动态库；模型与推理控制最直接 | `ort` 当前 2.0.0-rc.15 声明 Rust 1.92；绑定本身不提供完整 OCR 的裁剪、框还原、CTC 解码，需要实现并与 RapidOCR 逐阶段对齐 |
| `ocr-rs` ＋ MNN | 仓库当前 `next` 分支支持 v6 三档，Rust API 加载检测/识别/字典，默认 CPU，另有 Metal/Core ML 等特性；转换脚本默认输出 MNN FP16 | 需要 Paddle/ONNX 到 MNN 的转换一致性测试、原生 MNN 库和平台构建；`next` 分支能力不能自动等同于已发布 crate 全部能力，正式接入应固定已验证 revision；FP16 不能直接承诺精度不变或 CPU 更快 |

这几条路线发布时都可以不带 Python，但都包含原生推理依赖；“Rust API”不等于“纯 Rust 实现”。`oar-ocr`、`ocr-rs` 是社区项目，并非 PaddlePaddle 官方 Rust SDK。`ocr-rs` 代码和 `oar-ocr` 为 Apache-2.0，`ort` 的 Cargo 元数据为 MIT OR Apache-2.0；转换权重和原生运行库仍须分别保留许可。[S17][S18]

SnapLingo 的 Cargo 声明最低 Rust 1.77.2，CI/发布使用 stable。采用上述最新 `ort` / `oar-ocr` 会高于项目声明的最低版本，正式方案须明确更新工具链要求，或固定更早且经验证的组合；本次没有修改该声明。另一个具体打包问题是：核查到 Microsoft ONNX Runtime **v1.31.1** 发布附件有 macOS arm64、Windows x64 等，但没有 macOS x86_64 成品附件。因此 Intel Mac 必须另外验证可用运行库来源/构建和部署目标，不能假定下载最新官方包就覆盖 SnapLingo 全部目标。[S19]

作为排除项，纯 Rust 的 `ocrs` 官方 README 明确当前只识别拉丁字母，中文仍属计划范围，不能满足本次中英文统一需求。[S20]

### 其他路线的取舍

- **Paddle Inference C++**：使用 PP-OCR 模型并不必须采用它，ONNX Runtime 或 MNN 是其他推理路线。Paddle Inference **官方支持 macOS CPU**：3.0.0 下载页提供 Intel x86-64 与 Apple Silicon `m1` 两种 C/C++ SDK 成品包，下载链接均已确认返回 HTTP 200；官方还提供 macOS CPU 源码编译教程。这是 C++ 推理 SDK 的支持证据，不是从 Python PaddlePaddle 的 Mac 支持推断而来。该 CPU 路线不表示已经支持 Metal / Neural Engine 加速。[S11][S14][S17]

  需要区分 SDK 与完整 OCR 流程：当前 PaddleOCR 通用 C++ OCR 教程明确覆盖 Linux/Windows，要求 Paddle Inference 和 OpenCV；`deploy/cpp_infer/CMakeLists.txt` 没有专门的 `APPLE` 分支，部分 `NOT WIN32` 路径使用 `libdnnl.so.3`、OpenBLAS 和 `-ldl -lrt -lgomp` 等 Linux 链接假设。因此不能声称其当前 C++ demo 在 Mac 上开箱即用。macOS SDK 的存在已确认，PP-OCRv6 与所选 Mac SDK 的兼容性、完整流水线的构建和速度**尚未在本项目验证**；仍可能需要修改构建与依赖配置。3.0.0 是已确认的 SDK 版本，本次不声称它是最新版本。[S13][S17][S18]
- **MNN / Rust 集成**：MNN 是另一推理引擎，可以使用 PP-OCR 转换模型；模型格式与转换正确性、目标平台库、预处理和实际能力须单独核查。选它是部署决策，并不是脱离 PP-OCR 的另一套准确率来源。[S14]
- **EasyOCR**：官方直接给出 `Reader(['ch_sim', 'en'])` 和 `gpu=False`，英文可与其他语言搭配。标准实现依赖 PyTorch，可作质量对照；开发侧容易运行不代表最终 Rust 安装包容易部署。本次不声称其对截图质量优于原生 OCR。[S7]
- **Tesseract**：项目已经接入，可作为对照。官方区分 tessdata_fast 与 tessdata_best，允许不同语言数据权衡速度/精度；更换语言数据也必须测试，不能把扫描文档适用经验当作软件截图的保证。[S8]
- **文档 VLM**：PaddleOCR-VL 官方当前算法文档已覆盖 1.6，仍为 0.9B 参数，面向表格、公式和文档解析。官方也提供 Apple Silicon / MLX-VLM 路线，其中仅明确说明完成 M4 精度验证，其他 Apple Silicon 尚未验证。它值得在未来复杂文件解析需求下再评估，目前不优先于专用两阶段 OCR。[S9]

## 许可证与离线分发

PaddleOCR、RapidOCR 代码采用 Apache-2.0；本次核查的 PaddlePaddle 官方 v5/v6 ONNX 模型卡同样明确标记 `apache-2.0`，因此模型权重许可不是只从代码 LICENSE 推断。RapidOCR v3.10.0 还提供单独的 `MODEL_LICENSES.md`，列出模型来源、版权和分发要求。[S4][S15]

发布时保留代码、模型、运行库各自 LICENSE / NOTICE 与来源；转换后的模型也保留上游归属。ONNX Runtime 使用 MIT 许可。EasyOCR 的代码许可不能自动覆盖任意第三方检测/识别模型；本次不对未逐一核查的第三方权重作统一许可承诺。[S7][S15][S16]

完整离线运行应预先提供模型、字典和运行库，显式使用本地路径，并在断网安装环境验证无隐式下载。首次获取模型和实际 OCR 推理是两个阶段；“运行时本地”不表示自动下载行为已被禁止。[S5]

## SnapLingo 接入位置

当前应用为 Tauri 2 ＋ Rust，官方发布 macOS `aarch64` / `x86_64` 和 Windows `x86_64`。项目没有 ONNX Runtime、Python sidecar 或模型资源打包配置。对应依据见 [`Cargo.toml`](../../src-tauri/Cargo.toml)、[release workflow](../../.github/workflows/release.yml) 和 [Tauri 配置](../../src-tauri/tauri.conf.json)。

可以新增一个本地 OCR Provider，将推理实现放在 Infrastructure 层，复用现有 `OcrCoordinator` 单选、持久化和事件机制。截图与文件 OCR 已共享协调器，接入无需改变截图、翻译或结果窗口工作流。CPU 推理在专用工作线程或 `spawn_blocking` 执行，并复用加载好的模型 session。见 [`trait_def.rs`](../../src-tauri/src/application/providers/ocr/trait_def.rs)、[`coordinator.rs`](../../src-tauri/src/application/providers/ocr/coordinator.rs)、[`file_input.rs`](../../src-tauri/src/application/providers/ocr/file_input.rs) 与现有 [`system_ocr.rs`](../../src-tauri/src/application/providers/ocr/impls/system_ocr.rs)。

输出适配尤其需要处理坐标：当前 `OcrResult.lines` 的矩形是归一化坐标，原点在图像左下；PP-OCR 常用左上原点的像素四边形。需要先撤销检测输入缩放/padding，再把四边形转换为当前轴对齐矩形与坐标方向，正确映射原图。见 [`domain/ocr.rs`](../../src-tauri/src/domain/ocr.rs)。本阶段无需为了新引擎扩大 Domain 输出结构。

当前截图 OCR 输入来自冻结截图裁剪并编码为 PNG，已向外补 2 个逻辑像素；传给 OCR 时不包含标注和鼠标。见 [`render.rs`](../../src-tauri/src/application/capture/render.rs)。对照评测应保存这一路真实输入，另外检查混合 DPI 下合成/缩放和新引擎的检测边长限制，避免把输入像素损失与识别模型错误混为一谈。当前 Vision 的 `0.62` 恢复阈值只适用于该实现，不应直接搬到新模型。

## 最小验证方案

建议先做一次独立对照试验，确定质量增益，再决定正式引擎和打包路线。以下是建议验收设计，尚不是项目已采用的指标。

1. **固定输入和金标准。** 收集约 150–300 张真实截图，并保留当前 OCR 失败样本；包含英文/中文/混排、深色/浅色、浏览器/IDE/PDF/聊天、代码与 URL、表格、多列、竖排、低对比度、小字号、Retina 与 Windows 100%/150%/200% DPI。先用独立样本确定参数，再用未参与调参的样本验收。保留未经重采样的原始裁剪像素。
2. **同图比较。** 跑当前 Apple Vision / Windows OCR、Tesseract、v6 small/tiny/medium，必要时增加 v5 mobile 控制组。同一阶段固定模型、字典、运行库、线程和缩放规则。原生引擎记录 OS 版本与语言设置。
3. **拆分定位错误。** 字符错误率 CER 分英文、中文、混排报告；英文加 WER，中文标点/空格、代码/URL 单独统计。记录漏行、误检、阅读顺序和错误替换，不让文本归一化掩盖产品需要保留的字符。
4. **隔离像素与模型问题。** 比较原始裁剪、受控放大、不同检测边长；失败样本同时看检测框与识别 crop。整张图缩小可能让细字笔画丢失，盲目放大和锐化也不保证恢复信息。增强策略需要逐类证明有效，不能统一套用文档去畸变模型。
5. **测真正端侧成本。** 在 Apple Silicon、Intel Mac、普通 Windows x64 CPU 各测冷启动、热运行 p50/p95、模型加载、峰值 RSS、CPU 占用、不同字数/图片大小、完整安装体积及断网可用性。ONNX 推理耗时与整条 OCR 流程耗时分开记录。
6. **保留决策结果。** 至少要求 small 在主要中英文失败类别上改善，且没有明显损害英文代码/URL；延迟与包体预算由实测决定。若 small 仍不够，比较 medium 的增益与成本，再决定是否需要更大模型或针对性训练。官方资料支持把 v6 作为候选，最终默认切换应由此实验决定。

## 一手来源

- [S1] [PaddleOCR v3.7.0 release](https://github.com/PaddlePaddle/PaddleOCR/releases/tag/v3.7.0)：2026-06-11 发布 PP-OCRv6。
- [S2] [RapidOCR v3.10.0 release](https://github.com/RapidAI/RapidOCR/releases/tag/v3.10.0)、[README](https://github.com/RapidAI/RapidOCR/blob/v3.10.0/README.md)：2026-10-08 发布、定位与 Python 安装入口。
- [S3] [PP-OCRv6 算法与基准](https://github.com/PaddlePaddle/PaddleOCR/blob/main/docs/version3.x/algorithm/PP-OCRv6/PP-OCRv6.md)：语言、内部评估集、200 图耗时。
- [S4] PaddlePaddle 官方 ONNX 模型仓库及 `api/models/...?...blobs=true` 文件元数据：v6 [tiny det](https://huggingface.co/PaddlePaddle/PP-OCRv6_tiny_det_onnx/tree/2ba1506c0380b8f0b03dd142459aac66d4421f6c) / [tiny rec](https://huggingface.co/PaddlePaddle/PP-OCRv6_tiny_rec_onnx/tree/2612ab37152ae0a677521bae4e1e3d4fb4cf7c30)，[small det](https://huggingface.co/PaddlePaddle/PP-OCRv6_small_det_onnx/tree/28fe5895c24fd108c19eb3e8479f4ab385fbfc62) / [small rec](https://huggingface.co/PaddlePaddle/PP-OCRv6_small_rec_onnx/tree/b8f84f0b80c529de40b4fbb3544b84fa7233a513)，[medium det](https://huggingface.co/PaddlePaddle/PP-OCRv6_medium_det_onnx/tree/61323801669c338b7891481ec7bac61ce31b576a) / [medium rec](https://huggingface.co/PaddlePaddle/PP-OCRv6_medium_rec_onnx/tree/50c7eacafc52fa7bcf4194e8cd08e46f8558504b)；v5 [mobile det](https://huggingface.co/PaddlePaddle/PP-OCRv5_mobile_det_onnx/tree/e6f4fa85f00e168c862bc462aebca69eef9b3d3d) / [mobile rec](https://huggingface.co/PaddlePaddle/PP-OCRv5_mobile_rec_onnx/tree/ed152b8b495f84de93cda5709d768548a9127622)，[server det](https://huggingface.co/PaddlePaddle/PP-OCRv5_server_det_onnx/tree/dcf248c4dcc064c6d030db241255f9e8fbc6733a) / [server rec](https://huggingface.co/PaddlePaddle/PP-OCRv5_server_rec_onnx/tree/b70df217f4fd99d14f970bad092cebe7d74cc4d1)。记录的是这些 revision 下两个 `inference.onnx` 的合计字节；模型卡均标记 Apache-2.0。
- [S5] RapidOCR v3.10.0 [config.yaml](https://github.com/RapidAI/RapidOCR/blob/v3.10.0/python/rapidocr/config.yaml)、[default_models.yaml](https://github.com/RapidAI/RapidOCR/blob/v3.10.0/python/rapidocr/default_models.yaml)：默认 v6 small、模型路由、字典、输入配置与 Core ML 开关。
- [S6] [PP-OCRv5 算法说明](https://github.com/PaddlePaddle/PaddleOCR/blob/main/docs/version3.x/algorithm/PP-OCRv5/PP-OCRv5.md)、[PaddleOCR 模型表](https://github.com/PaddlePaddle/PaddleOCR/blob/main/docs/version3.x/model_list.md)。
- [S7] [EasyOCR 官方 README](https://github.com/JaidedAI/EasyOCR/blob/master/README.md)：中英组合、CPU、PyTorch、检测与识别架构。
- [S8] [Tesseract 数据文件官方说明](https://tesseract-ocr.github.io/tessdoc/Data-Files.html)：语言文件及 fast/best 区别。
- [S9] [PaddleOCR-VL-1.6 算法](https://github.com/PaddlePaddle/PaddleOCR/blob/main/docs/version3.x/algorithm/PaddleOCR-VL/PaddleOCR-VL-1.6.md)、[Apple Silicon 使用教程](https://github.com/PaddlePaddle/PaddleOCR/blob/main/docs/version3.x/pipeline_usage/PaddleOCR-VL-Apple-Silicon.md)。
- [S10] [RapidOcrOnnx C++ README](https://github.com/RapidAI/RapidOcrOnnx/blob/main/README.md)、[PaddleOCR 获取 ONNX 模型](https://github.com/PaddlePaddle/PaddleOCR/blob/main/docs/version3.x/inference_deployment/others/obtaining_onnx_models.md)。
- [S11] [ONNX Runtime C API 与部署](https://onnxruntime.ai/docs/get-started/with-c.html)、[C++ API](https://onnxruntime.ai/docs/get-started/with-cpp.html)。
- [S12] [ONNX Runtime CoreML EP](https://onnxruntime.ai/docs/execution-providers/CoreML-ExecutionProvider.html)、[ONNX Runtime 构建](https://onnxruntime.ai/docs/build/inferencing.html)：平台、库、动态 shape、算子、缓存。
- [S13] PaddleOCR [通用 OCR C++ Linux 部署](https://github.com/PaddlePaddle/PaddleOCR/blob/main/docs/version3.x/inference_deployment/local_inference/cpp/OCR.md)、[Windows 部署](https://github.com/PaddlePaddle/PaddleOCR/blob/main/docs/version3.x/inference_deployment/local_inference/cpp/OCR_windows.md)。
- [S14] [MNN 官方项目](https://github.com/alibaba/MNN)。
- [S15] [PaddleOCR LICENSE](https://github.com/PaddlePaddle/PaddleOCR/blob/main/LICENSE)、[RapidOCR LICENSE](https://github.com/RapidAI/RapidOCR/blob/v3.10.0/LICENSE)、[RapidOCR MODEL_LICENSES](https://github.com/RapidAI/RapidOCR/blob/v3.10.0/python/MODEL_LICENSES.md)。
- [S16] [ONNX Runtime LICENSE](https://github.com/microsoft/onnxruntime/blob/main/LICENSE)。
- [S17] [Paddle Inference 3.0.0 官方 Mac C/C++ SDK 下载](https://www.paddlepaddle.org.cn/inference/v3.0/guides/install/download_lib.html#mac)、[macOS 源码编译教程](https://www.paddlepaddle.org.cn/inference/v3.0/guides/install/compile/source_compile_under_MacOS.html)；官方 [x86-64 SDK](https://paddle-inference-lib.bj.bcebos.com/3.0.0/cxx_c/MacOS/x86-64_clang_avx_accelerate_blas/paddle_inference.tgz) 与 [Apple Silicon SDK](https://paddle-inference-lib.bj.bcebos.com/3.0.0/cxx_c/MacOS/m1_clang_noavx_accelerate_blas/paddle_inference.tgz) 下载链接均确认 HTTP 200，未下载或执行 SDK。
- [S18] [PaddleOCR C++ 推理 CMakeLists.txt](https://github.com/PaddlePaddle/PaddleOCR/blob/main/deploy/cpp_infer/CMakeLists.txt)：平台分支及第三方库链接假设。
- [S17] `ort` [README](https://github.com/pykeio/ort/blob/main/README.md)、[Cargo.toml](https://github.com/pykeio/ort/blob/main/Cargo.toml)、[链接方式](https://ort.pyke.io/setup/linking)、[crates.io 元数据](https://crates.io/api/v1/crates/ort)：Rust 绑定、2.0.0-rc.15 / Rust 1.92、原生库链接与许可。
- [S18] `oar-ocr` [README](https://github.com/GreatV/oar-ocr/blob/main/README.md)、[Cargo.toml](https://github.com/GreatV/oar-ocr/blob/main/Cargo.toml)、[crates.io 元数据](https://crates.io/api/v1/crates/oar-ocr)；`ocr-rs` 当前 `next` revision `664a945953e46186b37203fa4902a20d60a32809` 的 [README](https://github.com/zibo-chen/ocr-rs/blob/664a945953e46186b37203fa4902a20d60a32809/README.md)、[Cargo.toml](https://github.com/zibo-chen/ocr-rs/blob/664a945953e46186b37203fa4902a20d60a32809/Cargo.toml)：v6 API、模型转换、原生依赖、特性与版本要求。
- [S19] [ONNX Runtime v1.31.1 release 与附件](https://github.com/microsoft/onnxruntime/releases/tag/v1.31.1)、[release API](https://api.github.com/repos/microsoft/onnxruntime/releases/tags/v1.31.1)：2026-10-26 发布；本次核查其附件中没有 macOS x86_64 成品。
- [S20] [ocrs 官方 README](https://github.com/robertknight/ocrs/blob/main/README.md#language-support)：当前只识别拉丁字母。
