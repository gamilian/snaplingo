"""Compatibility and GPU-provider probes; these are not full latency rankings."""
import subprocess
import sys
from pathlib import Path

script = Path(__file__).resolve().parent
work = Path("/tmp/snaplingo-ocr-benchmark")
output = work / "results" / "compat"
output.mkdir(parents=True, exist_ok=True)
jobs = []
old_python = work / "ort123" / "bin" / "python"
for model in ("v6_tiny", "v6_small", "v6_medium", "v5_mobile", "v5_server"):
    jobs.append((f"ort123-{model}-cpu-probe", str(old_python), model, "cpu", "MLProgram", False))
jobs.append(("ort-v6_medium-coreml-audit", sys.executable, "v6_medium", "coreml_gpu", "MLProgram", True))
for model in ("v6_small", "v5_server"):
    jobs.append((f"ort-{model}-coreml-neuralnetwork-probe", sys.executable, model,
                 "coreml_gpu", "NeuralNetwork", False))
for name, python, model, backend, model_format, audit in jobs:
    resource = output / f"{name}.resources.json"
    if resource.exists():
        continue
    print("running " + name, flush=True)
    command = [python, str(script / "rapid_bench.py"), "--model", model, "--backend", backend,
               "--coreml-format", model_format, "--models", str(work / "onnx-models"),
               "--manifest", str(work / "dataset" / "probe.jsonl"),
               "--output", str(output / f"{name}.jsonl"), "--warmups", "0", "--repeats", "1"]
    if audit:
        command.append("--audit")
    subprocess.run([sys.executable, str(script / "run_job.py"), "--output", str(resource),
                    "--timeout", "180", "--"] + command, check=True)
print("postchecks complete", flush=True)
