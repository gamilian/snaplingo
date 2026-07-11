import { describe, expect, it } from 'vitest';
import {
  subscribeCaptureCancelRequests,
  subscribeCaptureCopyRequests,
  type CaptureCancelRequestListener,
} from './captureCancelRequest';

describe('capture cancel request', () => {
  it('invokes cancel when the native capture cancel event is received', async () => {
    const calls: string[] = [];
    const handlers: Array<() => void> = [];

    const listen: CaptureCancelRequestListener = async (nextHandler) => {
      calls.push('listen');
      handlers.push(nextHandler);
      return () => {
        calls.push('unlisten');
      };
    };

    const unlisten = await subscribeCaptureCancelRequests(
      () => {
        calls.push('cancel');
      },
      listen,
    );

    handlers[0]?.();
    unlisten();

    expect(calls).toEqual([
      'listen',
      'cancel',
      'unlisten',
    ]);
  });

  it('invokes copy when the native capture copy event is received', async () => {
    const calls: string[] = [];
    const handlers: Array<() => void> = [];

    const listen: CaptureCancelRequestListener = async (nextHandler) => {
      calls.push('listen');
      handlers.push(nextHandler);
      return () => {
        calls.push('unlisten');
      };
    };

    const unlisten = await subscribeCaptureCopyRequests(
      () => {
        calls.push('copy');
      },
      listen,
    );

    handlers[0]?.();
    unlisten();

    expect(calls).toEqual([
      'listen',
      'copy',
      'unlisten',
    ]);
  });
});
