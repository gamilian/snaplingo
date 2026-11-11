# ADR 0011: Local PaddleOCR provider

## Status

Accepted (2026-11-01).

## Unified small MNN CPU update — 2026-11-11

Every desktop platform now uses PP-OCRv6 **small with MNN CPU**, replacing
the macOS tiny trial and Windows/Linux medium ORT route at the user's request. Every request
starts a short-lived worker from the signed application executable, before Tauri,
database and tray initialization. It loads the paired small models and dictionary,
returns the OCR result, and exits. The desktop serializes requests but retains no
model session. Destroying an in-process session did not return its allocator caches
to the OS in the measured workloads; process exit provides the requested memory
lifetime. There is no prewarming or idle inference loop.

Every package includes the same SHA-256-pinned small MNN weights and dictionary,
15,686,931 bytes (15.69 MB), plus licenses. MNN is statically linked; old tiny,
medium, ONNX Runtime and DirectML resources and their loading dependencies are
removed. All targets use CPU, four inference threads, parallel text-line handling
and recognition batch size eight. The old saved GPU preference is retained for
settings compatibility but ignored by this CPU-only provider; its UI switch is
removed. System OCR and user-selected provider identities are unchanged.

The fixed ocr-rs revision builds MNN 3.6.1 from a checksum-pinned source when no
matching native prebuilt exists. In particular, Windows MSVC's default dynamic
CRT uses the source-build route because upstream's Windows prebuilt requires
static CRT. Windows packages retain Visual C++ redistributable DLLs. The shared
native license archive supplies license text only, not foreign-platform binaries.

The model smoke test checks small directly on macOS, and the installation smoke
test runs a real OCR request through the signed application's worker entrypoint.
The small/tiny latency, memory and CPU comparison is recorded in the
[small worker benchmark](../research/2026-11-11-small-ocr-benchmark.md).
The initial decision and its historical measurements follow.

## Original decision — 2026-11-01

Add `paddle-ocr` as a selectable local provider using PP-OCRv6 medium detection and recognition. Existing persisted provider selection remains authoritative. Screenshots and file OCR continue through `OcrCoordinator`; capture, translation, history, and result-window workflows do not select inference engines.

`PaddleOcrProvider` owns provider metadata, reads the current persisted OCR GPU preference, and dispatches blocking work. Its adjacent `PaddleOcrEngine` port exposes recognition, resource availability, and CPU/GPU status. Infrastructure owns model/session loading, image preprocessing, DB detection postprocessing, crops, CTC decoding, native runtime selection, and fallback. Native engine names are private to Infrastructure.

| Target | CPU | GPU enabled |
| --- | --- | --- |
| Windows x64 | ORT CPU | ORT DirectML; fall back to CPU on initialization/inference failure |
| Apple Silicon, macOS 14+ | ORT CPU | ORT Core ML MLProgram, CPUAndGPU; fall back to CPU on failure |
| Apple Silicon, macOS 12/13 | MNN CPU | MNN CPU |
| Intel Mac | MNN CPU | MNN CPU pending GPU validation |
| Linux x64 | ORT CPU | ORT CPU |

The GPU preference defaults to enabled for the local model and is independent of the screenshot canvas's GPU compositing preference. Turning it off selects CPU with the same medium model. Sessions load lazily, remain cached, and execute serially, including DirectML's sequential/no-memory-pattern requirements. A failed GPU switches the current request to CPU; later requests reuse CPU until the preference is toggled. Valid empty-image results do not trigger device fallback.

ORT is dynamically loaded from an absolute application-resource path. On old Apple Silicon systems and Intel Macs, platform selection occurs before any ORT call. The optional arm64 ORT library requires macOS 14; the executable retains macOS 12 compatibility through statically linked MNN. Signing and release verification explicitly include the optional runtime while validating the executable and mandatory dependencies against macOS 12.

Model vocabulary stays paired with pinned weights. Recognition padding is zero after normalization; CTC decoding preserves full dictionary tokens and validates output batch/vocabulary sizes. Library pixel coordinates with a top-left origin are converted to the existing normalized bottom-left `OcrLine` boxes.

## Distribution and verification

`npm run ocr:prepare` prepares SHA-256-pinned assets from `script/ocr-assets.json`. Tauri development/build hooks invoke it automatically, and runtime recognition is fully offline. Generated model/native files are ignored by Git and bundled under `ocr/`; the source manifest and licenses are retained. Preparation accepts `--target` with a supported Rust target triple. CI prepares resources and runs the real-model smoke test before packaging.

Dependencies: ORT Rust rc.11 / C API 1.23, oar-ocr-core/derive 0.6.0 for portable image processors, and the benchmark-pinned ocr-rs revision for MNN. macOS arm64 uses official ORT 1.31.1; Windows uses Microsoft's DirectML NuGet 1.24.4 plus DirectML 1.15.4. The DirectML NuGet release is pinned separately from the newer ordinary ORT SDK. Rust 1.88 or newer is required.

Windows preparation also copies the Visual C++ x64 redistributable DLLs from the build machine's Visual Studio C++ Build Tools. They are deployed beside the executable so the installer does not require a separate redistributable installation. Native development/tests load these DLLs from the prepared resources folder. DirectML.dll loads only when creating GPU sessions, so a missing GPU-only dependency can still fall back to CPU. Windows installer smoke tests explicitly check the deployed DLLs and OCR weights.

Run `cargo test --locked --manifest-path src-tauri/Cargo.toml`, and after preparation run `cargo test --locked --manifest-path src-tauri/Cargo.toml bundled_small_smoke -- --ignored --nocapture`. The smoke fixture is a generated bilingual screenshot workload and checks nonempty output and coordinates, not recognition accuracy. Windows, Intel Mac, and macOS 12/13 still need their respective machine acceptance; local M1 tests also exercise MNN as the compatibility implementation.

## Tradeoffs

Mac arm64 bundles both ONNX and MNN medium weights to retain old-system support, increasing resources to approximately 242 MiB before the main executable/frontend. Intel Mac needs only MNN weights. Core ML can execute some nodes on CPU and has a first-use compilation cost; enabling the setting does not promise every operation runs on GPU. Large-page latency and accuracy remain bounded by the empirical report's stated scope.
