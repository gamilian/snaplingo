import { beforeEach, describe, expect, it, vi } from 'vitest';

const settingsRuntime = vi.hoisted(() => ({
  load: vi.fn(),
  updateGeneral: vi.fn(),
  updateScreenshot: vi.fn(),
  updateTranslation: vi.fn(),
}));

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
    settingsRuntime.load.mockResolvedValue(structuredClone(backendSnapshot));
  });

  it('hydrates once from the backend snapshot', async () => {
    const { initializeSettingsConfigStore, useSettingsConfigStore } =
      await import('./settingsConfigStore');
    initializeSettingsConfigStore(settingsRuntime);

    await useSettingsConfigStore.getState().hydrate();
    await useSettingsConfigStore.getState().hydrate();

    expect(settingsRuntime.load).toHaveBeenCalledTimes(1);
    expect(useSettingsConfigStore.getState()).toMatchObject({
      hydrated: true,
      general: backendSnapshot.general,
      screenshot: backendSnapshot.screenshot,
      translation: backendSnapshot.translation,
    });
  });

  it('migrates legacy durable values once and clears only migrated keys', async () => {
    settingsRuntime.updateGeneral.mockResolvedValueOnce({
      ...backendSnapshot,
      general: {
        language: 'en',
        theme: 'dark',
        startOnBoot: true,
      },
    });
    settingsRuntime.updateScreenshot.mockResolvedValueOnce({
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
    settingsRuntime.updateTranslation.mockResolvedValueOnce({
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

    const { initializeSettingsConfigStore, useSettingsConfigStore } =
      await import('./settingsConfigStore');
    initializeSettingsConfigStore(settingsRuntime);

    await useSettingsConfigStore.getState().hydrate();
    await useSettingsConfigStore.getState().hydrate();

    expect(settingsRuntime.load).toHaveBeenCalledTimes(1);
    expect(settingsRuntime.updateGeneral).toHaveBeenCalledWith({
      language: 'en',
      theme: 'dark',
      startOnBoot: true,
    });
    expect(settingsRuntime.updateScreenshot).toHaveBeenCalledWith({
      savePath: '~/legacy-captures',
      format: 'jpg',
      quality: 81,
    });
    expect(settingsRuntime.updateTranslation).toHaveBeenCalledWith({
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

  it('leaves settingsStore with only navigation state after rehydration', async () => {
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
    const state = useSettingsStore.getState() as unknown as Record<string, unknown>;

    expect(state.activeMainTab).toBe('translation');
    expect(state.screenshotSubTab).toBe('save-settings');
    expect(state.translationSubTab).toBe('translation-settings');
    expect(state.ocrSubTab).toBe('favorites');
    expect(state.servicesSubTab).toBe('translation');
    expect('hotkeys' in state).toBe(false);
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
    expect('setHotkey' in state).toBe(false);
    expect('clearHotkey' in state).toBe(false);
    expect('resetHotkeys' in state).toBe(false);
    expect(JSON.parse(localStorage.getItem('snaplingo-settings') ?? '{}')).toMatchObject({
      state: {
        hotkeys: {
          screenshot: { screenshot: 'F12' },
          translation: { 'selection-translate': 'F10' },
          ocr: { 'screenshot-ocr': 'F9' },
        },
      },
    });
  });
});
