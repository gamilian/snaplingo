import { describe, expect, it } from 'vitest';
import {
  getCaptureWindowRevealPermissions,
  revealCaptureWindow,
  revealCaptureWindowForSession,
  shouldRevealCaptureWindow,
  type CaptureWindowHandle,
} from './captureWindowVisibility';

describe('capture window visibility', () => {
  it('keeps the capture window hidden while the session snapshot is loading', () => {
    expect(
      shouldRevealCaptureWindow({
        status: 'loading',
        hasSession: false,
        hasRevealed: false,
      }),
    ).toBe(false);
  });

  it('reveals the capture window once the frozen screen session is ready', () => {
    expect(
      shouldRevealCaptureWindow({
        status: 'selecting',
        hasSession: true,
        hasRevealed: false,
      }),
    ).toBe(true);
  });

  it('reveals error state so permission or session failures are visible', () => {
    expect(
      shouldRevealCaptureWindow({
        status: 'error',
        hasSession: false,
        hasRevealed: false,
      }),
    ).toBe(true);
  });

  it('does not reveal more than once for the same capture window instance', () => {
    expect(
      shouldRevealCaptureWindow({
        status: 'selecting',
        hasSession: true,
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
      invoke: async (command, args) => {
        calls.push(`${command}:${String(args?.sessionId)}`);
      },
    });

    expect(calls).toEqual([
      'show',
      'setFocus',
      'restore_capture_snapshot_windows_for_session:capture-1',
    ]);
  });

  it('declares the Tauri permissions required for delayed capture reveal', () => {
    expect(getCaptureWindowRevealPermissions()).toEqual([
      'core:window:allow-show',
      'core:window:allow-set-focus',
    ]);
  });
});
