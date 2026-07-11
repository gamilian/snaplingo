import { describe, expect, it, vi } from 'vitest';
import { createPinnedImagePlatformRuntime } from './platformRuntime';

describe('pinned image platform runtime', () => {
  it('translates pinned image window actions into portable window calls', async () => {
    const window = {
      resize: vi.fn(async () => undefined),
      moveBy: vi.fn(async () => undefined),
      startDragging: vi.fn(async () => undefined),
      close: vi.fn(async () => undefined),
    };
    const runtime = createPinnedImagePlatformRuntime({ window });

    await runtime.resizeTo(320, 240);
    await runtime.moveBy(12, -8);
    await runtime.beginDrag();
    await runtime.dismiss();

    expect(window.resize).toHaveBeenCalledWith(320, 240);
    expect(window.moveBy).toHaveBeenCalledWith(12, -8);
    expect(window.startDragging).toHaveBeenCalledTimes(1);
    expect(window.close).toHaveBeenCalledTimes(1);
  });
});
