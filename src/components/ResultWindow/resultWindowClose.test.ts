import { describe, expect, it, vi } from 'vitest';
import { closeResultWindowForPresentation } from './resultWindowClose';

describe('result window close behavior', () => {
  it('hides only React state for embedded overlay result windows', () => {
    const hideResultWindow = vi.fn();
    const hideNativeWindow = vi.fn(async () => undefined);

    closeResultWindowForPresentation({
      presentation: 'overlay',
      hideResultWindow,
      hideNativeWindow,
    });

    expect(hideResultWindow).toHaveBeenCalledTimes(1);
    expect(hideNativeWindow).not.toHaveBeenCalled();
  });

  it('also hides the native window for standalone result windows', () => {
    const hideResultWindow = vi.fn();
    const hideNativeWindow = vi.fn(async () => undefined);

    closeResultWindowForPresentation({
      presentation: 'standalone',
      hideResultWindow,
      hideNativeWindow,
    });

    expect(hideResultWindow).toHaveBeenCalledTimes(1);
    expect(hideNativeWindow).toHaveBeenCalledTimes(1);
  });
});
