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

  it('keeps the capture window hidden until frozen screen images are decoded', () => {
    expect(
      shouldRevealCaptureWindow({
        status: 'selecting',
        hasSession: true,
        hasCaptureImagesReady: false,
        hasRevealed: false,
      }),
    ).toBe(false);
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

  it('shows the capture window before focusing it', async () => {
    const calls: string[] = [];
    const window: CaptureWindowHandle = {
      show: async () => {
        calls.push('show');
      },
      setFocus: async () => {
        calls.push('setFocus');
      },
    };

    await revealCaptureWindow(window);

    expect(calls).toEqual(['show', 'setFocus']);
  });

  it('restores hidden app windows only after the capture window is visible', async () => {
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
      client: {
        restoreCaptureSnapshotWindowsForSession: async (sessionId) => {
          calls.push(`restore_capture_snapshot_windows_for_session:${sessionId}`);
        },
      },
    });

    expect(calls).toEqual([
      'show',
      'setFocus',
      'restore_capture_snapshot_windows_for_session:capture-1',
    ]);
  });

  it('waits for two animation frames before fading in the capture surface', async () => {
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
    expect(getCaptureWindowRevealPermissions()).toEqual([
      'core:window:allow-show',
      'core:window:allow-set-focus',
    ]);
  });
});
