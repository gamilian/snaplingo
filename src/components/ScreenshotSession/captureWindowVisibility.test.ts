import { describe, expect, it } from 'vitest';
import {
  getCaptureWindowRevealPermissions,
  revealCaptureWindow,
  revealCaptureWindowForSession,
  waitForCaptureSurfacePaint,
  shouldRevealCaptureWindow,
  type CaptureWindowHandle,
} from './captureWindowVisibility';

describe('capture window visibility', () => {
  it('keeps the capture window hidden while the session snapshot is loading', () => {
    expect(
      shouldRevealCaptureWindow({
        status: 'loading',
        hasSession: false,
        hasCaptureImagesReady: false,
        hasRevealed: false,
      }),
    ).toBe(false);
  });

  it('reveals the capture window once metadata is ready even before image hydration', () => {
    expect(
      shouldRevealCaptureWindow({
        status: 'selecting',
        hasSession: true,
        hasCaptureImagesReady: false,
        hasRevealed: false,
      }),
    ).toBe(true);
  });

  it('reveals the capture window once the frozen screen session is ready', () => {
    expect(
      shouldRevealCaptureWindow({
        status: 'selecting',
        hasSession: true,
        hasCaptureImagesReady: true,
        hasRevealed: false,
      }),
    ).toBe(true);
  });

  it('reveals error state so permission or session failures are visible', () => {
    expect(
      shouldRevealCaptureWindow({
        status: 'error',
        hasSession: false,
        hasCaptureImagesReady: false,
        hasRevealed: false,
      }),
    ).toBe(true);
  });

  it('does not reveal more than once for the same capture window instance', () => {
    expect(
      shouldRevealCaptureWindow({
        status: 'selecting',
        hasSession: true,
        hasCaptureImagesReady: true,
        hasRevealed: true,
      }),
    ).toBe(false);
  });

  it('delegates capture window reveal to the native client', async () => {
    const calls: string[] = [];
    const window: CaptureWindowHandle = {
      show: async () => {
        calls.push('show');
      },
      setFocus: async () => {
        calls.push('setFocus');
      },
    };

    await revealCaptureWindow(window, {
      revealCaptureWindow: async () => {
        calls.push('native_reveal_capture_window');
      },
      restoreCaptureSnapshotWindowsForSession: async () => {
        calls.push('restore');
      },
    });

    expect(calls).toEqual(['native_reveal_capture_window']);
  });

  it('keeps the native capture presentation active after reveal', async () => {
    const calls: string[] = [];
    const window: CaptureWindowHandle = {
      show: async () => {
        calls.push('show');
      },
      setFocus: async () => {
        calls.push('setFocus');
      },
    };

    await revealCaptureWindowForSession({
      window,
      sessionId: 'capture-1',
      prepareSurface: async () => {
        calls.push('prepare_surface');
      },
      client: {
        prepareCaptureWindowForReveal: async () => {
          calls.push('native_prepare_capture_window');
        },
        revealCaptureWindow: async () => {
          calls.push('native_reveal_capture_window');
        },
        restoreCaptureSnapshotWindowsForSession: async (sessionId) => {
          calls.push(
            `restore_capture_snapshot_windows_for_session:${sessionId}`,
          );
        },
      },
    });

    expect(calls).toEqual([
      'native_prepare_capture_window',
      'prepare_surface',
      'native_reveal_capture_window',
    ]);
  });

  it('waits for two animation frames before fading in the capture overlay', async () => {
    const calls: string[] = [];
    const pendingFrameCallbacks: FrameRequestCallback[] = [];
    const requestAnimationFrame = (callback: FrameRequestCallback) => {
      calls.push('requestAnimationFrame');
      pendingFrameCallbacks.push(callback);
      return pendingFrameCallbacks.length;
    };

    const wait = waitForCaptureSurfacePaint(requestAnimationFrame);
    expect(calls).toEqual(['requestAnimationFrame']);

    pendingFrameCallbacks.shift()?.(1);
    await Promise.resolve();
    expect(calls).toEqual(['requestAnimationFrame', 'requestAnimationFrame']);

    pendingFrameCallbacks.shift()?.(2);
    await wait;
  });

  it('declares the Tauri permissions required for delayed capture reveal', () => {
    expect(getCaptureWindowRevealPermissions()).toEqual([]);
  });
});
