import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DurableSettingsConfigurationState } from '../application/settings/configuration';
import type { SettingsSnapshot } from '../application/settings/ports';

const snapshot: SettingsSnapshot = {
  general: { language: 'zh-CN', theme: 'system', startOnBoot: false },
  screenshot: {
    savePath: '/captures',
    format: 'png',
    quality: 90,
    namingRule: 'timestamp',
    customFileName: 'SnapLingo',
    autoCopy: false,
    defaultStrokeWidth: 2,
    defaultFontSize: 24,
    rememberLastTool: true,
    showSelectionSize: true,
    showMagnifier: true,
    pinOpacity: 100,
    pinShadow: true,
    annotationColors: [],
  },
  translation: {
    defaultSourceLang: 'auto',
    defaultTargetLang: 'en',
    autoTranslate: true,
    autoCopy: false,
    preserveLineBreaks: true,
    incrementalTranslation: false,
    windowAlwaysOnTop: true,
    hideOnBlur: false,
  },
  ocr: {
    recognitionLanguage: 'auto',
    preserveFormatting: true,
    removeChineseSpaces: true,
    showConfidence: false,
  },
  history: {
    autoCleanupEnabled: false,
    retentionDays: 30,
    maximumRecords: 5000,
    maximumFavorites: 1000,
  },
};

describe('settingsConfigStore projection', () => {
  beforeEach(() => vi.resetModules());

  it('projects Application state and forwards update intents', async () => {
    let listener!: (state: { hydrated: boolean; snapshot: SettingsSnapshot | null }) => void;
    const configuration = {
      getState: (): DurableSettingsConfigurationState => ({
        hydrated: false,
        snapshot: null,
      }),
      subscribe: vi.fn((next) => {
        listener = next;
        return () => undefined;
      }),
      hydrate: vi.fn(async () => snapshot),
      refresh: vi.fn(async () => snapshot),
      updateGeneral: vi.fn(async () => snapshot),
      updateScreenshot: vi.fn(async () => snapshot),
      updateAnnotationColors: vi.fn(async () => snapshot),
      updateTranslation: vi.fn(async () => snapshot),
      updateOcr: vi.fn(async () => snapshot),
      updateHistory: vi.fn(async () => snapshot),
    };
    const { initializeSettingsConfigStore, useSettingsConfigStore } =
      await import('./settingsConfigStore');
    initializeSettingsConfigStore(configuration);

    listener({ hydrated: true, snapshot });
    await useSettingsConfigStore.getState().updateGeneralSettings({ theme: 'dark' });

    expect(useSettingsConfigStore.getState()).toMatchObject({
      hydrated: true,
      general: snapshot.general,
      screenshot: snapshot.screenshot,
    });
    expect(configuration.updateGeneral).toHaveBeenCalledWith({ theme: 'dark' });
  });
});
