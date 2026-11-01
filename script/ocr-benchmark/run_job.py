"""Run one isolated benchmark process and sample its resource use."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import time

import psutil


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=float, default=600)
    parser.add_argument("--max-rss-mb", type=float, default=6000)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    args.output.parent.mkdir(parents=True, exist_ok=True)
    # A preparation step may pause the matrix between jobs, never during a measurement.
    while (args.output.parent / "PAUSE").exists():
        time.sleep(1)
    start = time.perf_counter()
    samples = []
    timed_out = False
    memory_limited = False
    with args.output.with_suffix(".stdout.log").open("w") as stdout, \
            args.output.with_suffix(".stderr.log").open("w") as stderr:
        process = subprocess.Popen(command, stdout=stdout, stderr=stderr, start_new_session=True)
        observed = psutil.Process(process.pid)
        observed_processes = {observed.pid: observed}
        while process.poll() is None:
            try:
                processes = [observed] + observed.children(recursive=True)
                rss = cpu = 0
                for candidate in processes:
                    child = observed_processes.setdefault(candidate.pid, candidate)
                    rss += child.memory_info().rss
                    cpu += child.cpu_percent()
                samples.append({"elapsed_s": time.perf_counter() - start,
                                "rss_bytes": rss, "cpu_percent": cpu})
                memory_limited = rss > args.max_rss_mb * 1_000_000
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                pass
            if time.perf_counter() - start > args.timeout or memory_limited:
                timed_out = not memory_limited
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                break
            time.sleep(0.05)
        code = process.wait()
    result = {"command": command, "exit_code": code, "timed_out": timed_out,
              "memory_limited": memory_limited, "rss_limit_mb": args.max_rss_mb,
              "wall_seconds": time.perf_counter() - start,
              "peak_sampled_rss_bytes": max((x["rss_bytes"] for x in samples), default=0),
              "sample_interval_seconds": 0.05, "samples": samples,
              "memory_scope": "sum of process tree RSS; macOS shared GPU and system CoreML service memory is not isolated VRAM"}
    args.output.write_text(json.dumps(result, indent=2))
    print(json.dumps({k: v for k, v in result.items() if k not in ("samples", "command")}))


if __name__ == "__main__":
    main()
