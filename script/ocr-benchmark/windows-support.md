# OCR 推理引擎：Windows 与 macOS 部署核查

> 核查日期：2026-11-01。本文件只记录一手文档、发布资产和构建源码的支持性证据。没有 Windows 测试机，未执行 Windows 性能或精度测试，不能给 Windows 引擎排名，也不能将本机 Mac 耗时外推到 Windows。

## 已确认的支持

| 引擎 / 平台 | 已确认后端 | 发布与部署要点 | 证据 |
| --- | --- | --- | --- |
| ONNX Runtime / Windows x64 | CPU；DirectML；NVIDIA CUDA | v1.31.1 发布有 x64 CPU、CUDA12 和 CUDA13 C/C++ 库。DirectML 使用专门的 DML 构建/NuGet 包，普通 CPU 库不自动具备 DML。CUDA 需匹配 NVIDIA 驱动、CUDA/cuDNN DLL 与运行库版本。 | [O1][O2][O3] |
| ONNX Runtime / Apple Silicon | CPU；Core ML EP | v1.31.1 提供 macOS arm64 现成库。官方 arm64 C/C++ CPU 打包任务也启用 Core ML；仍需核对实际库并显式注册 EP。MLProgram 最低 macOS 12。 | [O1][O4] |
| ONNX Runtime / Intel Mac | CPU；可构建 Core ML EP | 本次最新发布 v1.31.1 没有 x86-64 Mac 资产；核查到 v1.23.2 有 x86_64 和 universal2 成品库。需要固定兼容版本及 Rust API 绑定，或自行构建新版本，不能直接混用 API31 的绑定与 API23 的库。 | [O1][O5][O6] |
| MNN / `ocr-rs` / Windows x64 | CPU；Vulkan/OpenCL；NVIDIA CUDA | 本次 `mnn-runtime-sys 0.1.0` 固定 MNN 3.6.1，有 Windows MSVC CPU、Vulkan/OpenCL、CUDA12 预编译 profile。**Windows 成品 profile 要求静态 CRT**；默认动态 CRT 组合可能改为源码构建。CUDA profile 要求 Toolkit >=12.8,<13，并仍需外部 CUDA/cuBLAS 运行库。 | [M1][M2][M3] |
| MNN / `ocr-rs` / macOS arm64、x64 | CPU；Metal；另有 Core ML profile | 同一固定发布有 universal CPU/Metal 和 Metal/Core ML profile；Rust 构建链接 Apple 系统 Metal 等 framework。没有 Mac CUDA 路线。 | [M1][M2][M3] |
| Paddle Inference C++ / Windows x64 | CPU；NVIDIA CUDA | 官方 3.0.0 下载页明确提供 Windows CPU SDK；官方源码编译文档支持 `WITH_GPU=ON`。旧版官方下载页还列 CUDA GPU SDK，能证明平台路线存在，但其旧 CUDA 版本不能作为当前 PP-OCRv6 依赖要求。 | [P1][P2][P3] |
| Paddle Inference C++ / macOS arm64、x64 | CPU | 官方 3.0.0 提供 Intel 与 Apple Silicon SDK；macOS 源码文档明确 CPU-only。该路线不能当作 Metal/ANE 支持。当前 PaddleOCR C++ demo 教程主要覆盖 Linux/Windows，CMake 的非 Windows 分支仍有 Linux 库名/链接假设，完整 Mac OCR 构建需要适配。 | [P1][P4][P5] |

## Windows GPU 的实际约束

- **DirectML 覆盖 AMD、NVIDIA、Intel** 的 DirectX 12 GPU，要求相应驱动，不依赖 CUDA。官方例子包括 AMD GCN、NVIDIA Kepler、Intel Haswell 及更新设备；双显卡机器的 adapter 0 可能是集显，不能当作最快设备。[O2]
- DML 必须关闭 **memory pattern**（`DisableMemPattern` / `enable_mem_pattern=false`），并使用 `ORT_SEQUENTIAL`；同一个 session 的 `Run` 不允许并发。**这不是要求关闭 CPU memory arena**，二者是不同选项。当前文档支持到 ONNX opset 20，部分算子不支持；模型导出版本须核查。[O2]
- DML 允许动态维度，但官方说明创建 session 时已知 shape 更有效。OCR 检测分辨率、识别行宽和 batch 变化会影响初始化、调度和性能；若需要固定尺寸/分桶，应另测全部前后处理成本，不能仅测一个固定 tensor。[O2]
- CUDA 只适用于 NVIDIA。ORT 当前 CUDA12/CUDA13 库分别要求匹配的 CUDA 和 cuDNN 主版本；MNN 这次固定 CUDA12 profile 的约束来自其构建源码，不能与 ORT 条件混用。Paddle 的 GPU SDK/编译也需单独匹配自身依赖。[O3][M3][P2]
- **后端注册成功不保证全图 GPU**：ORT 按节点能力分配 EP，`GPU + CPU` 列表允许不支持节点回到 CPU；MNN `Pipeline.cpp` 也存在 backup execution。`ocr-rs` 的后端可用性检查能拒绝未注册后端，不能证明每个算子都在 GPU。性能记录应包含实际节点后端、初始化与 CPU/GPU 拷贝。[O7][M4]
- Microsoft 当前将 DirectML 标记为 sustained engineering，新特性转向 WinML；DML 仍受支持。这里核查的是现成 ORT DML 路线，没有将 WinML 列作已测试候选。[O2]

## 对 SnapLingo 的最小部署建议

1. **先确保三个官方发布目标均有 CPU 基线**：macOS arm64/x64、Windows x64。GPU 是平台策略；Windows 首先评估覆盖多厂商的 DML 或 Vulkan，NVIDIA CUDA作为独立可选路线，Mac 再选择 Metal/Core ML。此顺序是部署建议，不是速度排名。
2. Rust 中保持同一个本地 OCR 接口，后端内部保留模型/session，明确线程所有权。DML 按 session 串行执行。避免为两套引擎分别复制截图、结果和翻译流程。
3. **固定运行库、API绑定、模型和构建参数**。ORT Intel Mac 成品库版本落后于 arm64；可选全平台统一 v1.23.2并匹配绑定，或各平台构建/绑定隔离。具体模型能否在所选版本运行仍须验证。MNN 预编译包来自 `zibo-chen/mnn-native-prebuilds`，属于该 Rust 集成作者的成品包，不能称为 Alibaba 官方 SDK；应保留哈希并控制源码 fallback。[O5][O6][M1]
4. **链接与安装包一起验收**：Windows 使用 MSVC时明确 Rust/C++ CRT一致性；MNN默认成品包条件不满足时源码构建是正常分支。使用动态库时随应用部署对应 DLL；CUDA 静态主库仍可能依赖外部 GPU运行 DLL。MinGW 源构建还需其运行时或 `static-cpp-runtime`。Mac 动态库放入应用包，固定加载路径并随应用签名；MNN静态路径只链接系统 Apple framework时无需分发这些系统框架。[M1][M2][M3]
5. 无 Windows 真机前只能确认支持性、构建和安装依赖，不能宣布“DML/Vulkan/CUDA 更快”。至少用集显与独显各一台 Windows机器，记录冷启动、热运行 p50/p95、峰值内存和整条 OCR 延迟，再决定默认。精度比较不在本轮范围。

## 一手来源与固定版本

- [O1] [ONNX Runtime v1.31.1 发布资产](https://github.com/microsoft/onnxruntime/releases/tag/v1.31.1)，2026-10-26 发布。
- [O2] [官方 DirectML EP 文档](https://onnxruntime.ai/docs/execution-providers/DirectML-ExecutionProvider.html)：厂商覆盖、DX12、opset、memory pattern、串行/并发、动态 shape及维护状态。
- [O3] [官方 CUDA EP 文档](https://onnxruntime.ai/docs/execution-providers/CUDA-ExecutionProvider.html)：CUDA/cuDNN兼容条件及依赖。
- [O4] [官方 CoreML EP 文档](https://onnxruntime.ai/docs/execution-providers/CoreML-ExecutionProvider.html)：macOS条件、arm64打包、动态shape、缓存及算子支持。
- [O5] [ONNX Runtime v1.23.2 发布资产](https://github.com/microsoft/onnxruntime/releases/tag/v1.23.2)：`onnxruntime-osx-x86_64-1.23.2.tgz`与`universal2`。
- [O6] [ORT v1.31.1 C API header](https://github.com/microsoft/onnxruntime/blob/v1.31.1/include/onnxruntime/core/session/onnxruntime_c_api.h)：`ORT_API_VERSION=31`，`GetApi`请求不支持的版本返回空指针。
- [O7] [官方 EP架构说明](https://onnxruntime.ai/docs/execution-providers/)：按节点能力分配及GPU/CPU后备。
- [M1] [ocr-rs README，commit 664a945](https://github.com/zibo-chen/ocr-rs/blob/664a945953e46186b37203fa4902a20d60a32809/README.md)，本次本机检出的`ocr-rs 2.4.1`版本；后端、MSVC/MinGW及部署说明。
- [M2] [mnn-runtime-sys prebuilt.rs，commit 2d88f77](https://github.com/zibo-chen/mnn-runtime-rs/blob/2d88f779744a9029859743db9a55b5d00a2622c8/crates/mnn-runtime-sys/prebuilt.rs)，[`mnn-3.6.1-d407447e-r1`发布](https://github.com/zibo-chen/mnn-native-prebuilds/releases/tag/mnn-3.6.1-d407447e-r1)：已核对本机下载crate的`.cargo_vcs_info.json`，平台profile和静态CRT选择条件。
- [M3] 同一crate [build.rs](https://github.com/zibo-chen/mnn-runtime-rs/blob/2d88f779744a9029859743db9a55b5d00a2622c8/crates/mnn-runtime-sys/build.rs)、[build_support.rs](https://github.com/zibo-chen/mnn-runtime-rs/blob/2d88f779744a9029859743db9a55b5d00a2622c8/crates/mnn-runtime-sys/build_support.rs)、[README](https://github.com/zibo-chen/mnn-runtime-rs/blob/2d88f779744a9029859743db9a55b5d00a2622c8/crates/mnn-runtime-sys/README.md)：CUDA>=12.8,<13、外部运行库、框架链接、动态/静态构建和源码fallback。读取了本机`/tmp/snaplingo-ocr-benchmark/mnn/runtime-sys/`中的对应文件。
- [M4] 固定MNN源码 [Pipeline.cpp](https://github.com/alibaba/MNN/blob/d407447ed56c4121a11ccbd266dc184ca1ead0c2/source/core/Pipeline.cpp)、[Schedule.cpp](https://github.com/alibaba/MNN/blob/d407447ed56c4121a11ccbd266dc184ca1ead0c2/source/core/Schedule.cpp)：backup execution与后端选择。
- [P1] [Paddle Inference 3.0.0 SDK下载](https://www.paddlepaddle.org.cn/inference/v3.0/guides/install/download_lib.html)：Windows CPU和Mac x86-64/Apple Silicon CPU成品库。
- [P2] [Paddle Inference Windows源码编译](https://www.paddlepaddle.org.cn/inference/v3.0/guides/install/compile/source_compile_under_Windows.html)：CPU/NVIDIA GPU路线；其中旧CUDA建议不视作PP-OCRv6依赖要求。
- [P3] [官方旧版GPU SDK下载页](https://www.paddlepaddle.org.cn/inference/guides/install/download_lib.html#windows)，实际页面列2.4.2资产，仅用于确认Windows GPU SDK路线存在。
- [P4] [Paddle Inference macOS源码编译](https://www.paddlepaddle.org.cn/inference/v3.0/guides/install/compile/source_compile_under_MacOS.html)：CPU-only。
- [P5] [PaddleOCR C++ OCR教程](https://github.com/PaddlePaddle/PaddleOCR/blob/main/docs/version3.x/inference_deployment/local_inference/cpp/OCR.md)、[CMakeLists.txt](https://github.com/PaddlePaddle/PaddleOCR/blob/main/deploy/cpp_infer/CMakeLists.txt)：当前Linux/Windows教程和非Windows链接分支。
