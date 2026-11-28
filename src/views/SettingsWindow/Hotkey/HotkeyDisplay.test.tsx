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
      expect(container.textContent).toBe('ShiftAltCtrlWinF1');
      expect(container.querySelector('button')?.getAttribute('aria-label')).toBe('修改快捷键：Shift + Alt + Ctrl + F1');
      expect(Array.from(container.querySelectorAll('kbd'), (key) => key.textContent)).toEqual(['Shift', 'Alt', 'Ctrl', 'Win']);
    } finally {
      await act(async () => root.unmount());
    }
  });

  it.each([
    ['F1', []],
    ['⊞F4', ['Win']],
    ['⌘F1', ['Ctrl']],
    ['⇧F3', ['Shift']],
    ['⌃⌘F3', ['Ctrl']],
    ['未设置', []],
  ] as const)('keeps inactive Windows modifiers dimmed for %s', async (value, keys) => {
    vi.stubGlobal('navigator', { platform: 'Win32' });
    const container = document.createElement('div');
    const root = createRoot(container);
    try {
      await act(async () => root.render(createElement(HotkeyDisplay, { value })));
      expect(Array.from(container.querySelectorAll('kbd'), (key) => key.textContent)).toEqual(['Shift', 'Alt', 'Ctrl', 'Win']);
      expect(Array.from(container.querySelectorAll('kbd.text-ink'), (key) => key.textContent)).toEqual(keys);
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

  it.each(['Win32', 'MacIntel'])('dims the old shortcut while recording on %s', async (platform) => {
    vi.stubGlobal('navigator', { platform });
    const container = document.createElement('div');
    const root = createRoot(container);
    const onClick = vi.fn();
    try {
      await act(async () => root.render(createElement(HotkeyDisplay, { value: '⇧⌥⌘F1', isRecording: true, onClick })));
      expect(container.querySelectorAll('kbd.text-faint')).toHaveLength(4);
      expect(container.querySelectorAll('kbd.text-ink')).toHaveLength(0);
      expect(container.textContent).toContain('按下快捷键...');
      expect(container.textContent).not.toContain('F1');
      const button = container.querySelector('button')!;
      expect(button.getAttribute('aria-label')).toContain('Esc 取消');
      button.click();
      expect(onClick).toHaveBeenCalledOnce();
    } finally {
      await act(async () => root.unmount());
    }
  });
});
