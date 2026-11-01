# MNN + ocr-rs 性能测试准备

本目录是独立 Rust harness，不依赖产品 workspace。用于 macOS 的 CPU/Metal 性能、资源与原始输出 sanity 检查；本轮没有人工标注失败样本或 Windows 测试机，因此不产生精度排名或 Windows 实测结论。

## 固定版本与构建

- ocr-rs `2.4.1`，Git revision `664a945953e46186b37203fa4902a20d60a32809`（next 分支当时的提交，包含 GPU inference 修复）。源码只 checkout `src/` 与根构建文件，避免下载完整模型集合。
- mnn-runtime / mnn-runtime-sys `0.1.0`，其余 Rust 依赖锁在 `Cargo.lock`。
- 原生 MNN `3.6.1`，上游 revision `d407447ed56c4121a11ccbd266dc184ca1ead0c2`；使用 zibo-chen/mnn-native-prebuilds 的校验预编译包。原生发布包已含四个构建兼容补丁，原始发布 manifest 保存在 `native-prebuilt-manifest.json`。
- harness 的唯一运行库补丁是 `strict-backend.patch`：当请求 GPU、实际会话主后端不匹配时返回错误。它不修改模型、算子、前后处理或推理参数。

在 macOS 安装 Rust、Git、curl、Python 3、ripgrep 和 Xcode Command Line Tools 后运行：

```bash
bash script/ocr-benchmark/mnn/setup.sh
```

所有 checkout、模型、原生下载及 cargo target 在 `/tmp/snaplingo-ocr-benchmark/mnn/`。Cargo 使用当前用户的常规 registry cache。`MNN_REQUIRE_PREBUILT=1` 禁止无提示切换成源码构建。安装脚本校验源码 revision、已修改源码、每个模型的准确字节数/SHA-256，然后用 `--locked --features metal` 构建。`--prepare-only` 只准备与校验，不构建或运行。

2026-11-01 已在 macOS 14.8.3 arm64 / M1 Pro / Rust 1.92 构建 release；首次构建 2m24s、2 个编译 job。`backend-metal.jsonl` 记录 `native_registered=true`、Rust Metal feature 可用、原生版本 `3.6.1`。这个检查本身不运行 OCR。实际会话后端由额外 guard 在后续 session creation 时验证。

## 运行接口

```bash
/tmp/snaplingo-ocr-benchmark/mnn/target/release/snaplingo-mnn-ocr-bench \
  --model v6_tiny --backend metal --threads 4 --batch 8 --parallel true \
  --warmups 1 --repeats 3 \
  --manifest /tmp/snaplingo-ocr-benchmark/dataset/manifest.jsonl \
  --output /tmp/snaplingo-ocr-benchmark/mnn/v6_tiny-metal.jsonl
```

`--model` 接受 `v6_tiny`、`v6_small`、`v6_medium`、`v5_fp16`、`v5_fp32`；`--backend` 接受 `cpu` 或 `metal`。同一 Metal feature 的二进制也可显式跑 CPU。`MNN_BENCH_MODEL_DIR` 可覆盖模型目录。`--precision normal|high|low` 控制运行时精度偏好，默认 normal；这与模型权重存储 FP16/FP32 是不同维度。`--batch` 是正整数，默认 8，支持 `--batch 32`，通过 `RecOptions::with_batch_size` 配置。`--parallel true|false` 默认 true，通过 `OcrEngineConfig::with_parallel` 配置。两项都在 session、image、complete 事件中记录。`--min-confidence` 默认 0.5。

小范围 GPU 策略对照可使用 `--model v6_small --backend metal --precision low --batch 32 --parallel false` 或把模型改成 `v6_medium`，并分别测 `--batch 8`、normal/low 精度偏好。这样可观察批量策略相对于已有默认 normal/batch8/parallel true 的变化。新增参数需重新构建 harness；正在运行的旧二进制保持原配置。

注意这个 revision 默认 `parallel=true`，检测框达到 5 个就走 ExactWidthParallel，每个框单独以 N=1 推理，忽略 batch_size；少于 5 个框时一次最多 4 个，也达不到 batch8 或 batch32。保持默认 parallel 配置时，8→32 不改变实际识别 batch。

`--parallel false` 对所有框数选择 Batch。识别器按检测框顺序切成最多 batch_size 个一组，将每组裁剪/缩放/填充到该组最大宽度，构造 `[N,3,48,max_width]`，然后一次 `run_dynamic`；最后一组或总框数不足时实际 N 小于配置 batch。默认 `RecOptions.enable_batch=true`，这条 region 识别路径直接使用 batch_size，没有失败后重试单框或自动退回 ExactWidthParallel 的分支；推理错误或非三维输出会传回图像事件。上游只按输出 shape[0] 的数量解码，没有单独验证输出 N 等于输入 N，异常模型可能输出较少行，补测应保留逐图文本/行数 sanity。批量填充会改变计算量与原来的 exact-width 路径，结果和耗时差应视为完整策略差异。

manifest 为 JSONL，读取 `id` 和 `image`（也接受 `path`）；额外的 category/width/height/line_count 由统一调度器按 id 对齐。绝对路径直接使用，相对路径以 manifest 所在目录为基准。也可以在命令末尾提供图片路径。

JSONL 始终写入 `--output` 文件，避免 MNN 原生 stdout 日志污染数据。记录包括：

- `backend`：注册检查、版本、编译 feature、PID 与主机类型。
- `session`：模型读取与两模型 session creation 的 `load_ms`、完整配置字符串、文件字节/hash、会话主后端校验。
- `image`：逐图 `decode_ms`、`ocr_ms`、`total_ms`，原始文本、每行识别置信度、检测置信度、`bbox=[x,y,width,height]` 和可选四角点；warmup 与计入测量的 iteration 分开标记。错误会留在该图 `error` 字段。
- `complete`：正常完成标记。进程 RSS/峰值内存与资源采样由统一调度器负责。

每轮按整个 manifest 顺序运行，先做指定轮数的完整 warmup，再做测量轮；每图重新解码。OCR 计时包含检测/裁剪/识别/前后处理与必要 GPU 同步及传输，不包含文件解码、结果 JSON 序列化或逐行输出。总耗时包含文件解码和 OCR。使用相同模型、同一进程 session 的 CPU/Metal 原始输出对照只用于等价性 sanity，不可代替准确率评估。

## 默认前后处理与限制

保留 ocr-rs 默认配置：最大检测边长 960、检测 score 0.3、box 0.5、unclip 1.5、最小框面积 16、框 border 5、merge false、识别高 48、识别 min_score 0.3、标点 min_score 0.1、batch size 8、parallel true、结果 min_confidence 0.5。不启用 orientation model 或 robust rotated text 多次检测模式。这些实现与 RapidOCR 的裁剪、宽度策略、过滤/排布逻辑可能不同，不能把全部端到端耗时差归因于推理引擎。

注册检查调用真实原生 `MNNGetExtraRuntimeCreator`；session guard 读取 MNN 的 `BACKENDS[0]`。两者都不会把缺少 Metal 的运行伪装成 CPU 成功。MNN 仍可对个别不支持的算子使用 CPU backup，属于其正常混合执行策略，本 harness 不宣称所有算子都在 GPU 上。

预编译包包含 macOS arm64/x86_64 universal `libMNN.a` 与 `libMNN.dylib`；本 harness 静态链接 arm64 slice，`otool -L` 确认没有外部分发 MNN dylib 依赖，仅链接系统 C++/Metal 等框架。`native-provenance.json` 记录下载字节/hash、native lib 与最终二进制字节/hash、构建命令。模型尺寸见 `weights-manifest.json`，不用 native archive 压缩尺寸代替产品包体。

Windows 的 CPU/OpenCL/Vulkan/CUDA 属于上游支持能力，本机未编译或实测。0.1.0 的 Windows MSVC 预编译选择要求静态 CRT，消费方需同时设置 `-C target-feature=+crt-static` 与运行库 `crt-static` feature；默认不满足时会回退源码构建，本脚本强制 prebuilt 会报错。Windows GNU 需 MinGW；CUDA 需 MSVC/CUDA SDK。本安装脚本固定 macOS 临时路径与 Metal 构建，不作为 Windows 安装脚本。

## 权重与来源

`weights-manifest.json` 固定 14 个必需文件，来自同一 ocr-rs Git revision，包括 v6 三档与 v5 mobile FP16/FP32。`inspect_weights.py` 依据固定 MNN schema 直接检查 FlatBuffer：卷积 `IDSTQuan.type=3` 是 FP16，`Blob.dataType=19` 是 half；FP32 bias 和整数 shape constant 分开记录。六个 v6 MNN 与 v5 `*_fp16` 已确认主权重 FP16；v5 无后缀版本确认 FP32。转换脚本默认 `--fp16` 与这个二进制检查一致。

这只确认存储类型和下载身份，不证明转换数值与原始 ONNX 一致。跨 MNN/ONNX 的转换一致性仍应在后续数据与对照条件充分时单独验证；本轮主要采样性能与资源。

The benchmark also applies `strict-batch.patch` to reject region batches whose output N differs from the requested N. This is a diagnostic guard, not a recognition algorithm change.
