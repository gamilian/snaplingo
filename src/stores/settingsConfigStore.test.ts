import { beforeEach, describe, expect, it, vi } from 'vitest';

const settingsRuntime = vi.hoisted(() => ({
  load: vi.fn(),
  updateGeneral: vi.fn(),
  updateScreenshot: vi.fn(),
  updateAnnotationColors: vi.fn(),
  updateTranslation: vi.fn(),
  updateOcr: vi.fn(),
  updateHistory: vi.fn(),
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
    annotationColors: [[255, 77, 79, 255]],
  },
  translation: {
    defaultSourceLang: 'auto',
    defaultTargetLang: 'zh-CN',
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

  it('refreshes an already hydrated snapshot from the backend', async () => {
    const refreshedSnapshot = {
      ...backendSnapshot,
      general: { ...backendSnapshot.general, theme: 'dark' },
    };
    const { initializeSettingsConfigStore, useSettingsConfigStore } =
      await import('./settingsConfigStore');
    initializeSettingsConfigStore(settingsRuntime);

    await useSettingsConfigStore.getState().hydrate();
    settingsRuntime.load.mockResolvedValueOnce(refreshedSnapshot);
    const snapshot = await useSettingsConfigStore.getState().refresh();

    expect(settingsRuntime.load).toHaveBeenCalledTimes(2);
    expect(snapshot).toEqual(refreshedSnapshot);
    expect(useSettingsConfigStore.getState().general?.theme).toBe('dark');
  });

  it('does not let an older refresh overwrite a newer update', async () => {
    let resolveRefresh!: (snapshot: typeof backendSnapshot) => void;
    const staleRefresh = new Promise<typeof backendSnapshot>((resolve) => {
      resolveRefresh = resolve;
    });
    const updatedSnapshot = {
      ...backendSnapshot,
      general: { ...backendSnapshot.general, theme: 'dark' },
    };
    const { initializeSettingsConfigStore, useSettingsConfigStore } =
      await import('./settingsConfigStore');
    initializeSettingsConfigStore(settingsRuntime);
    await useSettingsConfigStore.getState().hydrate();
    settingsRuntime.load.mockReturnValueOnce(staleRefresh);
    settingsRuntime.updateGeneral.mockResolvedValueOnce(updatedSnapshot);

    const refresh = useSettingsConfigStore.getState().refresh();
    const update = useSettingsConfigStore
      .getState()
      .updateGeneralSettings(updatedSnapshot.general);
    await update;
    resolveRefresh(structuredClone(backendSnapshot));
    const effectiveRefreshSnapshot = await refresh;

    expect(useSettingsConfigStore.getState().general?.theme).toBe('dark');
    expect(effectiveRefreshSnapshot.general.theme).toBe('dark');
  });

  it('serializes partial settings updates against the latest snapshot', async () => {
    let resolveFirst!: (snapshot: typeof backendSnapshot) => void;
    const firstResponse = new Promise<typeof backendSnapshot>((resolve) => {
      resolveFirst = resolve;
    });
    const firstSnapshot = {
      ...backendSnapshot,
      translation: { ...backendSnapshot.translation, autoCopy: true },
    };
    const secondSnapshot = {
      ...firstSnapshot,
      translation: {
        ...firstSnapshot.translation,
        preserveLineBreaks: false,
      },
    };
    const { initializeSettingsConfigStore, useSettingsConfigStore } =
      await import('./settingsConfigStore');
    initializeSettingsConfigStore(settingsRuntime);
    await useSettingsConfigStore.getState().hydrate();
    settingsRuntime.updateTranslation
      .mockReturnValueOnce(firstResponse)
      .mockResolvedValueOnce(secondSnapshot);

    const first = useSettingsConfigStore
      .getState()
      .updateTranslationSettings({ autoCopy: true });
    const second = useSettingsConfigStore
      .getState()
      .updateTranslationSettings({ preserveLineBreaks: false });

    await Promise.resolve();
    expect(settingsRuntime.updateTranslation).toHaveBeenCalledTimes(1);
    resolveFirst(firstSnapshot);
    await first;
    await second;

    expect(settingsRuntime.updateTranslation).toHaveBeenNthCalledWith(2, {
      ...firstSnapshot.translation,
      preserveLineBreaks: false,
    });
    expect(useSettingsConfigStore.getState().translation).toEqual(
      secondSnapshot.translation,
    );
  });

  it('serializes screenshot updates without restoring stale fields', async () => {
    let resolveFirst!: (snapshot: typeof backendSnapshot) => void;
    const firstResponse = new Promise<typeof backendSnapshot>((resolve) => {
      resolveFirst = resolve;
    });
    const firstSnapshot = {
      ...backendSnapshot,
      screenshot: { ...backendSnapshot.screenshot, quality: 80 },
    };
    const secondSnapshot = {
      ...firstSnapshot,
      screenshot: { ...firstSnapshot.screenshot, format: 'jpg' },
    };
    const { initializeSettingsConfigStore, useSettingsConfigStore } =
      await import('./settingsConfigStore');
    initializeSettingsConfigStore(settingsRuntime);
    await useSettingsConfigStore.getState().hydrate();
    settingsRuntime.updateScreenshot
      .mockReturnValueOnce(firstResponse)
      .mockResolvedValueOnce(secondSnapshot);

    const first = useSettingsConfigStore
      .getState()
      .updateScreenshotSettings({ quality: 80 });
    const second = useSettingsConfigStore
      .getState()
      .updateScreenshotSettings({ format: 'jpg' });

    await Promise.resolve();
    expect(settingsRuntime.updateScreenshot).toHaveBeenCalledTimes(1);
    resolveFirst(firstSnapshot);
    await first;
    await second;

    expect(settingsRuntime.updateScreenshot).toHaveBeenNthCalledWith(2, {
      ...firstSnapshot.screenshot,
      format: 'jpg',
    });
  });

  it('ignores legacy durable values and uses the backend snapshot', async () => {
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
    expect(settingsRuntime.updateGeneral).not.toHaveBeenCalled();
    expect(settingsRuntime.updateScreenshot).not.toHaveBeenCalled();
    expect(settingsRuntime.updateTranslation).not.toHaveBeenCalled();
    expect(useSettingsConfigStore.getState()).toMatchObject({
      hydrated: true,
      general: backendSnapshot.general,
      screenshot: backendSnapshot.screenshot,
      translation: backendSnapshot.translation,
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
        language: 'en',
        theme: 'dark',
        startOnBoot: true,
        screenshotSavePath: '~/legacy-captures',
        screenshotFormat: 'jpg',
        screenshotQuality: 81,
        defaultSourceLang: 'ja',
        defaultTargetLang: 'fr',
      },
      version: 0,
    });
  });

  it('updates annotation colors through the narrow durable settings action', async () => {
    const colors: [number, number, number, number][] = [
      [12, 34, 56, 255],
      [200, 150, 100, 255],
    ];
    settingsRuntime.updateAnnotationColors.mockResolvedValueOnce({
      ...backendSnapshot,
      screenshot: {
        ...backendSnapshot.screenshot,
        annotationColors: colors,
      },
    });
    const { initializeSettingsConfigStore, useSettingsConfigStore } =
      await import('./settingsConfigStore');
    initializeSettingsConfigStore(settingsRuntime);

    await useSettingsConfigStore.getState().updateAnnotationColors(colors);

    expect(settingsRuntime.updateAnnotationColors).toHaveBeenCalledWith(colors);
    expect(useSettingsConfigStore.getState().screenshot).toEqual({
      ...backendSnapshot.screenshot,
      annotationColors: colors,
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
    expect('screenshotSubTab' in state).toBe(false);
    expect('translationSubTab' in state).toBe(false);
    expect('ocrSubTab' in state).toBe(false);
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

  it('falls back when persisted navigation points at the removed advanced page', async () => {
    legacyPersistedState({
      activeMainTab: 'advanced',
      servicesSubTab: 'translation',
    });

    const { useSettingsStore } = await import('./settingsStore');

    expect(useSettingsStore.getState().activeMainTab).toBe('screenshot');
    expect(useSettingsStore.getState().servicesSubTab).toBe('translation');
  });

  it('keeps one-shot section requests out of persisted navigation state', async () => {
    const { useSettingsStore } = await import('./settingsStore');

    useSettingsStore.getState().navigate({ tab: 'general', section: 'about' });

    expect(useSettingsStore.getState()).toMatchObject({
      activeMainTab: 'general',
      requestedSection: 'about',
    });
    expect(
      JSON.parse(localStorage.getItem('snaplingo-settings') ?? '{}').state,
    ).not.toHaveProperty('requestedSection');

    useSettingsStore.getState().setActiveMainTab('screenshot');
    expect(useSettingsStore.getState().requestedSection).toBeNull();

    useSettingsStore.getState().navigate({ tab: 'general', section: 'about' });
    useSettingsStore.getState().consumeRequestedSection();
    expect(useSettingsStore.getState().requestedSection).toBeNull();
  });
});
