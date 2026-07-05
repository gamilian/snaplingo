import { describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();
const open = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open }));

describe('ocr tauri adapter', () => {
  it('recognizes in-memory image bytes through the native OCR command', async () => {
    const { recognizeImageData } = await import('../ocr');
    invoke.mockResolvedValueOnce({ text: 'recognized' });

    await recognizeImageData(new Uint8Array([1, 2, 3]));

    expect(invoke).toHaveBeenCalledWith('recognize_image', {
      request: {
        image_data: [1, 2, 3],
        language: null,
      },
    });
  });

  it('recognizes selected image files by path', async () => {
    const { recognizeImageFile } = await import('../ocr');
    invoke.mockResolvedValueOnce({ text: 'recognized' });

    await recognizeImageFile('/tmp/image.png');

    expect(invoke).toHaveBeenCalledWith('recognize_image_file', {
      path: '/tmp/image.png',
    });
  });

  it('returns null when image selection is cancelled', async () => {
    const { selectImageFile } = await import('../ocr');
    open.mockResolvedValueOnce(null);

    await expect(selectImageFile()).resolves.toBeNull();
  });
});
