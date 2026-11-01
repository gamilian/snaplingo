"""Offline RapidOCR/ORT benchmark. Accuracy is deliberately not scored."""
import argparse
import json
import os
import sys
import time
from pathlib import Path
from types import SimpleNamespace

IMPORT_START = time.perf_counter()
import cv2
import onnxruntime as ort
import yaml
from rapidocr import RapidOCR
from rapidocr.inference_engine.onnxruntime.main import OrtInferSession
from rapidocr.utils.typings import ModelType, OCRVersion
IMPORT_MS = (time.perf_counter() - IMPORT_START) * 1000


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True)
    parser.add_argument("--backend", choices=["cpu", "coreml_gpu", "coreml_all", "paddle_cpp"], required=True)
    parser.add_argument("--models", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--batch", type=int, default=8)
    parser.add_argument("--audit", action="store_true")
    parser.add_argument("--native-models", type=Path, default=Path("/tmp/snaplingo-ocr-benchmark/paddle/models"))
    parser.add_argument("--coreml-format", choices=["MLProgram", "NeuralNetwork"], default="MLProgram")
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    output = args.output.open("w")

    def emit(row):
        output.write(json.dumps(row, ensure_ascii=False, default=str) + "\n")
        output.flush()

    available = ort.get_available_providers()
    use_coreml = args.backend.startswith("coreml")
    if use_coreml and "CoreMLExecutionProvider" not in available:
        raise RuntimeError("CoreML EP unavailable; do not silently substitute CPU")
    original_options = OrtInferSession._init_sess_opts
    sessions = []
    calls = []
    original_init = OrtInferSession.__init__
    original_call = OrtInferSession.__call__

    def init(instance, config):
        original_init(instance, config)
        sessions.append(instance.session)

    def session_options(config):
        options = original_options(config)
        options.add_session_config_entry("session.intra_op.allow_spinning", "0")
        options.add_session_config_entry("session.inter_op.allow_spinning", "0")
        if args.audit:
            options.enable_profiling = True
            options.profile_file_prefix = str(args.output.parent / f"{args.model}-{args.backend}-profile")
        return options

    OrtInferSession.__init__ = init
    OrtInferSession._init_sess_opts = staticmethod(session_options)
    def timed_call(instance, tensor):
        start = time.perf_counter()
        result = original_call(instance, tensor)
        calls.append({"shape": list(tensor.shape), "milliseconds": (time.perf_counter() - start) * 1000})
        return result
    OrtInferSession.__call__ = timed_call
    family, tier = args.model.split("_")
    model_name = "PP-OCR" + family
    if args.backend == "paddle_cpp":
        sys.path.insert(0, str(Path(__file__).parent / "paddle"))
        from paddle_cpp import PaddleCppSession
        import rapidocr.ch_ppocr_det.main as detector_module
        import rapidocr.ch_ppocr_rec.main as recognizer_module

        class PaddleAdapter:
            def __init__(self, config):
                task = config.task_type.value
                self.native = PaddleCppSession(args.native_models / f"{model_name}_{tier}_{task}_infer", args.threads)
                sessions.append(self)

            def __call__(self, tensor):
                start = time.perf_counter()
                result = self.native.run(tensor)
                calls.append({"shape": list(tensor.shape), "milliseconds": (time.perf_counter() - start) * 1000,
                              "paddle_cpp_ms": self.native.timings_ms.copy()})
                return result

            def have_key(self, key="character"):
                return False

            def get_providers(self):
                return ["PaddleInferenceCPU"]

            def get_inputs(self):
                return [SimpleNamespace(name="x", shape=["N", 3, "H", "W"], type="float32")]

        detector_module.get_engine = lambda engine_type: PaddleAdapter
        recognizer_module.get_engine = lambda engine_type: PaddleAdapter
    rec_directory = args.models / f"{model_name}_{tier}_rec"
    rec_config = yaml.safe_load((rec_directory / "inference.yml").read_text())
    dictionary = args.output.parent / f"{args.model}-dict.txt"
    dictionary.write_text("\n".join(rec_config["PostProcess"]["character_dict"]) + "\n")
    params = {
        "Global.use_cls": False,
        "Global.max_side_len": 4000,
        "Global.use_preprocess_img": False,
        "Global.use_vertical_padding": False,
        "Global.log_level": "warning",
        "Det.model_path": str(args.models / f"{model_name}_{tier}_det" / "inference.onnx"),
        "Rec.model_path": str(rec_directory / "inference.onnx"),
        "Rec.rec_keys_path": str(dictionary),
        "Det.ocr_version": OCRVersion(model_name),
        "Rec.ocr_version": OCRVersion(model_name),
        "Det.model_type": ModelType(tier),
        "Rec.model_type": ModelType(tier),
        "Det.limit_side_len": 960,
        "Det.limit_type": "max",
        "Det.thresh": 0.3,
        "Det.box_thresh": 0.5,
        "Det.unclip_ratio": 1.5,
        "Det.use_dilation": False,
        "Det.mean": [0.485, 0.456, 0.406],
        "Det.std": [0.229, 0.224, 0.225],
        "Rec.rec_batch_num": args.batch,
        "EngineConfig.onnxruntime.intra_op_num_threads": args.threads,
        "EngineConfig.onnxruntime.inter_op_num_threads": 1,
        "EngineConfig.onnxruntime.enable_cpu_mem_arena": False,
        "EngineConfig.onnxruntime.use_coreml": use_coreml,
        "EngineConfig.onnxruntime.coreml_ep_cfg.MLComputeUnits": (
            "CPUAndGPU" if args.backend == "coreml_gpu" else "ALL"),
        "EngineConfig.onnxruntime.coreml_ep_cfg.ModelFormat": args.coreml_format,
        "EngineConfig.onnxruntime.coreml_ep_cfg.ModelCacheDirectory": str(
            args.output.parent / "coreml-cache" / args.model / args.backend / args.coreml_format),
    }
    emit({"type": "environment", "pid": os.getpid(), "model": args.model,
          "backend": args.backend, "ort_version": ort.__version__,
          "available_providers": available, "python_import_ms": IMPORT_MS,
          "profiling": args.audit, "params": params})
    start = time.perf_counter()
    ocr = RapidOCR(params=params)
    # RapidOCR 3.10 loads models lazily; measure session creation separately.
    ocr._load_det_model()
    ocr._load_rec_model()
    emit({"type": "session", "load_ms": (time.perf_counter() - start) * 1000,
          "model": args.model, "backend": args.backend,
          "sessions": [{"providers": session.get_providers(),
                        "inputs": [{"name": x.name, "shape": x.shape, "type": x.type}
                                   for x in session.get_inputs()]}
                       for session in sessions]})
    rows = [json.loads(line) for line in args.manifest.read_text().splitlines() if line]
    for iteration in range(args.warmups + args.repeats):
        for row in rows:
            call_start = len(calls)
            start = time.perf_counter()
            image = cv2.imread(row["image"])
            decode_ms = (time.perf_counter() - start) * 1000
            infer_start = time.perf_counter()
            try:
                result = ocr(image, use_cls=False)
                ocr_ms = (time.perf_counter() - infer_start) * 1000
                texts = list(result.txts) if result.txts is not None else []
                event = {"type": "image", "id": row["id"], "model": args.model,
                         "backend": args.backend, "warmup": iteration < args.warmups,
                         "iteration": iteration - args.warmups,
                         "decode_ms": decode_ms, "ocr_ms": ocr_ms,
                         "total_ms": decode_ms + ocr_ms,
                         "line_count": len(texts), "text": "\n".join(texts),
                         "elapse": result.elapse, "error": None,
                         "engine_calls": calls[call_start:],
                         "engine_ms": sum(x["milliseconds"] for x in calls[call_start:])}
            except Exception as error:
                event = {"type": "image", "id": row["id"], "model": args.model,
                         "backend": args.backend, "warmup": iteration < args.warmups,
                         "iteration": iteration - args.warmups, "error": repr(error)}
            emit(event)
    for index, session in enumerate(sessions):
        if args.audit and args.backend != "paddle_cpp":
            emit({"type": "profile", "session": index,
                  "path": session.end_profiling()})
    output.close()


if __name__ == "__main__":
    main()
