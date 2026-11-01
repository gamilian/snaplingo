"""Run CPU batch tuning, Paddle C++ engine comparison, and the legacy baseline."""
import subprocess
import sys
from pathlib import Path

script = Path(__file__).resolve().parent
work = Path("/tmp/snaplingo-ocr-benchmark")
output = work / "results" / "extra"
output.mkdir(parents=True, exist_ok=True)
manifest = work / "dataset" / "manifest.jsonl"
jobs = []
for model in ("v6_small", "v6_medium"):
    for batch in (1, 4):
        name = f"ort-{model}-cpu-b{batch}"
        jobs.append((name, [sys.executable, str(script / "rapid_bench.py"), "--model", model,
                           "--backend", "cpu", "--batch", str(batch),
                           "--models", str(work / "onnx-models"), "--manifest", str(manifest),
                           "--output", str(output / f"{name}.jsonl")]))
for model in ("v6_tiny", "v6_small", "v6_medium", "v5_mobile", "v5_server"):
    name = f"paddle-{model}-cpu"
    jobs.append((name, [sys.executable, str(script / "rapid_bench.py"), "--model", model,
                       "--backend", "paddle_cpp", "--models", str(work / "onnx-models"),
                       "--manifest", str(manifest), "--output", str(output / f"{name}.jsonl")]))
name = "tesseract-cli"
jobs.append((name, [sys.executable, str(script / "tesseract_bench.py"), "--manifest", str(manifest),
                   "--output", str(output / f"{name}.jsonl")]))
for name, command in jobs:
    resource = output / f"{name}.resources.json"
    if resource.exists():
        continue
    print("running " + name, flush=True)
    subprocess.run([sys.executable, str(script / "run_job.py"), "--output", str(resource),
                    "--timeout", "300", "--"] + command, check=True)
print("extra matrix complete", flush=True)
