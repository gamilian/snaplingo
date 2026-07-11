import { beforeEach, describe, expect, it, vi } from 'vitest';

const { getByLabel, setFocus, show } = vi.hoisted(() => ({
  getByLabel: vi.fn(),
  setFocus: vi.fn(),
  show: vi.fn(),
}));

vi.mock('@tauri-apps/api/webviewWindow', () => ({
  WebviewWindow: { getByLabel },
}));

import { settingsWindow } from './settingsWindow';

describe('Tauri settings window adapter', () => {
  beforeEach(() => {
    setFocus.mockReset().mockResolvedValue(undefined);
    show.mockReset().mockResolvedValue(undefined);
    getByLabel.mockReset().mockResolvedValue({ setFocus, show });
  });

  it('opens settings by owning its label lookup, show, and focus sequence', async () => {
    await settingsWindow.openSettings();

    expect(getByLabel).toHaveBeenCalledWith('settings');
    expect(show).toHaveBeenCalledOnce();
    expect(setFocus).toHaveBeenCalledOnce();
    expect(show.mock.invocationCallOrder[0]).toBeLessThan(
      setFocus.mock.invocationCallOrder[0],
    );
  });

  it('does nothing when the settings window is unavailable', async () => {
    getByLabel.mockResolvedValueOnce(null);

    await expect(settingsWindow.openSettings()).resolves.toBeUndefined();

    expect(show).not.toHaveBeenCalled();
    expect(setFocus).not.toHaveBeenCalled();
  });
});
