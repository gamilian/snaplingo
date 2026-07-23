import { describe, expect, it, vi } from 'vitest';

import { createCapturePointerFrameDispatcher } from './capturePointerFrame';

describe('capture pointer frame dispatch', () => {
  it('publishes only the latest pointer point in a display frame', () => {
    const frames: FrameRequestCallback[] = [];
    const move = vi.fn();
    const dispatcher = createCapturePointerFrameDispatcher({
      requestFrame: (callback) => {
        frames.push(callback);
        return frames.length;
      },
      cancelFrame: vi.fn(),
      move,
    });

    dispatcher.schedule({ point: { x: 10, y: 20 }, source: 'root' });
    dispatcher.schedule({ point: { x: 30, y: 40 }, source: 'root' });

    expect(frames).toHaveLength(1);
    expect(move).not.toHaveBeenCalled();

    frames[0](16);

    expect(move).toHaveBeenCalledOnce();
    expect(move).toHaveBeenCalledWith({
      point: { x: 30, y: 40 },
      source: 'root',
    });
  });

  it('flushes the latest pointer point before pointer release', () => {
    const move = vi.fn();
    const cancelFrame = vi.fn();
    const dispatcher = createCapturePointerFrameDispatcher({
      requestFrame: () => 7,
      cancelFrame,
      move,
    });

    dispatcher.schedule({ point: { x: 130, y: 90 }, source: 'root' });
    dispatcher.flush();

    expect(cancelFrame).toHaveBeenCalledWith(7);
    expect(move).toHaveBeenCalledWith({
      point: { x: 130, y: 90 },
      source: 'root',
    });
  });
});
