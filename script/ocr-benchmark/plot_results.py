"""Publication-style charts for the completed comparison report."""
import argparse
import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--data", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    rows = json.loads(args.data.read_text())
    selected = [r for r in rows if r["status"] == "ok" and r["model"] in ("v6_small", "v6_medium")]
    selected.sort(key=lambda r: (r["model"], r["p50_total_ms"]))
    colors = {"cpu": "#3a709d", "paddle_cpp": "#986330", "metal": "#528e6b",
              "coreml_gpu": "#ad617b", "coreml_all": "#8063a6"}
    fig, axes = plt.subplots(1, 3, figsize=(18, max(5, len(selected) * .45)), sharey=True)
    positions = list(range(len(selected)))
    names = [r["name"] for r in selected]
    for ax, key, title in zip(axes, ("p50_total_ms", "p95_total_ms", "peak_rss_mb"),
                              ("Warm p50 (ms, log scale)", "Warm p95 (ms, log scale)", "Peak process-tree RSS (MB)")):
        values = [r[key] for r in selected]
        ax.barh(positions, values, color=[colors.get(r["backend"], "#557b94") for r in selected])
        ax.set_title(title, loc="left", fontsize=13, pad=12)
        ax.spines[["top", "right", "left"]].set_visible(False)
        ax.grid(axis="x", alpha=.2)
        ax.set_axisbelow(True)
        if key != "peak_rss_mb":
            ax.set_xscale("log")
            ax.set_xlim(10, max(values) * 1.6 if values else 100)
        else:
            ax.set_xlim(0, max(values) * 1.2 if values else 1)
        for y, value in zip(positions, values):
            position = value * 1.06 if key != "peak_rss_mb" else value + max(values) * .015
            ax.text(position, y, f"{value:.0f}", va="center", fontsize=9)
    axes[0].set_yticks(positions, names, fontsize=9)
    axes[0].invert_yaxis()
    fig.suptitle("PP-OCRv6: valid configurations on M1 Pro", fontsize=16, x=.03, ha="left")
    fig.text(.03, .015, "12 synthetic screenshot workloads; 3 warm repeats each. Accuracy not scored. RSS is not isolated GPU VRAM.", fontsize=9)
    fig.tight_layout(rect=(0, .04, 1, .96))
    fig.savefig(args.output / "latency-memory.png", dpi=160)
    plt.close(fig)

    names = ["mnn-v6_small-cpu", "ort-v6_small-cpu", "ort-v6_small-cpu-b1",
             "ort-v6_medium-coreml_all", "ort-v6_medium-cpu", "vision-auto"]
    by_name = {r["name"]: r for r in rows}
    categories = ["small", "compact", "document", "dense", "4k", "wide"]
    fig, ax = plt.subplots(figsize=(12, 6))
    for name in names:
        if name not in by_name or by_name[name]["status"] != "ok":
            continue
        row = by_name[name]
        ax.plot(categories, [row["cases"][c]["p50_total_ms"] for c in categories],
                marker="o", label=name, linewidth=2)
    ax.set_yscale("log")
    ax.set_ylabel("Warm pipeline p50 (ms, log scale)")
    ax.set_title("Latency depends on workload and model", loc="left", fontsize=15, pad=15)
    ax.grid(alpha=.2)
    ax.spines[["top", "right"]].set_visible(False)
    ax.legend(fontsize=9, loc="upper left")
    fig.text(.08, .01, "Includes PNG loading/decoding and OCR; excludes screen capture, UI and translation.", fontsize=9)
    fig.tight_layout(rect=(0, .03, 1, 1))
    fig.savefig(args.output / "workload-latency.png", dpi=160)
    plt.close(fig)


if __name__ == "__main__":
    main()
