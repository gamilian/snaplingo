import { describe, expect, it, vi } from 'vitest';

const appWindow = { label: 'main' };
const webviewWindow = { label: 'webview' };
const settingsWindow = { label: 'settings' };
const getCurrentWindow = vi.fn(() => appWindow);
const getCurrentWebviewWindow = vi.fn(() => webviewWindow);
const getByLabel = vi.fn(() => Promise.resolve(settingsWindow));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow,
  LogicalSize: class LogicalSize {
    constructor(
      public width: number,
      public height: number,
    ) {}
  },
  PhysicalPosition: class PhysicalPosition {
    constructor(
      public x: number,
      public y: number,
    ) {}
  },
}));

vi.mock('@tauri-apps/api/webviewWindow', () => ({
  getCurrentWebviewWindow,
  WebviewWindow: { getByLabel },
}));

describe('window adapter', () => {
  it('returns current app and webview windows through the Tauri seam', async () => {
    const { getCurrentAppWindow, getCurrentAppWebviewWindow } = await import(
      '../window'
    );

    expect(getCurrentAppWindow()).toBe(appWindow);
    expect(getCurrentAppWebviewWindow()).toBe(webviewWindow);
  });

  it('creates Tauri size and position value objects', async () => {
    const { createLogicalSize, createPhysicalPosition } = await import('../window');

    expect(createLogicalSize(640, 480)).toMatchObject({ width: 640, height: 480 });
    expect(createPhysicalPosition(12, 24)).toMatchObject({ x: 12, y: 24 });
  });

  it('resolves webview windows by label', async () => {
    const { getWebviewWindowByLabel } = await import('../window');

    await expect(getWebviewWindowByLabel('settings')).resolves.toBe(settingsWindow);
    expect(getByLabel).toHaveBeenCalledWith('settings');
  });
});
