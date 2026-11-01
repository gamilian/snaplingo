"""Download official ONNX weights and verify the model repository's LFS SHA256."""
import argparse
import concurrent.futures
import hashlib
import json
import urllib.request
from pathlib import Path


def fetch(url):
    return urllib.request.urlopen(urllib.request.Request(
        url, headers={"User-Agent": "SnapLingo-OCR-benchmark"}), timeout=120)


def download_model(item):
    name, output = item
    directory = output / name
    directory.mkdir(parents=True, exist_ok=True)
    base = f"https://huggingface.co/PaddlePaddle/{name}_onnx"
    with fetch(f"https://huggingface.co/api/models/PaddlePaddle/{name}_onnx?blobs=true") as r:
        info = json.load(r)
    revision = info["sha"]
    records = []
    for entry in info["siblings"]:
        filename = entry["rfilename"]
        if filename not in ("inference.onnx", "inference.yml", "config.json"):
            continue
        path = directory / filename
        expected = entry.get("lfs", {}).get("sha256")
        if not path.exists() or (expected and hashlib.sha256(path.read_bytes()).hexdigest() != expected):
            with fetch(f"{base}/resolve/{revision}/{filename}") as r, path.open("wb") as f:
                while data := r.read(1024 * 1024):
                    f.write(data)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if expected and digest != expected:
            raise RuntimeError(f"SHA256 mismatch: {path}")
        records.append({"file": str(path), "bytes": path.stat().st_size,
                        "sha256": digest, "url": f"{base}/resolve/{revision}/{filename}"})
    return {"model": name, "revision": revision, "license": info.get("cardData", {}).get("license"),
            "files": records}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    names = [f"PP-OCRv6_{tier}_{task}" for tier in ("tiny", "small", "medium")
             for task in ("det", "rec")]
    names += [f"PP-OCRv5_{tier}_{task}" for tier in ("mobile", "server")
              for task in ("det", "rec")]
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
        records = list(pool.map(download_model, [(name, args.output) for name in names]))
    (args.output / "manifest.json").write_text(json.dumps(records, indent=2))
    print(json.dumps({"models": len(records), "bytes": sum(
        f["bytes"] for r in records for f in r["files"])}))


if __name__ == "__main__":
    main()
