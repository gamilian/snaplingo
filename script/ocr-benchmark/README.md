# OCR benchmark harness

This is isolated research, not a product feature. See the
[completed report](../../docs/research/2026-11-01-ocr-benchmark.md).
Accuracy is not scored. Performance is measured on one M1 Pro Mac;
[Windows support](windows-support.md) is documented, not measured.

## Prepare

All large dependencies, models, SDKs and build outputs stay under
`/tmp/snaplingo-ocr-benchmark/`. Use Python 3.12.

```sh
uv venv --python 3.12 /tmp/snaplingo-ocr-benchmark/python
uv pip install --python /tmp/snaplingo-ocr-benchmark/python/bin/python -r script/ocr-benchmark/requirements.lock
/tmp/snaplingo-ocr-benchmark/python/bin/python script/ocr-benchmark/generate_dataset.py --output /tmp/snaplingo-ocr-benchmark/dataset
/tmp/snaplingo-ocr-benchmark/python/bin/python script/ocr-benchmark/download_models.py --output /tmp/snaplingo-ocr-benchmark/onnx-models
bash script/ocr-benchmark/mnn/setup.sh
sh script/ocr-benchmark/paddle/prepare.sh
clang -fobjc-arc -O2 -framework Foundation -framework Vision script/ocr-benchmark/vision_bench.m -o /tmp/snaplingo-ocr-benchmark/vision-bench
```

The generator uses a macOS system font. The report includes the exact 12 PNGs
and hashes for another platform to reuse the same pixels. MNN code/models are
pinned; the Paddle SDK's internal development version is recorded separately.

## Execute sequentially

```sh
/tmp/snaplingo-ocr-benchmark/python/bin/python script/ocr-benchmark/run_matrix.py
/tmp/snaplingo-ocr-benchmark/python/bin/python script/ocr-benchmark/run_gpu_tuning.py
/tmp/snaplingo-ocr-benchmark/python/bin/python script/ocr-benchmark/run_extra.py
```

Do not run matrices concurrently. A `PAUSE` file in a matrix output directory
holds the next job; remove it to resume. It never alters a measurement in progress.
Existing resource-result files are skipped, including failed attempts. Use a
fresh result directory or remove only the relevant owned results for a rerun.

Optional version/provider probes require a second environment:

```sh
uv venv --python 3.12 /tmp/snaplingo-ocr-benchmark/ort123
uv pip install --python /tmp/snaplingo-ocr-benchmark/ort123/bin/python rapidocr==3.10.0 onnxruntime==1.23.2 psutil
/tmp/snaplingo-ocr-benchmark/python/bin/python script/ocr-benchmark/run_postchecks.py
```

## Export

```sh
/tmp/snaplingo-ocr-benchmark/python/bin/python script/ocr-benchmark/summarize.py --output docs/research/2026-11-01-ocr-benchmark
/tmp/snaplingo-ocr-benchmark/python/bin/python script/ocr-benchmark/plot_results.py --data docs/research/2026-11-01-ocr-benchmark/summary.json --output docs/research/2026-11-01-ocr-benchmark
```

Normal matrices use one warmup and three repeats, 36 hot image observations.
Resource sampling is every 50 ms; process RSS is capped at 6 GB and a full job at
300 seconds. RSS is not isolated GPU VRAM, and Core ML service allocations can
be external. Failed/empty/incomplete outputs are excluded from valid speed claims.

MNN uses native Rust OCR; ORT/Paddle share RapidOCR Python pre/postprocessing.
Complete-process memory includes different frontends. Paddle native engine time
is recorded inside `Predictor::Run`. Product integration should use native APIs.
Raw logs/caches stay in `/tmp`; the report retains condensed observations, tables,
workload PNGs and provenance. These scripts currently orchestrate the Mac test;
they do not claim an executed Windows benchmark.
