"""Measure tiny MNN lifetime policies serially; use generated public fixtures only."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument('--binary', type=Path, required=True)
parser.add_argument('--models', type=Path, required=True)
parser.add_argument('--images', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
manifest = json.loads((Path(__file__).resolve().parents[1] / 'mnn/weights-manifest.json').read_text())
files = {'PP-OCRv6_tiny_det.mnn', 'PP-OCRv6_tiny_rec.mnn', 'ppocr_keys_v6_tiny.txt'}
verified = []
for row in manifest['files']:
    if row['file'] not in files:
        continue
    data = (args.models / row['file']).read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    assert len(data) == row['bytes'] and digest == row['sha256'], row['file']
    verified.append({'file': row['file'], 'bytes': len(data), 'sha256': digest})
assert len(verified) == 3
(args.output / 'models.json').write_text(json.dumps(verified, indent=2) + '\n')


def run(image, mode, repeats, threads=4, parallel=True):
    started = time.perf_counter()
    result = subprocess.run([
        str(args.binary.resolve()), str(args.models.resolve()),
        str((args.images / f'{image}.png').resolve()), mode, str(repeats),
        str(threads), str(parallel).lower(),
    ], text=True, capture_output=True, check=True)
    elapsed = (time.perf_counter() - started) * 1000
    events = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
    start = next(row for row in events if row['type'] == 'start')
    assert start['threads'] == threads and start['parallel'] == parallel
    return result, elapsed, events


for image in ('small-light', 'document-light'):
    for mode in ('cached', 'per-request'):
        result, _, _ = run(image, mode, 5)
        (args.output / f'{image}-{mode}.jsonl').write_text(result.stdout)
        (args.output / f'{image}-{mode}.stderr').write_text(result.stderr)

workers = []
for image in ('small-light', 'document-light'):
    for iteration in range(10):
        _, elapsed, events = run(image, 'worker', 1)
        workers.append({'image': image, 'iteration': iteration,
                        'process_total_ms': elapsed, 'events': events})
(args.output / 'process-per-request.json').write_text(json.dumps(workers, indent=2) + '\n')

comparison = []
for threads, parallel in ((4, False), (2, False), (4, True)):
    for image in ('small-light', 'document-light'):
        for iteration in range(5):
            _, elapsed, events = run(image, 'worker', 1, threads, parallel)
            comparison.append({'image': image, 'threads': threads, 'parallel': parallel,
                               'iteration': iteration, 'process_total_ms': elapsed,
                               'events': events})
(args.output / 'parallel-process-comparison.json').write_text(json.dumps(comparison, indent=2) + '\n')
print(f'Wrote verified model metadata and serial benchmark results to {args.output}')
