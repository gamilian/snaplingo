"""Compare native and React pin windows with the isolated macOS UI harness.

Build native_ui_workflows with tauri/custom-protocol first. Use its executable and an output JSON path as
arguments. Debug builds need Vite on 127.0.0.1:5189; release builds use dist.
"""
import ctypes
import json
import os
from pathlib import Path
import signal
import statistics
import subprocess
import sys
import tempfile
import time


class Usage(ctypes.Structure):
    _fields_ = [("uuid", ctypes.c_uint8 * 16)] + [
        (name, ctypes.c_uint64)
        for name in (
            "user_time", "system_time", "idle_wakeups", "interrupt_wakeups",
            "pageins", "wired_size", "resident_size", "physical_footprint",
            "start_time", "exit_time",
        )
    ]


lib = ctypes.CDLL("/usr/lib/libSystem.B.dylib", use_errno=True)
lib.proc_pid_rusage.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_void_p]
lib.responsibility_get_pid_responsible_for_pid.argtypes = [ctypes.c_int]
lib.responsibility_get_pid_responsible_for_pid.restype = ctypes.c_int

class Timebase(ctypes.Structure):
    _fields_ = [("numerator", ctypes.c_uint32), ("denominator", ctypes.c_uint32)]


timebase = Timebase()
assert lib.mach_timebase_info(ctypes.byref(timebase)) == 0
cpu_tick_ns = timebase.numerator / timebase.denominator


def usage(pid):
    result = Usage()
    if lib.proc_pid_rusage(pid, 0, ctypes.byref(result)) != 0:
        return None
    return {
        "cpu_ns": result.user_time + result.system_time,
        "physical_bytes": result.physical_footprint,
        "resident_bytes": result.resident_size,
    }


def launch(executable, arguments, log_path):
    # Disclaim the parent IDE's responsibility so helpers are attributed to this
    # fixture process. This is measurement setup, never part of the shipped app.
    attributes = ctypes.c_void_p()
    actions = ctypes.c_void_p()
    assert lib.posix_spawnattr_init(ctypes.byref(attributes)) == 0
    assert lib.responsibility_spawnattrs_setdisclaim(ctypes.byref(attributes), 1) == 0
    assert lib.posix_spawn_file_actions_init(ctypes.byref(actions)) == 0
    with open(log_path, "wb") as log:
        for descriptor in (1, 2):
            assert lib.posix_spawn_file_actions_adddup2(
                ctypes.byref(actions), log.fileno(), descriptor
            ) == 0
        argv_values = [os.fsencode(executable)] + [os.fsencode(arg) for arg in arguments]
        argv = (ctypes.c_char_p * (len(argv_values) + 1))(*argv_values, None)
        environment_values = [os.fsencode(key + "=" + value) for key, value in os.environ.items()]
        environment = (ctypes.c_char_p * (len(environment_values) + 1))(*environment_values, None)
        pid = ctypes.c_int()
        result = lib.posix_spawn(
            ctypes.byref(pid), os.fsencode(executable), ctypes.byref(actions),
            ctypes.byref(attributes), argv, environment,
        )
    lib.posix_spawn_file_actions_destroy(ctypes.byref(actions))
    lib.posix_spawnattr_destroy(ctypes.byref(attributes))
    if result:
        raise OSError(result, "Could not start native UI fixture")
    return pid.value


def process_group(root):
    output = subprocess.check_output(["ps", "-axo", "pid=,comm="], text=True)
    result = {}
    for line in output.splitlines():
        pid_text, name = line.strip().split(None, 1)
        pid = int(pid_text)
        if pid == root or lib.responsibility_get_pid_responsible_for_pid(pid) == root:
            value = usage(pid)
            if value is not None:
                result[pid] = {"name": name, **value}
    return result


def messages(path):
    rows = []
    for line in Path(path).read_text(errors="replace").splitlines():
        if line.startswith("{"):
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    return rows


def measure(executable, renderer, pins, repetition, directory):
    log_path = Path(directory) / f"{renderer}-{pins}-{repetition}.log"
    root = launch(executable, ["--benchmark", renderer, "--pins", str(pins), "--seconds", "14"], log_path)
    reaped = False
    try:
        time.sleep(4)
        assert lib.responsibility_get_pid_responsible_for_pid(root) == root
        metadata = messages(log_path)
        assert any(row.get("mode") == renderer and row.get("pins") == pins for row in metadata), log_path.read_text()
        if renderer == "webview":
            ready = [row for row in metadata if row.get("imageWidth") == 800 and row.get("toolbarButtons") == 3]
            assert len(ready) == pins, "React pin windows did not finish loading: " + log_path.read_text()
        previous = process_group(root)
        previous_time = time.monotonic()
        samples = []
        for _ in range(8):
            time.sleep(0.75)
            current = process_group(root)
            now = time.monotonic()
            cpu = sum(max(0, value["cpu_ns"] - previous.get(pid, value)["cpu_ns"]) for pid, value in current.items())
            samples.append({
                "physical_bytes": sum(value["physical_bytes"] for value in current.values()),
                "main_physical_bytes": current[root]["physical_bytes"],
                "cpu_percent": cpu * cpu_tick_ns / 1e9 / (now - previous_time) * 100,
                "helper_count": len(current) - 1,
            })
            previous, previous_time = current, now
        _, status = os.waitpid(root, 0)
        reaped = True
        assert os.waitstatus_to_exitcode(status) == 0, log_path.read_text()
        return {
            "renderer": renderer, "pins": pins, "repetition": repetition,
            "peak_physical_bytes": max(row["physical_bytes"] for row in samples),
            "mean_physical_bytes": statistics.mean(row["physical_bytes"] for row in samples),
            "mean_cpu_percent": statistics.mean(row["cpu_percent"] for row in samples),
            "peak_cpu_percent": max(row["cpu_percent"] for row in samples),
            "helper_count": max(row["helper_count"] for row in samples),
            "processes": {str(pid): value["name"] for pid, value in previous.items()},
            "samples": samples,
        }
    finally:
        if not reaped:
            try:
                os.kill(root, signal.SIGTERM)
            except ProcessLookupError:
                pass
            os.waitpid(root, 0)


def main():
    executable, output = map(Path, sys.argv[1:])
    rows = []
    with tempfile.TemporaryDirectory(prefix="snaplingo-ui-profile-") as directory:
        for repetition in range(1, 4):
            for pins in (1, 5):
                for renderer in ("native", "webview"):
                    row = measure(executable.resolve(), renderer, pins, repetition, directory)
                    rows.append(row)
                    output.parent.mkdir(parents=True, exist_ok=True)
                    output.write_text(json.dumps(rows, indent=2) + "\n")
                    print(json.dumps({key: value for key, value in row.items() if key not in ("processes", "samples")}), flush=True)


if __name__ == "__main__":
    main()
