import { describe, expect, it } from 'vitest';
import {
  getCaptureWindowRevealPermissions,
  shouldRevealCaptureWindow,
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

  it('keeps the capture window hidden until the frozen images are hydrated', () => {
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

  it('declares the Tauri permissions required for delayed capture reveal', () => {
    expect(getCaptureWindowRevealPermissions()).toEqual([]);
  });
});
