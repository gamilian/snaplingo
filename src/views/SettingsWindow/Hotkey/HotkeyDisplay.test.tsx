// @vitest-environment happy-dom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { HotkeyDisplay } from './HotkeyDisplay';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe('HotkeyDisplay', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('uses Windows modifier names for recorded shortcuts', async () => {
    vi.stubGlobal('navigator', { platform: 'Win32' });
    const container = document.createElement('div');
    const root = createRoot(container);
    try {
      await act(async () => root.render(createElement(HotkeyDisplay, { value: '⇧⌥⌘F1' })));
      expect(container.textContent).toBe('ShiftAltCtrlF1');
    } finally {
      await act(async () => root.unmount());
    }
  });

  it('keeps Apple modifier symbols on macOS', async () => {
    vi.stubGlobal('navigator', { platform: 'MacIntel' });
    const container = document.createElement('div');
    const root = createRoot(container);
    try {
      await act(async () => root.render(createElement(HotkeyDisplay, { value: '⇧⌥⌘F1' })));
      expect(container.textContent).toBe('⇧⌥⌘⌃F1');
    } finally {
      await act(async () => root.unmount());
    }
  });
});
