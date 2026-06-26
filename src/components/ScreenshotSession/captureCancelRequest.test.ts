import { describe, expect, it } from 'vitest';
import {
  CAPTURE_CANCEL_REQUESTED_EVENT,
  subscribeCaptureCancelRequests,
  type CaptureCancelRequestListener,
} from './captureCancelRequest';

describe('capture cancel request', () => {
  it('invokes cancel when the native capture cancel event is received', async () => {
    const calls: string[] = [];
    const handlers: Array<() => void> = [];

    const listen: CaptureCancelRequestListener = async (eventName, nextHandler) => {
      calls.push(`listen:${eventName}`);
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
      `listen:${CAPTURE_CANCEL_REQUESTED_EVENT}`,
      'cancel',
      'unlisten',
    ]);
  });
});
