#!/usr/bin/env python3
"""Inspect MNN FlatBuffer weight storage without executing inference.

Field positions are pinned to MNN d407447ed56c4121a11ccbd266dc184ca1ead0c2:
schema/default/{MNN,CaffeOp,Tensor,Type}.fbs. Convolution IDSTQuan.type=3
means FP16 weights; Blob.dataType=19 means DT_HALF. FP32 biases and integer
shape constants are counted separately, so 'fp16' does not imply every tensor
or the runtime's arithmetic uses half precision.
"""
import argparse
import collections
import hashlib
import json
import pathlib
import struct

REVISION = "664a945953e46186b37203fa4902a20d60a32809"


def inspect(path):
    data = path.read_bytes()

    def scalar(pos, fmt="I"):
        return struct.unpack_from("<" + fmt, data, pos)[0]

    def field(table, index):
        vtable = table - scalar(table, "i")
        entry = vtable + 4 + index * 2
        offset = scalar(entry, "H") if entry < vtable + scalar(vtable, "H") else 0
        return table + offset if offset else 0

    def pointer(pos):
        return pos + scalar(pos) if pos else 0

    def vector_count(table, index):
        pos = pointer(field(table, index))
        return scalar(pos) if pos else 0

    root = scalar(0)
    ops = pointer(field(root, 3))
    counts = collections.Counter()
    storage = collections.Counter()
    for index in range(scalar(ops)):
        op = pointer(ops + 4 + index * 4)
        kind_pos = field(op, 1)
        kind = scalar(kind_pos, "B") if kind_pos else 0
        main = pointer(field(op, 2))
        if kind == 9:  # OpParameter.Convolution2D
            fp32_count = vector_count(main, 1)
            storage["fp32_convolution_weight_values"] += fp32_count
            storage["fp32_convolution_bias_values"] += vector_count(main, 2)
            quant = pointer(field(main, 3))
            if quant:
                type_pos = field(quant, 2)
                quant_type = scalar(type_pos, "i") if type_pos else 0
                counts[f"convolution_quant_type_{quant_type}"] += 1
                if quant_type == 3:
                    storage["fp16_convolution_weight_values"] += vector_count(quant, 0) // 2
                else:
                    storage[f"quant_type_{quant_type}_buffer_bytes"] += vector_count(quant, 0)
            elif fp32_count:
                counts["fp32_convolution"] += 1
        elif kind == 7:  # OpParameter.Blob
            type_pos = field(main, 2)
            dtype = scalar(type_pos, "i") if type_pos else 1
            counts[f"constant_dtype_{dtype}"] += 1
            if dtype == 19:
                storage["fp16_constant_values"] += vector_count(main, 3) // 2
            elif dtype == 1:
                storage["fp32_constant_values"] += vector_count(main, 7)
    fp16 = storage["fp16_convolution_weight_values"] + storage["fp16_constant_values"]
    fp32_weights = storage["fp32_convolution_weight_values"]
    return {"operators": scalar(ops), "counts": dict(counts), "storage": dict(storage),
            "weight_storage": "fp16_weights_fp32_biases" if fp16 and not fp32_weights else "fp32" if fp32_weights and not fp16 else "mixed_or_unconfirmed"}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-dir", type=pathlib.Path, default=pathlib.Path("/tmp/snaplingo-ocr-benchmark/mnn/models"))
    parser.add_argument("--output", type=pathlib.Path, default=pathlib.Path("weights-manifest.json"))
    args = parser.parse_args()
    result = {"ocr_rs_revision": REVISION, "schema_revision": "d407447ed56c4121a11ccbd266dc184ca1ead0c2", "files": []}
    for path in sorted(args.model_dir.iterdir()):
        if path.suffix not in (".mnn", ".txt"):
            continue
        content = path.read_bytes()
        entry = {"file": path.name, "bytes": len(content), "sha256": hashlib.sha256(content).hexdigest(),
                 "source_url": f"https://raw.githubusercontent.com/zibo-chen/ocr-rs/{REVISION}/models/{path.name}"}
        if path.suffix == ".mnn":
            entry.update(inspect(path))
        else:
            entry["dictionary_lines"] = len(content.decode().splitlines())
        result["files"].append(entry)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
