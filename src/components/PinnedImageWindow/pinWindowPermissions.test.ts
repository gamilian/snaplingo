import { describe, expect, it } from 'vitest';
import { getPinnedWindowRequiredPermissions } from './pinWindowPermissions';

describe('pinned image window permissions', () => {
  it('declares the Tauri window permissions required for Snipaste-style pin controls', () => {
    expect(getPinnedWindowRequiredPermissions()).toEqual([
      'core:window:allow-close',
      'core:window:allow-set-position',
      'core:window:allow-set-size',
      'core:window:allow-start-dragging',
    ]);
  });
});
