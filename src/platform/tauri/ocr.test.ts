import { describe, expect, it, vi } from 'vitest';

const { invoke, open } = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open }));

describe('Tauri OCR command adapter', () => {
  it('recognizes in-memory image bytes', async () => {
    const { recognizeImageData } = await import('./ocr');
    invoke.mockResolvedValueOnce({ text: 'recognized' });
    await recognizeImageData(new Uint8Array([1, 2, 3]));
    expect(invoke).toHaveBeenCalledWith('recognize_image', {
      request: { image_data: [1, 2, 3], language: null },
    });
  });

  it('returns null when image selection is cancelled', async () => {
    const { selectImageFile } = await import('./ocr');
    open.mockResolvedValueOnce(null);
    await expect(selectImageFile()).resolves.toBeNull();
  });
});
