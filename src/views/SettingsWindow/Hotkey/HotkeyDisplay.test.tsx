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
      expect(container.textContent).toBe('Ctrl+Alt+Shift+F1');
      expect(Array.from(container.querySelectorAll('kbd'), (key) => key.textContent)).toEqual(['Ctrl', 'Alt', 'Shift', 'F1']);
    } finally {
      await act(async () => root.unmount());
    }
  });

  it.each([
    ['F1', ['F1']],
    ['⌘F1', ['Ctrl', 'F1']],
    ['⇧F3', ['Shift', 'F3']],
    ['⌃⌘F3', ['Ctrl', 'F3']],
    ['未设置', []],
  ] as const)('shows only the active Windows keys for %s', async (value, keys) => {
    vi.stubGlobal('navigator', { platform: 'Win32' });
    const container = document.createElement('div');
    const root = createRoot(container);
    try {
      await act(async () => root.render(createElement(HotkeyDisplay, { value })));
      expect(Array.from(container.querySelectorAll('kbd'), (key) => key.textContent)).toEqual(keys);
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
