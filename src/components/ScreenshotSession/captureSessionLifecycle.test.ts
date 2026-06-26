import { describe, expect, it } from 'vitest';
import {
  closeInactiveCaptureSession,
  finishCaptureSession,
  hideInactiveCaptureWindow,
  type CaptureLifecycleClient,
} from './captureSessionLifecycle';

describe('capture session lifecycle', () => {
  it('closes the capture window before ending the native session for immediate Esc feedback', async () => {
    const events: string[] = [];
    const client: CaptureLifecycleClient = {
      cancelCaptureSession: async (sessionId) => {
        events.push(`cancel_capture_session:${sessionId}`);
      },
    };

    await finishCaptureSession({
      client,
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
      'close-start',
      'close-end',
      'cancel_capture_session:session-1',
    ]);
  });

  it('clears the capture UI before ending the native session when no inactive handler is available', async () => {
    const events: string[] = [];
    const client: CaptureLifecycleClient = {
      cancelCaptureSession: async (sessionId) => {
        events.push(`cancel_capture_session:${sessionId}`);
      },
    };

    await finishCaptureSession({
      client,
      sessionId: 'session-2',
      resetSessionState: () => {
        events.push('reset');
      },
    });

    expect(events).toEqual(['reset', 'cancel_capture_session:session-2']);
  });

  it('does not end the native session if closing the capture window fails', async () => {
    const events: string[] = [];
    const client: CaptureLifecycleClient = {
      cancelCaptureSession: async (sessionId) => {
        events.push(`cancel_capture_session:${sessionId}`);
      },
    };

    await expect(
      finishCaptureSession({
        client,
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

    expect(events).toEqual(['close-start']);
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

  it('hides the capture window for reuse when it becomes inactive', async () => {
    const events: string[] = [];

    await hideInactiveCaptureWindow(
      {
        hide: async () => {
          events.push('window.hide');
        },
      },
      {
        hideCaptureWindow: async () => {
          events.push('hide_capture_window');
        },
      },
    );

    expect(events).toEqual(['hide_capture_window']);
  });
});
