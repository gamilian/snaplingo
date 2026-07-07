import { beforeEach, describe, expect, it, vi } from 'vitest';

const settingsApi = vi.hoisted(() => ({
  getSettingsSnapshot: vi.fn(),
  updateGeneralSettings: vi.fn(),
  updateScreenshotSettings: vi.fn(),
  updateTranslationSettings: vi.fn(),
}));

vi.mock('../tauri/settings', () => settingsApi);

const backendSnapshot = {
  general: {
    language: 'zh-CN',
    theme: 'system',
    startOnBoot: false,
  },
  screenshot: {
    savePath: '/backend/captures',
    format: 'png',
    quality: 90,
  },
  translation: {
    defaultSourceLang: 'auto',
    defaultTargetLang: 'zh-CN',
  },
};

function createMemoryStorage(): Storage {
  const values = new Map<string, string>();

  return {
    get length() {
      return values.size;
    },
    clear() {
      values.clear();
    },
    getItem(key) {
      return values.get(key) ?? null;
    },
    key(index) {
      return Array.from(values.keys())[index] ?? null;
    },
    removeItem(key) {
      values.delete(key);
    },
    setItem(key, value) {
      values.set(key, value);
    },
  };
}

function legacyPersistedState(state: Record<string, unknown>) {
  localStorage.setItem(
    'snaplingo-settings',
    JSON.stringify({
      state,
      version: 0,
    }),
  );
}

describe('settingsConfigStore', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      value: createMemoryStorage(),
    });
    localStorage.clear();
    settingsApi.getSettingsSnapshot.mockResolvedValue(structuredClone(backendSnapshot));
  });

  it('hydrates once from the backend snapshot', async () => {
    const { useSettingsConfigStore } = await import('./settingsConfigStore');

    await useSettingsConfigStore.getState().hydrate();
    await useSettingsConfigStore.getState().hydrate();

    expect(settingsApi.getSettingsSnapshot).toHaveBeenCalledTimes(1);
    expect(useSettingsConfigStore.getState()).toMatchObject({
      hydrated: true,
      general: backendSnapshot.general,
      screenshot: backendSnapshot.screenshot,
      translation: backendSnapshot.translation,
    });
  });

  it('migrates legacy durable values once and clears only migrated keys', async () => {
    settingsApi.updateGeneralSettings.mockResolvedValueOnce({
      ...backendSnapshot,
      general: {
        language: 'en',
        theme: 'dark',
        startOnBoot: true,
      },
    });
    settingsApi.updateScreenshotSettings.mockResolvedValueOnce({
      ...backendSnapshot,
      general: {
        language: 'en',
        theme: 'dark',
        startOnBoot: true,
      },
      screenshot: {
        savePath: '~/legacy-captures',
        format: 'jpg',
        quality: 81,
      },
    });
    settingsApi.updateTranslationSettings.mockResolvedValueOnce({
      general: {
        language: 'en',
        theme: 'dark',
        startOnBoot: true,
      },
      screenshot: {
        savePath: '~/legacy-captures',
        format: 'jpg',
        quality: 81,
      },
      translation: {
        defaultSourceLang: 'ja',
        defaultTargetLang: 'fr',
      },
    });
    legacyPersistedState({
      activeMainTab: 'translation',
      screenshotSubTab: 'save-settings',
      hotkeys: {
        screenshot: { screenshot: 'F12' },
        translation: { 'selection-translate': 'F10' },
        ocr: { 'screenshot-ocr': 'F9' },
      },
      capturedScreenshot: 'data:image/png;base64,abc',
      language: 'en',
      theme: 'dark',
      startOnBoot: true,
      screenshotSavePath: '~/legacy-captures',
      screenshotFormat: 'jpg',
      screenshotQuality: 81,
      defaultSourceLang: 'ja',
      defaultTargetLang: 'fr',
    });

    const { useSettingsConfigStore } = await import('./settingsConfigStore');

    await useSettingsConfigStore.getState().hydrate();
    await useSettingsConfigStore.getState().hydrate();

    expect(settingsApi.getSettingsSnapshot).toHaveBeenCalledTimes(1);
    expect(settingsApi.updateGeneralSettings).toHaveBeenCalledWith({
      language: 'en',
      theme: 'dark',
      startOnBoot: true,
    });
    expect(settingsApi.updateScreenshotSettings).toHaveBeenCalledWith({
      savePath: '~/legacy-captures',
      format: 'jpg',
      quality: 81,
    });
    expect(settingsApi.updateTranslationSettings).toHaveBeenCalledWith({
      defaultSourceLang: 'ja',
      defaultTargetLang: 'fr',
    });
    expect(useSettingsConfigStore.getState()).toMatchObject({
      hydrated: true,
      general: {
        language: 'en',
        theme: 'dark',
        startOnBoot: true,
      },
      screenshot: {
        savePath: '~/legacy-captures',
        format: 'jpg',
        quality: 81,
      },
      translation: {
        defaultSourceLang: 'ja',
        defaultTargetLang: 'fr',
      },
    });

    expect(JSON.parse(localStorage.getItem('snaplingo-settings') ?? '{}')).toEqual({
      state: {
        activeMainTab: 'translation',
        screenshotSubTab: 'save-settings',
        hotkeys: {
          screenshot: { screenshot: 'F12' },
          translation: { 'selection-translate': 'F10' },
          ocr: { 'screenshot-ocr': 'F9' },
        },
        capturedScreenshot: 'data:image/png;base64,abc',
      },
      version: 0,
    });
  });

  it('leaves settingsStore with only navigation and hotkey state after rehydration', async () => {
    legacyPersistedState({
      activeMainTab: 'translation',
      screenshotSubTab: 'save-settings',
      translationSubTab: 'translation-settings',
      ocrSubTab: 'favorites',
      servicesSubTab: 'translation',
      hotkeys: {
        screenshot: { screenshot: 'F12' },
        translation: { 'selection-translate': 'F10' },
        ocr: { 'screenshot-ocr': 'F9' },
      },
      language: 'en',
      theme: 'dark',
      startOnBoot: true,
      screenshotSavePath: '~/legacy-captures',
      screenshotFormat: 'jpg',
      screenshotQuality: 81,
      defaultSourceLang: 'ja',
      defaultTargetLang: 'fr',
    });

    const { useSettingsStore } = await import('./settingsStore');
    const state = useSettingsStore.getState() as Record<string, unknown>;

    expect(state.activeMainTab).toBe('translation');
    expect(state.screenshotSubTab).toBe('save-settings');
    expect(state.translationSubTab).toBe('translation-settings');
    expect(state.ocrSubTab).toBe('favorites');
    expect(state.servicesSubTab).toBe('translation');
    expect(state.hotkeys).toMatchObject({
      screenshot: { screenshot: 'F12' },
      translation: { 'selection-translate': 'F10' },
      ocr: { 'screenshot-ocr': 'F9' },
    });
    expect('language' in state).toBe(false);
    expect('theme' in state).toBe(false);
    expect('startOnBoot' in state).toBe(false);
    expect('screenshotSavePath' in state).toBe(false);
    expect('screenshotFormat' in state).toBe(false);
    expect('screenshotQuality' in state).toBe(false);
    expect('defaultSourceLang' in state).toBe(false);
    expect('defaultTargetLang' in state).toBe(false);
    expect('setLanguage' in state).toBe(false);
    expect('setScreenshotSavePath' in state).toBe(false);
    expect('setDefaultTargetLang' in state).toBe(false);
    expect(typeof state.setActiveMainTab).toBe('function');
    expect(typeof state.setHotkey).toBe('function');
  });
});
