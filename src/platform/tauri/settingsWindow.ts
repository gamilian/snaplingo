import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { SettingsWindowPort } from '../../application/settings/ports';

const SETTINGS_WINDOW_LABEL = 'settings';

export const settingsWindow: SettingsWindowPort = {
  async openSettings() {
    const window = await WebviewWindow.getByLabel(SETTINGS_WINDOW_LABEL);
    if (!window) return;

    await window.show();
    await window.setFocus();
  },
};
