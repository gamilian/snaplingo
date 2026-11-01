"""CLI baseline; process/model initialization is included in every OCR call."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=3)
    args = parser.parse_args()
    rows = [json.loads(x) for x in args.manifest.read_text().splitlines() if x]
    env = dict(os.environ, OMP_THREAD_LIMIT="4")
    with args.output.open("w") as output:
        output.write(json.dumps({"type": "environment", "backend": "tesseract_cli",
                                 "language": "chi_sim+eng", "psm": 3,
                                 "timing_scope": "CLI process and model loading included per image"}) + "\n")
        for iteration in range(args.repeats + 1):
            for row in rows:
                start = time.perf_counter()
                result = subprocess.run(["tesseract", row["image"], "stdout", "-l", "chi_sim+eng",
                                         "--psm", "3"], capture_output=True, text=True, env=env)
                elapsed = (time.perf_counter() - start) * 1000
                event = {"type": "image", "id": row["id"], "model": "tesseract",
                         "backend": "tesseract_cli", "warmup": iteration == 0,
                         "iteration": iteration - 1, "ocr_ms": elapsed, "total_ms": elapsed,
                         "line_count": len(result.stdout.strip().splitlines()), "text": result.stdout.strip(),
                         "error": result.stderr if result.returncode else None}
                output.write(json.dumps(event, ensure_ascii=False) + "\n")
                output.flush()


if __name__ == "__main__":
    main()
