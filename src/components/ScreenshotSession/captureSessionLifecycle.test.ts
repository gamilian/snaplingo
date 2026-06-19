import { describe, expect, it } from 'vitest';
import {
  closeInactiveCaptureSession,
  finishCaptureSession,
  type CaptureLifecycleInvoke,
} from './captureSessionLifecycle';

describe('capture session lifecycle', () => {
  it('waits for the inactive handler instead of clearing the visible capture UI', async () => {
    const events: string[] = [];
    const invoke: CaptureLifecycleInvoke = async (command, args) => {
      events.push(`${command}:${String(args?.sessionId)}`);
      return undefined;
    };

    await finishCaptureSession({
      invoke,
      sessionId: 'session-1',
      onInactive: async () => {
        events.push('close-start');
        await Promise.resolve();
        events.push('close-end');
      },
      resetSessionState: () => {
        events.push('reset');
      },
    });

    expect(events).toEqual([
      'cancel_capture_session:session-1',
      'close-start',
      'close-end',
    ]);
  });

  it('clears the capture UI only when no inactive handler is available', async () => {
    const events: string[] = [];
    const invoke: CaptureLifecycleInvoke = async (command, args) => {
      events.push(`${command}:${String(args?.sessionId)}`);
      return undefined;
    };

    await finishCaptureSession({
      invoke,
      sessionId: 'session-2',
      resetSessionState: () => {
        events.push('reset');
      },
    });

    expect(events).toEqual(['cancel_capture_session:session-2', 'reset']);
  });

  it('does not clear state if closing the capture window fails', async () => {
    const events: string[] = [];
    const invoke: CaptureLifecycleInvoke = async (command, args) => {
      events.push(`${command}:${String(args?.sessionId)}`);
      return undefined;
    };

    await expect(
      finishCaptureSession({
        invoke,
        sessionId: 'session-3',
        onInactive: async () => {
          events.push('close-start');
          throw new Error('close failed');
        },
        resetSessionState: () => {
          events.push('reset');
        },
      }),
    ).rejects.toThrow('close failed');

    expect(events).toEqual(['cancel_capture_session:session-3', 'close-start']);
  });

  it('uses the same close-before-reset fallback for explicit cancellation', async () => {
    const events: string[] = [];

    await closeInactiveCaptureSession({
      onInactive: async () => {
        events.push('close');
      },
      resetSessionState: () => {
        events.push('reset');
      },
    });

    expect(events).toEqual(['close']);
  });
});
