import { beforeEach, describe, expect, it, vi } from 'vitest';

const {
  close,
  getCurrentWebviewWindow,
  getCurrentWindow,
  outerPosition,
  setPosition,
  setSize,
  startDragging,
} = vi.hoisted(() => {
  const close = vi.fn();
  const outerPosition = vi.fn();
  const setPosition = vi.fn();
  const setSize = vi.fn();
  const startDragging = vi.fn();

  return {
    close,
    getCurrentWebviewWindow: vi.fn(() => ({ close })),
    getCurrentWindow: vi.fn(() => ({
      outerPosition,
      setPosition,
      setSize,
      startDragging,
    })),
    outerPosition,
    setPosition,
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
  PhysicalPosition: class PhysicalPosition {
    constructor(
      public x: number,
      public y: number,
    ) {}
  },
}));

vi.mock('@tauri-apps/api/webviewWindow', () => ({
  getCurrentWebviewWindow,
}));

import { pinnedWindow } from './pinnedWindow';

describe('Tauri pinned window adapter', () => {
  beforeEach(() => {
    close.mockReset().mockResolvedValue(undefined);
    outerPosition.mockReset().mockResolvedValue({ x: 100, y: 200 });
    setPosition.mockReset().mockResolvedValue(undefined);
    setSize.mockReset().mockResolvedValue(undefined);
    startDragging.mockReset().mockResolvedValue(undefined);
  });

  it('resizes the current pinned window from portable dimensions', async () => {
    await pinnedWindow.resize(320, 180);

    expect(setSize).toHaveBeenCalledWith({ width: 320, height: 180 });
  });

  it('moves the current pinned window by a portable delta', async () => {
    await pinnedWindow.moveBy(12, -24);

    expect(setPosition).toHaveBeenCalledWith({ x: 112, y: 176 });
  });

  it('starts dragging the current pinned window', async () => {
    await pinnedWindow.startDragging();

    expect(startDragging).toHaveBeenCalledOnce();
  });

  it('closes the current pinned webview window', async () => {
    await pinnedWindow.close();

    expect(close).toHaveBeenCalledOnce();
  });
});
