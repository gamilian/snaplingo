#!/bin/sh
set -eu

bench_dir=/tmp/snaplingo-ocr-benchmark/paddle
source_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sdk_version=3.3.1
sdk_dir="$bench_dir/sdk-$sdk_version"
mkdir -p "$sdk_dir" "$bench_dir/models"

archive="$bench_dir/paddle_inference-$sdk_version-macos-arm64.tgz"
if [ ! -f "$archive" ]; then
  curl -fLsS --retry 2 "https://paddle-inference-lib.bj.bcebos.com/$sdk_version/cxx_c/MacOS/m1_clang_noavx_accelerate_blas/paddle_inference.tgz" -o "$archive.partial"
  mv "$archive.partial" "$archive"
fi
if [ ! -f "$sdk_dir/paddle_inference_install_dir/version.txt" ]; then
  tar -xzf "$archive" -C "$sdk_dir"
fi

for model in PP-OCRv6_tiny_det PP-OCRv6_tiny_rec PP-OCRv6_small_det PP-OCRv6_small_rec PP-OCRv6_medium_det PP-OCRv6_medium_rec PP-OCRv5_mobile_det PP-OCRv5_mobile_rec PP-OCRv5_server_det PP-OCRv5_server_rec; do
  archive="$bench_dir/models/${model}_infer.tar"
  if [ ! -f "$archive" ]; then
    curl -fLsS --retry 2 "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/${model}_infer.tar" -o "$archive.partial"
    mv "$archive.partial" "$archive"
  fi
  if [ ! -f "$bench_dir/models/${model}_infer/inference.json" ]; then
    tar -xf "$archive" -C "$bench_dir/models"
  fi
done

cmake -S "$source_dir" -B "$bench_dir/build" -DCMAKE_BUILD_TYPE=Release -DPADDLE_SDK="$sdk_dir/paddle_inference_install_dir"
cmake --build "$bench_dir/build" --parallel 2
