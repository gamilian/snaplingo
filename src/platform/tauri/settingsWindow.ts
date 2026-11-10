import { getVersion } from '@tauri-apps/api/app';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { currentMonitor, getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalSize } from '@tauri-apps/api/dpi';
import type { SettingsWindowPort } from '../../application/settings/ports';

const SETTINGS_WINDOW_LABEL = 'settings';

export const settingsWindow: SettingsWindowPort = {
  async openSettings() {
    const window = await WebviewWindow.getByLabel(SETTINGS_WINDOW_LABEL);
    if (!window) return;

    await window.show();
    await window.setFocus();
  },
  async selectScreenshotDirectory() {
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === 'string' ? selected : null;
  },
  getAppVersion: () => getVersion(),
  async displayBounds() {
    const monitor = await currentMonitor();
    return monitor ? { x: monitor.position.x / monitor.scaleFactor, y: monitor.position.y / monitor.scaleFactor, width: monitor.size.width / monitor.scaleFactor, height: monitor.size.height / monitor.scaleFactor } : null;
  },
  async applyInterfaceScale(scale) {
    const window = getCurrentWindow();
    const monitor = await currentMonitor();
    const maxWidth = monitor ? monitor.size.width / monitor.scaleFactor - 48 : 4000;
    const maxHeight = monitor ? monitor.size.height / monitor.scaleFactor - 48 : 3000;
    await window.setMinSize(new LogicalSize(Math.min(700 * scale, maxWidth), Math.min(500 * scale, maxHeight)));
  },
  openExternal: (url) => invoke('plugin:shell|open', { path: url }),
};
