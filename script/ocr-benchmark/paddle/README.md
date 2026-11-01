# Paddle C++ CPU benchmark bridge (macOS arm64)

This adapter passes already-preprocessed float32 tensors to the official Paddle
Inference C++ library. It returns the raw detector/recognizer tensor to a shared
Python OCR pipeline, so image decoding, normalization, box extraction, crop
selection and CTC decoding can be identical to the ONNX comparison.

## Verified compatibility

Host: macOS 14.8.3, Apple M1 Pro, arm64; built with AppleClang 16 and CMake 4.1.1.
The SDK is CPU-only (`WITH_GPU=OFF`, `WITH_MKL=OFF`, `WITH_ONEDNN=OFF`) and links
Apple Accelerate. This is not a Metal or ANE benchmark.

* SDK URL version **3.0.0** (commit `6ed5dd3833c32c3b21e14b1fb1a71f5a535a0fcc`)
  builds the bridge but fails to load both official PP-OCRv6 tiny models.
  `CreatePredictor` reports that convolution `strides` must contain PIR Int32
  attributes. Full errors are in `probe-v6tiny-{det,rec}.jsonl`.
* SDK URL version **3.3.1** identifies itself inside the archive as
  **3.3.0.dev20260319**, commit `7688495538f4d6c1893f084dd238a402e8f68ab6`.
  It successfully loads and runs both official PP-OCRv6 tiny models. The detector
  produces `[1,1,64,64]`; the recognizer produces `[1,40,6906]` from
  `[1,3,48,320]`. Both outputs contain only finite values.
* With the updated SDK, v6 small and medium det/rec also pass a single-run
  compatibility probe. Both recognition models produce `[1,40,18710]`.
  V5 mobile/server models are downloaded and hashed; their execution is left to
  the shared pipeline's scheduled benchmark.

These are single-run compatibility probes with synthetic tensors, not OCR
accuracy or performance comparisons. Executed v6 probe logs use the
`probe-sdk-3.3.1-*` convention. Long benchmarks are scheduled by the shared
benchmark runner.

## Prepare and call

All SDKs, model archives, extracted models and compiled objects live in
`/tmp/snaplingo-ocr-benchmark/paddle/`. Repo files contain only the bridge,
setup/inspection scripts and diagnostics.

```sh
sh script/ocr-benchmark/paddle/prepare.sh
/tmp/snaplingo-ocr-benchmark/python/bin/python script/ocr-benchmark/paddle/paddle_cpp.py \
  /tmp/snaplingo-ocr-benchmark/paddle/models/PP-OCRv6_tiny_rec_infer \
  --shape 1,3,48,320 --threads 4
```

`prepare.sh` fetches the official SDK at the 3.3.1 URL, all six v6 models and v5
mobile/server det/rec. It requires `curl`, `tar`, `cmake` and a C++17 compiler. Python
requires NumPy; no Paddle Python package, OpenCV or PaddleOCR package is required
by this adapter. Rebuild against another SDK with:

```sh
cmake -S script/ocr-benchmark/paddle -B /tmp/snaplingo-ocr-benchmark/paddle/build \
  -DCMAKE_BUILD_TYPE=Release -DPADDLE_SDK=/absolute/path/to/paddle_inference_install_dir
cmake --build /tmp/snaplingo-ocr-benchmark/paddle/build --parallel 2
```

Shared pipeline integration:

```python
from paddle_cpp import PaddleCppSession  # Add this directory to sys.path.

det = PaddleCppSession("/tmp/snaplingo-ocr-benchmark/paddle/models/PP-OCRv6_tiny_det_infer", threads=4)
prediction = det.run(detector_input_nchw_float32)
print(det.load_ms, det.timings_ms)
det.close()
```

One session owns one predictor. `run` returns a NumPy float32 array, not a list.
Keep sessions alive across images. `load_ms` includes predictor creation and
graph optimization. `timings_ms` separates input reshape/copy, `Predictor::Run`,
and C++ output copy; the end-to-end runner must separately time Python overhead,
NumPy's final copy and all preprocessing/postprocessing. `--input file.npy` and
`--output file.npy` allow inspection with the exact same input as another engine.
No service or subprocess per inference is required.

## Packaging and evidence

The dylib bridge has an absolute build RPATH to the downloaded SDK. A packaged
app needs the bridge plus `libpaddle_inference.dylib`, `libphi.dylib`,
`libphi_core.dylib` and `libcommon.dylib` together, with their `@rpath` entries
resolved by the application's RPATH (typically `@executable_path/../Frameworks`).
Accelerate, libc++ and libSystem are system dependencies. macOS code signing
and redistributable licence notices remain app-integration work.

The four current SDK dylibs total **257,893,993 bytes** (245.95 MiB), before the
small bridge and OCR models. Native models include the graph, parameters and
preprocessing/dictionary configuration; their extracted det+rec totals are:

| Model pair | Extracted bytes |
| --- | ---: |
| PP-OCRv6 tiny | 6,533,062 |
| PP-OCRv6 small | 31,481,281 |
| PP-OCRv6 medium | 139,110,993 |
| PP-OCRv5 mobile | 21,748,351 |
| PP-OCRv5 server | 173,199,936 |

`vtool -show-build` confirms that **all four newer SDK dylibs contain actual
Mach-O `LC_BUILD_VERSION minos=13.6`**, with SDK 13.3. The old 3.0.0 dylibs declare
`minos=11.4` (SDK 11.3), but fail to load the tested v6 models. The tested newer
CPU dynamic SDK therefore cannot be shipped unchanged for SnapLingo's minimum
macOS **12** requirement. Its minimum-version metadata must be treated as a
deployment constraint; macOS 12 compatibility has not been demonstrated.

The locally compiled bridge itself has `minos=14.0` from the host compiler's
default. A future application build can choose its bridge deployment target,
but that does not lower the official SDK dylibs' 13.6 requirement. The complete
Mach-O evidence is in `macho-build-version.log` and `manifest.json`.

Run `manifest.py` to record actual file sizes, SHA-256 hashes, the SDK's internal
version text and `otool -L` output in `manifest.json`. `build*.log`, `prepare.log`
and `probe*.jsonl` preserve the build and compatibility results. The unused
static `.a` files were removed to save 383,183,888 bytes (3.0.0) and 393,352,528
bytes (3.3.1 URL SDK); both archives and dynamic runtimes remain available.

`prepare.log` records a one-time shell error (`i: command not found`) after the
original download list finished because the setup script was edited while that
process was running. All downloaded archives were extracted successfully; the
updated SDK bridge had already been built separately (`build-sdk-3.3.1.log`).
The current setup source includes the server models and atomic archive downloads.
It was not rerun during the shared performance matrix, to avoid an extra build.

This harness currently targets the supplied macOS dylib. Windows SDK support
requires a separate DLL/import-library build and runtime packaging; it has not
been executed because no Windows test machine is available.
