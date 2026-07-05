import { beforeEach, describe, expect, it, vi } from 'vitest';

const writeText = vi.fn();

describe('clipboard adapter', () => {
  beforeEach(() => {
    writeText.mockReset();
    Object.defineProperty(globalThis.navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
  });

  it('writes text through the browser clipboard seam', async () => {
    const { writeClipboardText } = await import('../clipboard');

    await writeClipboardText('hello');

    expect(writeText).toHaveBeenCalledWith('hello');
  });
});
