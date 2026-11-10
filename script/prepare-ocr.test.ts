import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { describe, expect, it } from 'vitest';
import { cleanupOcrResources } from './prepare-ocr.mjs';

const manifest = JSON.parse(await readFile(new URL('./ocr-assets.json', import.meta.url), 'utf8'));

describe('OCR target resources', () => {
  it.each(['darwin-arm64', 'darwin-x64'])('pins tiny models and their matching dictionary for %s', platform => {
    const models = manifest.models.filter(asset => asset.platforms.includes(platform));
    expect(models.map(asset => asset.file)).toEqual(expect.arrayContaining(['tiny_det.mnn', 'tiny_rec.mnn', 'tiny_keys.txt']));
    expect(models.some(asset => /medium|\.onnx$/.test(asset.file))).toBe(false);
    const dictionary = models.find(asset => asset.file === 'tiny_keys.txt');
    expect(dictionary.url).toContain('/664a945953e46186b37203fa4902a20d60a32809/models/ppocr_keys_v6_tiny.txt');
    expect(dictionary.sha256).toBe('c5cbe34ef40c29c4df07ed012bf96569cb69a2d2a01a07027e9f13cb832bd9cd');
    expect(manifest.archives.filter(asset => asset.platform === platform || asset.platforms?.includes(platform))
      .flatMap(asset => Object.values(asset.files)).some(file => String(file).includes('runtime/'))).toBe(false);
  });

  it('removes retired assets on the same platform without deleting other files', async () => {
    const root = await mkdtemp(join(tmpdir(), 'snaplingo-ocr-assets-'));
    const retired = ['medium_det.mnn', 'medium_rec.mnn', 'medium_det.onnx', 'medium_rec.onnx',
      'keys.txt', 'runtime/libonnxruntime.dylib', 'licenses/onnxruntime.txt', 'licenses/onnxruntime-third-party.txt'];
    const retained = ['tiny_det.mnn', 'tiny_rec.mnn', 'tiny_keys.txt', 'licenses/ocr-rs.txt',
      'licenses/mnn/LICENSE-MNN.txt', 'runtime/custom-engine.dylib', 'licenses/custom-license.txt', 'personal-note.txt'];
    try {
      await writeFile(join(root, '.platform'), 'darwin-arm64');
      for (const file of [...retired, ...retained]) {
        await mkdir(dirname(join(root, file)), { recursive: true });
        await writeFile(join(root, file), file);
      }
      await cleanupOcrResources('darwin-arm64', root);
      for (const file of retired) expect(existsSync(join(root, file)), file).toBe(false);
      for (const file of retained) expect(await readFile(join(root, file), 'utf8')).toBe(file);
      await cleanupOcrResources('darwin-arm64', root);
      expect(await readFile(join(root, '.platform'), 'utf8')).toBe('darwin-arm64');
    } finally { await rm(root, { recursive: true, force: true }); }
  });

  it.each(['win32-x64', 'linux-x64'])('preserves medium models and their dictionary for %s', async platform => {
    const models = manifest.models.filter(asset => asset.platforms.includes(platform));
    expect(models.map(asset => asset.file)).toEqual(expect.arrayContaining(['medium_det.onnx', 'medium_rec.onnx', 'keys.txt']));
    expect(models.find(asset => asset.file === 'keys.txt').url).toContain('ppocr_keys_v6_medium.txt');
    const root = await mkdtemp(join(tmpdir(), 'snaplingo-ocr-assets-'));
    try {
      const runtime = platform === 'win32-x64' ? 'runtime/onnxruntime.dll' : 'runtime/libonnxruntime.so';
      for (const file of ['medium_det.onnx', 'medium_rec.onnx', 'keys.txt', runtime, 'tiny_det.mnn']) {
        await mkdir(dirname(join(root, file)), { recursive: true });
        await writeFile(join(root, file), file);
      }
      await cleanupOcrResources(platform, root);
      for (const file of ['medium_det.onnx', 'medium_rec.onnx', 'keys.txt', runtime]) expect(existsSync(join(root, file)), file).toBe(true);
      expect(existsSync(join(root, 'tiny_det.mnn'))).toBe(false);
    } finally { await rm(root, { recursive: true, force: true }); }
  });
});
