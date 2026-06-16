import { describe, expect, it } from 'vitest';
import {
  type PinInvoke,
  type PinInvokeArgs,
  savePinnedImage,
} from './pinActions';

describe('pinned image actions', () => {
  it('saves a pinned image to the default capture path', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const invoke: PinInvoke = async <T>(
      command: string,
      args?: PinInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      if (command === 'default_capture_save_path') {
        return '/tmp/SnapLingo-20260617-023000.png' as T;
      }
      return undefined as T;
    };

    await savePinnedImage(invoke, 'pin-1');

    expect(calls).toEqual([
      { command: 'default_capture_save_path', args: undefined },
      {
        command: 'save_pinned_image',
        args: {
          imageId: 'pin-1',
          path: '/tmp/SnapLingo-20260617-023000.png',
        },
      },
    ]);
  });
});
