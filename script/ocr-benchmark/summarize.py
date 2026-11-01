"""Export comparable timing/resource metrics; never score recognition accuracy."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import shutil
import statistics


def quantile(values, q):
    values = sorted(values)
    if not values:
        return None
    position = (len(values) - 1) * q
    lower = int(position)
    upper = min(lower + 1, len(values) - 1)
    return values[lower] + (values[upper] - values[lower]) * (position - lower)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--work", type=Path, default=Path("/tmp/snaplingo-ocr-benchmark"))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    dataset = [json.loads(x) for x in (args.work / "dataset" / "manifest.jsonl").read_text().splitlines()]
    categories = {x["id"]: x["category"] for x in dataset}
    summaries = []
    condensed = []
    case_rows = []
    for directory in ("matrix", "gpu-tuning", "extra", "compat"):
        for path in sorted((args.work / "results" / directory).glob("*.jsonl")):
            resource_path = path.with_suffix(".resources.json")
            if not resource_path.exists():
                continue
            resource = json.loads(resource_path.read_text())
            records = [json.loads(x) for x in path.read_text().splitlines() if x]
            env = next((x for x in records if x["type"] in ("environment", "backend")), {})
            session = next((x for x in records if x["type"] == "session"), {})
            images = [x for x in records if x["type"] == "image"]
            hot = [x for x in images if not x.get("warmup")]
            valid = [x for x in hot if not x.get("error") and "total_ms" in x]
            zeros = sum(x.get("line_count", len(x.get("lines", []))) == 0 for x in valid)
            errors = sum(bool(x.get("error")) for x in hot)
            complete = len(hot) == 36 and resource.get("exit_code") == 0
            status = "ok" if complete and errors == 0 and zeros == 0 else "incomplete"
            if resource.get("timed_out"):
                status = "timeout"
            elif resource.get("memory_limited"):
                status = "memory_limit"
            elif resource.get("exit_code") != 0:
                status = "process_error"
            elif errors or zeros:
                status = "sanity_failed"
            values = [x["total_ms"] for x in valid]
            first = next((x for x in images if not x.get("error")), {})
            samples = resource.get("samples", [])
            summary = {"name": path.stem, "group": directory, "status": status,
                       "model": session.get("model", env.get("model", hot[0].get("model") if hot else "unknown")),
                       "backend": session.get("backend", env.get("backend")),
                       "hot_samples": len(hot), "zero_output_samples": zeros, "error_samples": errors,
                       "p50_total_ms": quantile(values, .5), "p95_total_ms": quantile(values, .95),
                       "p50_ocr_ms": quantile([x["ocr_ms"] for x in valid], .5),
                       "peak_rss_mb": resource.get("peak_sampled_rss_bytes", 0) / 1e6,
                       "load_ms": session.get("load_ms"), "first_input_ms": first.get("total_ms"),
                       "mean_cpu_percent": statistics.mean(x["cpu_percent"] for x in samples) if samples else None,
                       "exit_code": resource.get("exit_code"), "wall_seconds": resource.get("wall_seconds"),
                       "params": {k: session.get(k) for k in ("threads", "batch", "parallel", "config") if k in session},
                       "environment": env, "source": str(path), "cases": {}}
            for category in sorted(set(categories.values())):
                rows = [x for x in valid if categories[x["id"]] == category]
                numbers = [x["total_ms"] for x in rows]
                item = {"name": path.stem, "category": category, "samples": len(rows),
                        "p50_total_ms": quantile(numbers, .5), "p95_total_ms": quantile(numbers, .95),
                        "p50_lines": quantile([x.get("line_count", len(x.get("lines", []))) for x in rows], .5)}
                summary["cases"][category] = item
                case_rows.append(item)
            summaries.append(summary)
            for row in images:
                text = row.get("text", "")
                item = {k: v for k, v in row.items() if k not in ("text", "lines", "engine_calls")}
                item.update({"configuration": path.stem, "line_count": row.get("line_count", len(row.get("lines", []))),
                             "text_sha256": hashlib.sha256(text.encode()).hexdigest(), "text_preview": text[:120]})
                if row.get("engine_calls"):
                    item["engine_call_count"] = len(row["engine_calls"])
                    item["observed_batches"] = sorted(set(x["shape"][0] for x in row["engine_calls"]))
                condensed.append(item)
    (args.output / "summary.json").write_text(json.dumps(summaries, indent=2, ensure_ascii=False))
    (args.output / "observations.jsonl").write_text("".join(json.dumps(x, ensure_ascii=False) + "\n" for x in condensed))
    fields = ["name", "status", "model", "backend", "hot_samples", "zero_output_samples", "error_samples",
              "p50_total_ms", "p95_total_ms", "peak_rss_mb", "load_ms", "first_input_ms", "mean_cpu_percent"]
    with (args.output / "summary.csv").open("w", newline="") as f:
        writer = csv.DictWriter(f, fields, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(summaries)
    if case_rows:
        with (args.output / "case-metrics.csv").open("w", newline="") as f:
            writer = csv.DictWriter(f, list(case_rows[0]))
            writer.writeheader()
            writer.writerows(case_rows)
    dataset_dir = args.output / "dataset"
    dataset_dir.mkdir(exist_ok=True)
    for row in dataset:
        filename = Path(row["image"]).name
        shutil.copy2(row["image"], dataset_dir / filename)
        row["image"] = f"dataset/{filename}"
    (args.output / "dataset.json").write_text(json.dumps(dataset, indent=2, ensure_ascii=False))
    print(json.dumps({"configurations": len(summaries), "valid": sum(x["status"] == "ok" for x in summaries),
                      "image_observations": len(condensed), "output": str(args.output)}))


if __name__ == "__main__":
    main()
