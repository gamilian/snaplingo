import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { describe, expect, it } from 'vitest';
import { cleanupOcrResources } from './prepare-ocr.mjs';

const manifest = JSON.parse(await readFile(new URL('./ocr-assets.json', import.meta.url), 'utf8'));
const platforms = ['darwin-arm64', 'darwin-x64', 'win32-x64', 'linux-x64'];

describe('OCR target resources', () => {
  it.each(platforms)('pins the same small MNN models and dictionary for %s', platform => {
    const models = manifest.models.filter(asset => asset.platforms.includes(platform));
    expect(models.map(asset => asset.file)).toEqual(expect.arrayContaining(['small_det.mnn', 'small_rec.mnn', 'small_keys.txt']));
    expect(models.some(asset => /tiny|medium|\.onnx$/.test(asset.file))).toBe(false);
    const dictionary = models.find(asset => asset.file === 'small_keys.txt');
    expect(dictionary.url).toContain('/664a945953e46186b37203fa4902a20d60a32809/models/ppocr_keys_v6_small.txt');
    expect(dictionary.sha256).toBe('b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d');
    expect(manifest.archives.filter(asset => asset.platform === platform || asset.platforms?.includes(platform))
      .flatMap(asset => Object.values(asset.files)).some(file => String(file).includes('runtime/'))).toBe(false);
    expect(models).toEqual(manifest.models.filter(asset => asset.platforms.includes(platforms[0])));
  });

  it.each(platforms)('removes retired OCR assets without deleting other files for %s', async platform => {
    const root = await mkdtemp(join(tmpdir(), 'snaplingo-ocr-assets-'));
    const retired = ['medium_det.mnn', 'medium_rec.mnn', 'medium_det.onnx', 'medium_rec.onnx',
      'tiny_det.mnn', 'tiny_rec.mnn', 'tiny_keys.txt', 'small_det.onnx', 'small_rec.onnx', 'keys.txt',
      'runtime/libonnxruntime.dylib', 'runtime/libonnxruntime.so', 'runtime/onnxruntime.dll',
      'runtime/onnxruntime_providers_shared.dll', 'runtime/DirectML.dll',
      'licenses/onnxruntime.txt', 'licenses/onnxruntime-third-party.txt', 'licenses/directml.txt'];
    const retained = ['small_det.mnn', 'small_rec.mnn', 'small_keys.txt', 'licenses/ocr-rs.txt',
      'licenses/mnn/LICENSE-MNN.txt', 'runtime/custom-engine.dylib', 'licenses/custom-license.txt', 'personal-note.txt'];
    try {
      await writeFile(join(root, '.platform'), platform);
      for (const file of [...retired, ...retained]) {
        await mkdir(dirname(join(root, file)), { recursive: true });
        await writeFile(join(root, file), file);
      }
      await cleanupOcrResources(platform, root);
      for (const file of retired) expect(existsSync(join(root, file)), file).toBe(false);
      for (const file of retained) expect(await readFile(join(root, file), 'utf8')).toBe(file);
      await cleanupOcrResources(platform, root);
      expect(await readFile(join(root, '.platform'), 'utf8')).toBe(platform);
    } finally { await rm(root, { recursive: true, force: true }); }
  });
});
