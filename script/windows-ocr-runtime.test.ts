import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { describe, expect, it } from 'vitest';
import { copyWindowsCrt } from './windows-ocr-runtime.mjs';

describe('Windows application-local OCR runtime', () => {
  it('deploys the required CRT and additional runtime DLLs together', async () => {
    const root = await mkdtemp(join(tmpdir(), 'snaplingo-crt-'));
    try {
      const source = join(root, 'source');
      const destination = join(root, 'application');
      await mkdir(source);
      for (const name of ['vcruntime140.dll', 'vcruntime140_1.dll', 'msvcp140.dll', 'msvcp140_1.dll', 'msvcp140_atomic_wait.dll']) {
        await writeFile(join(source,name), name);
      }
      await copyWindowsCrt(source, destination);
      expect(await readFile(join(destination,'msvcp140_atomic_wait.dll'),'utf8')).toBe('msvcp140_atomic_wait.dll');
      expect(await readFile(join(destination,'vcruntime140_1.dll'),'utf8')).toBe('vcruntime140_1.dll');
    } finally { await rm(root, { recursive: true, force: true }); }
  });

  it('rejects an incomplete redistributable before creating an installer payload', async () => {
    const root = await mkdtemp(join(tmpdir(), 'snaplingo-crt-'));
    try {
      await expect(copyWindowsCrt(root,join(root,'application'))).rejects.toThrow('missing vcruntime140.dll');
    } finally { await rm(root, { recursive: true, force: true }); }
  });
});
