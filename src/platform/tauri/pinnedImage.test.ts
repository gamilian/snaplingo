import { describe, expect, it, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('Tauri pinned image command adapter', () => {
  it('loads pinned image data with backend parameter name', async () => {
    const { getPinnedImage } = await import('./pinnedImage');
    invoke.mockResolvedValueOnce({
      id: 'pin-1',
      image_base64: 'base64',
      width: 10,
      height: 8,
      source_text: null,
    });

    await getPinnedImage('pin-1');

    expect(invoke).toHaveBeenCalledWith('get_pinned_image', {
      imageId: 'pin-1',
    });
  });

  it('saves a pinned image with explicit path', async () => {
    const { savePinnedImage } = await import('./pinnedImage');
    invoke.mockResolvedValueOnce(undefined);

    await savePinnedImage('pin-1', '/tmp/pin.png');

    expect(invoke).toHaveBeenCalledWith('save_pinned_image', {
      imageId: 'pin-1',
      path: '/tmp/pin.png',
    });
  });
});
