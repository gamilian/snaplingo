"""Record exact downloaded model/archive/runtime sizes and hashes."""

import hashlib
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path("/tmp/snaplingo-ocr-benchmark/paddle")
OUTPUT = Path(__file__).resolve().parent / "manifest.json"


def digest(path):
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(block)
    return hasher.hexdigest()


def main():
    sdk_roots = [ROOT / "paddle_inference_install_dir", ROOT / "sdk-3.3.1/paddle_inference_install_dir"]
    entries = []
    paths = list(ROOT.glob("*.tgz")) + list((ROOT / "models").rglob("*"))
    for sdk in sdk_roots:
        paths.extend((sdk / "paddle/lib").glob("*.dylib"))
    for path in sorted(paths):
        if not path.is_file():
            continue
        entry = {"file": str(path.relative_to(ROOT)), "bytes": path.stat().st_size, "sha256": digest(path)}
        if path.suffix == ".tgz":
            version = path.name.split("-")[1]
            entry["source_url"] = f"https://paddle-inference-lib.bj.bcebos.com/{version}/cxx_c/MacOS/m1_clang_noavx_accelerate_blas/paddle_inference.tgz"
        elif path.is_relative_to(ROOT / "models"):
            archive_name = path.name if path.suffix == ".tar" else path.parent.name + ".tar"
            entry["source_url"] = "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/" + archive_name
        if path.suffix == ".json":
            entry["pir_base_code"] = json.loads(path.read_text()).get("base_code")
        entries.append(entry)
    sdks = []
    for sdk in sdk_roots:
        if sdk.exists():
            sdks.append({
                "directory": str(sdk),
                "version_txt": (sdk / "version.txt").read_text(),
                "runtime_dependencies": subprocess.check_output(["otool", "-L", str(sdk / "paddle/lib/libpaddle_inference.dylib")], text=True),
                "all_runtime_dependencies": {path.name: subprocess.check_output(["otool", "-L", str(path)], text=True) for path in sorted((sdk / "paddle/lib").glob("*.dylib"))},
                "macho_build_versions": {path.name: subprocess.check_output(["vtool", "-show-build", str(path)], text=True) for path in sorted((sdk / "paddle/lib").glob("*.dylib"))},
                "macos_deployment_target": next(line.split("=", 1)[1] for line in (sdk / "CMakeCache.txt").read_text().splitlines() if line.startswith("CMAKE_OSX_DEPLOYMENT_TARGET:")),
                "static_library_bytes": (sdk / "paddle/lib/libpaddle_inference.a").stat().st_size if (sdk / "paddle/lib/libpaddle_inference.a").exists() else None,
            })
    OUTPUT.write_text(json.dumps({"recorded_at_utc": datetime.now(timezone.utc).isoformat(), "root": str(ROOT), "sdk": sdks, "files": entries}, indent=2) + "\n")


if __name__ == "__main__":
    main()
