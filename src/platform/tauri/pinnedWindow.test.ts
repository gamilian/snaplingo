import { beforeEach, describe, expect, it, vi } from 'vitest';

const {
  close,
  getCurrentWebviewWindow,
  getCurrentWindow,
  outerPosition,
  outerSize,
  scaleFactor,
  setPosition,
  setSize,
  startDragging,
} = vi.hoisted(() => {
  const close = vi.fn();
  const outerPosition = vi.fn();
  const outerSize = vi.fn();
  const scaleFactor = vi.fn();
  const setPosition = vi.fn();
  const setSize = vi.fn();
  const startDragging = vi.fn();

  return {
    close,
    getCurrentWebviewWindow: vi.fn(() => ({ close })),
    getCurrentWindow: vi.fn(() => ({
      outerPosition,
      outerSize,
      scaleFactor,
      setPosition,
      setSize,
      startDragging,
    })),
    outerPosition,
    outerSize,
    scaleFactor,
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
    outerSize.mockReset().mockResolvedValue({ width: 300, height: 200 });
    scaleFactor.mockReset().mockResolvedValue(1);
    setPosition.mockReset().mockResolvedValue(undefined);
    setSize.mockReset().mockResolvedValue(undefined);
    startDragging.mockReset().mockResolvedValue(undefined);
  });

  it('resizes the current pinned window from portable dimensions', async () => {
    await pinnedWindow.resize(320, 180);

    expect(setSize).toHaveBeenCalledWith({ width: 320, height: 180 });
    expect(setPosition).toHaveBeenCalledWith({ x: 90, y: 210 });
  });

  it('moves the current pinned window by a portable delta', async () => {
    await pinnedWindow.moveBy(12, -24);

    expect(setPosition).toHaveBeenCalledWith({ x: 112, y: 176 });
  });

  it('preserves the physical center on a high-DPI display', async () => {
    scaleFactor.mockResolvedValue(2);
    outerSize.mockResolvedValue({ width: 600, height: 400 });
    await pinnedWindow.resize(400, 300);
    expect(setPosition).toHaveBeenCalledWith({ x: 0, y: 100 });
  });

  it('serializes rapid zoom updates so each resize uses the preceding window size', async () => {
    let finish!: () => void;
    setSize.mockImplementationOnce(() => new Promise<void>((resolve) => { finish = resolve; }));
    outerSize.mockResolvedValueOnce({ width: 300, height: 200 }).mockResolvedValue({ width: 320, height: 180 });
    outerPosition.mockResolvedValueOnce({ x: 100, y: 200 }).mockResolvedValue({ x: 90, y: 210 });
    const first = pinnedWindow.resize(320, 180);
    const second = pinnedWindow.resize(640, 360);
    await vi.waitFor(() => expect(setSize).toHaveBeenCalledOnce());
    finish();
    await Promise.all([first, second]);
    expect(setPosition.mock.calls).toEqual([[{ x: 90, y: 210 }], [{ x: -70, y: 120 }]]);
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
