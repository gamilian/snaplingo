#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
bench_tmp=/tmp/snaplingo-ocr-benchmark/mnn
revision=664a945953e46186b37203fa4902a20d60a32809
prepare_only=false
if [[ "${1:-}" == --prepare-only ]]; then
    prepare_only=true
elif [[ $# -ne 0 ]]; then
    echo 'Usage: setup.sh [--prepare-only]' >&2
    exit 2
fi

mkdir -p "$bench_tmp/source" "$bench_tmp/models"
if [[ ! -d "$bench_tmp/source/.git" ]]; then
    git -C "$bench_tmp/source" init -q
    git -C "$bench_tmp/source" remote add origin https://github.com/zibo-chen/ocr-rs.git
    git -C "$bench_tmp/source" fetch --filter=blob:none --depth=1 origin "$revision"
    git -C "$bench_tmp/source" sparse-checkout init --cone
    git -C "$bench_tmp/source" sparse-checkout set src
    git -C "$bench_tmp/source" checkout -q --detach FETCH_HEAD
fi
[[ "$(git -C "$bench_tmp/source" rev-parse HEAD)" == "$revision" ]]
if ! rg -q 'Benchmark batch size mismatch' "$bench_tmp/source/src/rec.rs"; then
    git -C "$bench_tmp/source" diff --quiet HEAD
    (cd "$bench_tmp/source" && patch -p1 < "$script_dir/strict-batch.patch")
fi

if [[ ! -d "$bench_tmp/runtime-sys" ]]; then
    cargo info mnn-runtime-sys@0.1.0 >/dev/null
    package_dir=
    for candidate in "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/mnn-runtime-sys-0.1.0; do
        if [[ -f "$candidate/cpp/mnn_runtime_bridge.cpp" ]]; then
            package_dir="$candidate"
            break
        fi
    done
    [[ -n "$package_dir" ]]
    cp -R "$package_dir" "$bench_tmp/runtime-sys"
fi
if ! rg -q 'Benchmark forbids backend fallback' "$bench_tmp/runtime-sys/cpp/mnn_runtime_bridge.cpp"; then
    (cd "$bench_tmp/runtime-sys" && patch -p1 < "$script_dir/strict-backend.patch")
fi

# Only fetch the 14 pinned model/dictionary files named in this manifest.
python3 - "$script_dir/weights-manifest.json" "$bench_tmp/models" <<'PY'
import hashlib
import json
import pathlib
import subprocess
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text())
model_dir = pathlib.Path(sys.argv[2])
for entry in manifest["files"]:
    path = model_dir / entry["file"]
    download = not path.is_file()
    candidate = path.with_suffix(path.suffix + ".part") if download else path
    if download:
        subprocess.run(["curl", "-fLsS", "--retry", "2", "--output", str(candidate), entry["source_url"]], check=True)
    content = candidate.read_bytes()
    if len(content) != entry["bytes"] or hashlib.sha256(content).hexdigest() != entry["sha256"]:
        raise SystemExit(f"Pinned model bytes/hash mismatch: {candidate}")
    if download:
        candidate.replace(path)
print(f"Verified {len(manifest['files'])} pinned files ({sum(e['bytes'] for e in manifest['files'])} bytes).")
PY
python3 "$script_dir/inspect_weights.py" --model-dir "$bench_tmp/models" --output "$bench_tmp/verified-weights.json"

if [[ "$prepare_only" == true ]]; then
    exit 0
fi
cd "$script_dir"
export CARGO_TARGET_DIR="$bench_tmp/target"
export CARGO_BUILD_JOBS=2
export MNN_REQUIRE_PREBUILT=1
export MNN_PREBUILT_CACHE_DIR="$bench_tmp/native-cache"
cargo build --release --locked --features metal
"$CARGO_TARGET_DIR/release/snaplingo-mnn-ocr-bench" \
    --backend metal --check-backend --output "$bench_tmp/backend-metal.jsonl"
cat "$bench_tmp/backend-metal.jsonl"
