"""Measure actual macOS OCR workers serially, including startup and model loading."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import tempfile
import time


def metadata(path):
    data = path.read_bytes()
    return {"path": str(path.resolve()), "bytes": len(data),
            "sha256": hashlib.sha256(data).hexdigest()}


def measure(binary, models, model, image, response, stderr):
    started = time.perf_counter()
    process = subprocess.Popen([
        str(binary.resolve()), f"--ocr-{model}-worker", str(models.resolve()),
        str(image.resolve()), str(response),
    ], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=stderr)
    _, status, usage = os.wait4(process.pid, 0)
    process.returncode = os.waitstatus_to_exitcode(status)
    elapsed_ms = (time.perf_counter() - started) * 1000
    assert process.returncode == 0, (model, image, process.returncode)
    result = json.loads(response.read_text())["Ok"]
    assert result["text"].strip() and result["lines"], (model, image)
    assert all(line["bounding_box"] is not None for line in result["lines"])
    cpu_ms = (usage.ru_utime + usage.ru_stime) * 1000
    return {"elapsed_ms": elapsed_ms, "cpu_ms": cpu_ms,
            "cpu_pct_one_core": cpu_ms / elapsed_ms * 100,
            "peak_rss_bytes": usage.ru_maxrss, "exit_code": process.returncode,
            "lines": len(result["lines"]), "chars": len(result["text"]),
            "text": result["text"]}


def main():
    assert sys.platform == "darwin", "wait4 peak RSS units are macOS bytes"
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--small-binary", type=Path, required=True)
    parser.add_argument("--small-models", type=Path, required=True)
    parser.add_argument("--tiny-binary", type=Path, required=True)
    parser.add_argument("--tiny-models", type=Path, required=True)
    parser.add_argument("--images", type=Path, required=True,
                        help="JSONL manifest with image paths relative to its directory")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=10)
    args = parser.parse_args()
    assert args.repeats > 0
    args.output.mkdir(parents=True, exist_ok=True)
    models = {"tiny": (args.tiny_binary, args.tiny_models),
              "small": (args.small_binary, args.small_models)}
    images = [json.loads(line) for line in args.images.read_text().splitlines() if line]
    for row in images:
        row["image"] = str((args.images.parent / row["image"]).resolve())
        assert metadata(Path(row["image"]))["sha256"] == row["sha256"]
    details = {
        "platform": platform.platform(), "machine": platform.machine(),
        "hardware": subprocess.check_output([
            "sysctl", "-n", "machdep.cpu.brand_string", "hw.memsize", "hw.logicalcpu"
        ], text=True).splitlines(),
        "scope": "worker spawn to exit; includes decode, model load, OCR, result write and teardown; excludes UI, translation and caller input staging",
        "memory": "wait4 ru_maxrss in bytes on macOS; worker process only",
        "cpu": "wait4 user+system CPU time / spawn-to-exit wall time; 100% = one logical core",
        "runtime": "MNN CPU, inference threads 4, parallel true, recognition batch 8",
        "cache": "new worker per request; no filesystem-cache purge; first request separately labeled",
        "quality": "nonempty text and bounding boxes only; no accuracy gold standard",
        "repeats": args.repeats, "images": images,
        "models": {model: {"binary": metadata(binary), "assets": [
            metadata(root / f"{model}_det.mnn"), metadata(root / f"{model}_rec.mnn"),
            metadata(root / f"{model}_keys.txt")
        ]} for model, (binary, root) in models.items()},
    }
    (args.output / "metadata.json").write_text(json.dumps(details, indent=2) + "\n")
    records = []
    with tempfile.TemporaryDirectory(prefix="snaplingo-worker-bench-") as temporary:
        response = Path(temporary) / "result.json"
        with (args.output / "workers.stderr").open("w") as stderr, \
                (args.output / "requests.jsonl").open("w") as output:
            for row in images:
                for iteration in range(args.repeats + 1):
                    # Alternate ordering to reduce bias from background activity.
                    order = ("tiny", "small") if iteration % 2 == 0 else ("small", "tiny")
                    for model in order:
                        binary, root = models[model]
                        request = measure(binary, root, model, Path(row["image"]), response, stderr)
                        request.update(model=model, image=row["id"], iteration=iteration,
                                       first_request=iteration == 0)
                        records.append(request)
                        output.write(json.dumps(request, ensure_ascii=False) + "\n")
                        output.flush()
                print(f"Measured {row['id']}: {args.repeats} repeats per model", flush=True)
    summary = []
    for row in images:
        for model in models:
            samples = [r for r in records if r["image"] == row["id"] and r["model"] == model]
            repeated = [r for r in samples if not r["first_request"]]
            times = sorted(r["elapsed_ms"] for r in repeated)
            summary.append({
                "image": row["id"], "width": row["width"], "height": row["height"],
                "model": model, "n": len(repeated), "first_ms": samples[0]["elapsed_ms"],
                "median_ms": statistics.median(times), "mean_ms": statistics.mean(times),
                "min_ms": min(times), "max_ms": max(times),
                "p95_ms": times[max(0, math.ceil(len(times) * .95) - 1)],
                "peak_rss_median_bytes": statistics.median(r["peak_rss_bytes"] for r in repeated),
                "peak_rss_max_bytes": max(r["peak_rss_bytes"] for r in samples),
                "cpu_pct_one_core": sum(r["cpu_ms"] for r in repeated) / sum(times) * 100,
                "lines": repeated[0]["lines"], "chars": repeated[0]["chars"],
            })
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2), flush=True)


if __name__ == "__main__":
    main()
