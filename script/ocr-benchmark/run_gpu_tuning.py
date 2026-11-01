"""Compare valid Metal precision and batching strategies, with the matrix paused."""
import json
import subprocess
import sys
from pathlib import Path

work = Path("/tmp/snaplingo-ocr-benchmark")
script = Path(__file__).resolve().parent
results = work / "results" / "gpu-tuning"
results.mkdir(parents=True, exist_ok=True)
binary = work / "mnn" / "target" / "release" / "snaplingo-mnn-ocr-bench"
jobs = [("v6_tiny", "high", True, 8)]
jobs += [(model, precision, False, batch)
         for model in ("v6_small", "v6_medium")
         for precision, batch in (("normal", 8), ("high", 8), ("high", 32))]
for model, precision, parallel, batch in jobs:
    name = f"mnn-{model}-metal-{precision}-p{int(parallel)}-b{batch}"
    resource = results / f"{name}.resources.json"
    if resource.exists():
        continue
    print("running " + name, flush=True)
    subprocess.run([sys.executable, str(script / "run_job.py"), "--output", str(resource),
                    "--timeout", "300", "--", str(binary), "--model", model,
                    "--backend", "metal", "--precision", precision,
                    "--parallel", str(parallel).lower(), "--batch", str(batch),
                    "--manifest", str(work / "dataset" / "manifest.jsonl"),
                    "--output", str(results / f"{name}.jsonl"), "--repeats", "3"], check=True)
print("GPU strategy matrix complete", flush=True)
