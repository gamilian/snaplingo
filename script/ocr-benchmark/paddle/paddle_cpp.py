"""Single-input/output float32 Paddle C++ CPU adapter for shared OCR pipelines."""

import argparse
import ctypes
import json
from pathlib import Path

import numpy as np


DEFAULT_LIBRARY = Path("/tmp/snaplingo-ocr-benchmark/paddle/build/libsnaplingo_paddle_bridge.dylib")


class PaddleCppSession:
    def __init__(self, model_dir, threads=4, library=DEFAULT_LIBRARY):
        self.lib = ctypes.CDLL(str(library))
        self.lib.sl_paddle_last_error.restype = ctypes.c_char_p
        self.lib.sl_paddle_version.restype = ctypes.c_char_p
        self.version = self.lib.sl_paddle_version().decode()
        self.lib.sl_paddle_create.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_int, ctypes.POINTER(ctypes.c_double)]
        self.lib.sl_paddle_create.restype = ctypes.c_void_p
        self.lib.sl_paddle_run.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_float), ctypes.POINTER(ctypes.c_int), ctypes.c_int, ctypes.POINTER(ctypes.c_double)]
        self.lib.sl_paddle_run.restype = ctypes.c_int
        self.lib.sl_paddle_output_rank.argtypes = [ctypes.c_void_p]
        self.lib.sl_paddle_output_rank.restype = ctypes.c_int
        self.lib.sl_paddle_output_shape.argtypes = [ctypes.c_void_p]
        self.lib.sl_paddle_output_shape.restype = ctypes.POINTER(ctypes.c_int)
        self.lib.sl_paddle_output_data.argtypes = [ctypes.c_void_p]
        self.lib.sl_paddle_output_data.restype = ctypes.POINTER(ctypes.c_float)
        self.lib.sl_paddle_destroy.argtypes = [ctypes.c_void_p]
        model_dir = Path(model_dir)
        load_ms = ctypes.c_double()
        self.handle = self.lib.sl_paddle_create(
            str(model_dir / "inference.json").encode(),
            str(model_dir / "inference.pdiparams").encode(),
            threads,
            ctypes.byref(load_ms),
        )
        if not self.handle:
            raise RuntimeError(self.lib.sl_paddle_last_error().decode())
        self.load_ms = load_ms.value
        self.timings_ms = {}

    def run(self, tensor):
        tensor = np.ascontiguousarray(tensor, dtype=np.float32)
        shape = (ctypes.c_int * tensor.ndim)(*tensor.shape)
        timings = (ctypes.c_double * 3)()
        status = self.lib.sl_paddle_run(
            self.handle, tensor.ctypes.data_as(ctypes.POINTER(ctypes.c_float)),
            shape, tensor.ndim, timings,
        )
        if status:
            raise RuntimeError(self.lib.sl_paddle_last_error().decode())
        rank = self.lib.sl_paddle_output_rank(self.handle)
        output_shape = tuple(self.lib.sl_paddle_output_shape(self.handle)[i] for i in range(rank))
        output = np.ctypeslib.as_array(self.lib.sl_paddle_output_data(self.handle), shape=(int(np.prod(output_shape)),)).reshape(output_shape).copy()
        self.timings_ms = dict(zip(("input_copy_ms", "infer_ms", "output_copy_ms"), timings))
        return output

    def close(self):
        if getattr(self, "handle", None):
            self.lib.sl_paddle_destroy(self.handle)
            self.handle = None

    def __del__(self):
        self.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("model_dir")
    parser.add_argument("--input", help="Optional preprocessed float32 .npy tensor")
    parser.add_argument("--shape", default="1,3,48,320")
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--repeat", type=int, default=1)
    parser.add_argument("--output", help="Optional output .npy path")
    parser.add_argument("--library", default=str(DEFAULT_LIBRARY))
    args = parser.parse_args()
    tensor = np.load(args.input) if args.input else np.zeros(tuple(map(int, args.shape.split(","))), dtype=np.float32)
    try:
        session = PaddleCppSession(args.model_dir, args.threads, args.library)
        print(json.dumps({"stage": "load", "backend": "paddle_cpp_cpu", "version": session.version, "model": args.model_dir, "threads": args.threads, "load_ms": session.load_ms}), flush=True)
        for index in range(args.repeat):
            output = session.run(tensor)
            print(json.dumps({"stage": "run", "index": index, "input_shape": tensor.shape, "output_shape": output.shape, "finite": bool(np.isfinite(output).all()), **session.timings_ms}), flush=True)
        if args.output:
            np.save(args.output, output)
        session.close()
    except Exception as error:
        print(json.dumps({"stage": "error", "backend": "paddle_cpp_cpu", "error": str(error)}), flush=True)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
