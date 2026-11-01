"""Serialize all CPU/GPU workloads so they do not compete with each other."""
import argparse
import json
import subprocess
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--work", type=Path, default=Path("/tmp/snaplingo-ocr-benchmark"))
    parser.add_argument("--repeats", type=int, default=3)
    args = parser.parse_args()
    script = Path(__file__).resolve().parent
    results = args.work / "results" / "matrix"
    results.mkdir(parents=True, exist_ok=True)
    manifest = args.work / "dataset" / "manifest.jsonl"
    jobs = []
    for model in ("v6_tiny", "v6_small", "v6_medium", "v5_mobile", "v5_server"):
        backends = ["cpu", "coreml_gpu"]
        if model in ("v6_small", "v6_medium"):
            backends.append("coreml_all")
        for backend in backends:
            name = f"ort-{model}-{backend}"
            jobs.append((name, [sys.executable, str(script / "rapid_bench.py"),
                               "--model", model, "--backend", backend,
                               "--models", str(args.work / "onnx-models"),
                               "--manifest", str(manifest), "--output", str(results / f"{name}.jsonl"),
                               "--repeats", str(args.repeats)]))
    binary = args.work / "mnn" / "target" / "release" / "snaplingo-mnn-ocr-bench"
    for model in ("v6_tiny", "v6_small", "v6_medium", "v5_fp16", "v5_fp32"):
        for backend in ("cpu", "metal"):
            name = f"mnn-{model}-{backend}"
            jobs.append((name, [str(binary), "--model", model, "--backend", backend,
                               "--manifest", str(manifest), "--output", str(results / f"{name}.jsonl"),
                               "--repeats", str(args.repeats)]))
    for backend in ("auto", "cpu"):
        name = "vision-" + backend
        jobs.append((name, [str(args.work / "vision-bench"), str(manifest),
                           str(results / f"{name}.jsonl"), str(args.repeats), backend]))
    (results / "jobs.json").write_text(json.dumps(jobs, indent=2))
    for index, (name, command) in enumerate(jobs):
        resource = results / f"{name}.resources.json"
        if resource.exists():
            print(f"[{index + 1}/{len(jobs)}] existing {name}", flush=True)
            continue
        print(f"[{index + 1}/{len(jobs)}] running {name}", flush=True)
        subprocess.run([sys.executable, str(script / "run_job.py"), "--output", str(resource),
                        "--timeout", "300", "--"] + command, check=True)
    print("matrix complete", flush=True)


if __name__ == "__main__":
    main()
