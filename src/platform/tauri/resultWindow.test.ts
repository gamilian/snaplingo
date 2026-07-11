import { beforeEach, describe, expect, it, vi } from 'vitest';

const { getCurrentWindow, hide, setSize, startDragging } = vi.hoisted(() => {
  const hide = vi.fn();
  const setSize = vi.fn();
  const startDragging = vi.fn();

  return {
    getCurrentWindow: vi.fn(() => ({ hide, setSize, startDragging })),
    hide,
    setSize,
    startDragging,
  };
});

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow,
  LogicalSize: class LogicalSize {
    constructor(
      public width: number,
      public height: number,
    ) {}
  },
}));

import { resultWindow } from './resultWindow';

describe('Tauri result window adapter', () => {
  beforeEach(() => {
    getCurrentWindow.mockClear();
    hide.mockReset().mockResolvedValue(undefined);
    setSize.mockReset().mockResolvedValue(undefined);
    startDragging.mockReset().mockResolvedValue(undefined);
  });

  it('resizes the current result window from portable dimensions', async () => {
    await resultWindow.resize(660, 480);

    expect(setSize).toHaveBeenCalledWith({ width: 660, height: 480 });
  });

  it('hides the current result window instead of closing it', async () => {
    await resultWindow.hide();

    expect(hide).toHaveBeenCalledOnce();
  });

  it('starts dragging the current result window', async () => {
    await resultWindow.startDragging();

    expect(startDragging).toHaveBeenCalledOnce();
  });
});
