import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

const backendSnapshot = {
  general: {
    language: 'en',
    theme: 'dark',
    start_on_boot: true,
  },
  screenshot: {
    save_path: '/captures',
    format: 'webp',
    quality: 77,
  },
  translation: {
    default_source_lang: 'ja',
    default_target_lang: 'fr',
  },
};

describe('Tauri settings command adapter', () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it('normalizes the backend settings snapshot', async () => {
    const { getSettingsSnapshot } = await import('./settings');
    invoke.mockResolvedValueOnce(backendSnapshot);

    await expect(getSettingsSnapshot()).resolves.toEqual({
      general: {
        language: 'en',
        theme: 'dark',
        startOnBoot: true,
      },
      screenshot: {
        savePath: '/captures',
        format: 'webp',
        quality: 77,
      },
      translation: {
        defaultSourceLang: 'ja',
        defaultTargetLang: 'fr',
      },
    });
    expect(invoke).toHaveBeenCalledWith('get_settings_snapshot');
  });

  it('maps general settings updates to the backend payload shape', async () => {
    const { updateGeneralSettings } = await import('./settings');
    invoke.mockResolvedValueOnce(backendSnapshot);

    await updateGeneralSettings({
      language: 'en',
      theme: 'dark',
      startOnBoot: true,
    });

    expect(invoke).toHaveBeenCalledWith('update_general_settings', {
      input: {
        language: 'en',
        theme: 'dark',
        start_on_boot: true,
      },
    });
  });

  it('maps screenshot settings updates to the backend payload shape', async () => {
    const { updateScreenshotSettings } = await import('./settings');
    invoke.mockResolvedValueOnce(backendSnapshot);

    await updateScreenshotSettings({
      savePath: '/captures',
      format: 'jpg',
      quality: 81,
    });

    expect(invoke).toHaveBeenCalledWith('update_screenshot_settings', {
      input: {
        save_path: '/captures',
        format: 'jpg',
        quality: 81,
      },
    });
  });

  it('maps translation settings updates to the backend payload shape', async () => {
    const { updateTranslationSettings } = await import('./settings');
    invoke.mockResolvedValueOnce(backendSnapshot);

    await updateTranslationSettings({
      defaultSourceLang: 'auto',
      defaultTargetLang: 'en',
    });

    expect(invoke).toHaveBeenCalledWith('update_translation_settings', {
      input: {
        default_source_lang: 'auto',
        default_target_lang: 'en',
      },
    });
  });
});
