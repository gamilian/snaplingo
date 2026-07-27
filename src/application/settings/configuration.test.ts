import { describe, expect, it, vi } from 'vitest';
import type {
  OcrProviderInfo,
  ProviderInfo,
  SettingsSnapshot,
} from './ports';
import {
  createSettingsConfiguration,
  type SettingsConfigurationEventsPort,
} from './configuration';

const initialSettings: SettingsSnapshot = {
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
    annotationColors: [[255, 77, 79, 255]],
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

const hotkeys = {
  screenshot: { screenshot: 'Shift+Command+R' },
  translation: { 'selection-translate': 'Alt+D' },
  ocr: { 'screenshot-ocr': 'Shift+Alt+S' },
};

describe('settings configuration application module', () => {
  it('shares one hydration request and publishes the durable snapshot', async () => {
    let finishHydration!: (snapshot: SettingsSnapshot) => void;
    const hydration = new Promise<SettingsSnapshot>((resolve) => {
      finishHydration = resolve;
    });
    const { configuration, durableSettings } = createFixture();
    durableSettings.getSettingsSnapshot.mockReturnValueOnce(hydration);
    const listener = vi.fn();
    configuration.settings.subscribe(listener);

    const first = configuration.settings.hydrate();
    const second = configuration.settings.hydrate();
    finishHydration(initialSettings);

    await expect(Promise.all([first, second])).resolves.toEqual([
      initialSettings,
      initialSettings,
    ]);
    expect(durableSettings.getSettingsSnapshot).toHaveBeenCalledTimes(1);
    expect(configuration.settings.getState()).toEqual({
      hydrated: true,
      snapshot: initialSettings,
    });
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it('does not let an older refresh overwrite a newer update', async () => {
    let finishRefresh!: (snapshot: SettingsSnapshot) => void;
    const staleRefresh = new Promise<SettingsSnapshot>((resolve) => {
      finishRefresh = resolve;
    });
    const updated = {
      ...initialSettings,
      general: { ...initialSettings.general, theme: 'dark' },
    };
    const { configuration, durableSettings } = createFixture();
    await configuration.settings.hydrate();
    durableSettings.getSettingsSnapshot.mockReturnValueOnce(staleRefresh);
    durableSettings.updateGeneralSettings.mockResolvedValueOnce(updated);

    const refresh = configuration.settings.refresh();
    await configuration.settings.updateGeneral({ theme: 'dark' });
    finishRefresh(initialSettings);

    await expect(refresh).resolves.toBe(updated);
    expect(configuration.settings.getState().snapshot).toBe(updated);
  });

  it('serializes partial updates against the latest applied snapshot', async () => {
    let finishFirst!: (snapshot: SettingsSnapshot) => void;
    const firstResponse = new Promise<SettingsSnapshot>((resolve) => {
      finishFirst = resolve;
    });
    const firstSnapshot = {
      ...initialSettings,
      translation: { ...initialSettings.translation, autoCopy: true },
    };
    const secondSnapshot = {
      ...firstSnapshot,
      translation: {
        ...firstSnapshot.translation,
        preserveLineBreaks: false,
      },
    };
    const { configuration, durableSettings } = createFixture();
    await configuration.settings.hydrate();
    durableSettings.updateTranslationSettings
      .mockReturnValueOnce(firstResponse)
      .mockResolvedValueOnce(secondSnapshot);

    const first = configuration.settings.updateTranslation({ autoCopy: true });
    const second = configuration.settings.updateTranslation({
      preserveLineBreaks: false,
    });

    await Promise.resolve();
    expect(durableSettings.updateTranslationSettings).toHaveBeenCalledTimes(1);
    finishFirst(firstSnapshot);
    await Promise.all([first, second]);
    expect(durableSettings.updateTranslationSettings).toHaveBeenNthCalledWith(2, {
      ...firstSnapshot.translation,
      preserveLineBreaks: false,
    });
    expect(configuration.settings.getState().snapshot).toBe(secondSnapshot);
  });

  it('owns provider conversion and mutation reload policy', async () => {
    const { configuration, providers } = createFixture();
    providers.listTranslation.mockResolvedValue([
      {
        id: 'custom-llm-1',
        name: 'custom-llm-1',
        isConfigured: true,
        requiresApiKey: true,
        isActive: true,
        isBuiltin: false,
        protocol: 'openai',
        endpoint: 'https://example.com',
        model: 'gpt-5-mini',
        reasoningLevel: null,
        promptStrategyId: null,
        promptFallbackStrategyId: null,
      },
    ]);

    await configuration.providers.loadTranslation();
    await configuration.providers.configureTranslation('custom-llm-1', {
      apiKey: 'secret',
    });

    expect(configuration.providers.getState()).toMatchObject({
      activeTranslationProviders: ['custom-llm-1'],
      translationProviders: [{ name: 'gpt-5-mini', status: 'active' }],
    });
    expect(providers.configureTranslationCredentials).toHaveBeenCalledWith(
      'custom-llm-1',
      { api_key: 'secret' },
    );
    expect(providers.listTranslation).toHaveBeenCalledTimes(2);
  });

  it('hydrates and mutates hotkeys through one observable facet', async () => {
    const updated = {
      ...hotkeys,
      translation: { 'selection-translate': 'Shift+Alt+D' },
    };
    const { configuration, hotkeyPort } = createFixture();
    hotkeyPort.updateHotkey.mockResolvedValueOnce({
      snapshot: updated,
      accelerator: 'Shift+Alt+KeyD',
    });

    await configuration.hotkeys.hydrate();
    await configuration.hotkeys.update(
      'translation',
      'selection-translate',
      'Shift+Alt+D',
    );

    expect(hotkeyPort.getHotkeySnapshot).toHaveBeenCalledTimes(1);
    expect(hotkeyPort.getDefaultHotkeySnapshot).toHaveBeenCalledTimes(1);
    expect(configuration.hotkeys.getState()).toEqual({
      hydrated: true,
      snapshot: updated,
      defaultSnapshot: hotkeys,
    });
  });

  it('owns cross-window refresh and Settings-window invalidation policy', async () => {
    const handlers = new Map<string, () => void | Promise<void>>();
    const unsubscribers: Array<ReturnType<typeof vi.fn>> = [];
    const events = createEvents((name, handler) => {
      handlers.set(name, handler);
      const unsubscribe = vi.fn();
      unsubscribers.push(unsubscribe);
      return unsubscribe;
    });
    const { configuration, durableSettings, providers, hotkeyPort } = createFixture({
      events,
    });
    const onSettingsChanged = vi.fn();
    const invalidateHistory = vi.fn();
    const invalidateFavorites = vi.fn();
    const invalidateScreenshotFavorites = vi.fn();

    const dispose = configuration.synchronize({
      settingsWindow: true,
      onSettingsChanged,
      invalidateHistory,
      invalidateFavorites,
      invalidateScreenshotFavorites,
    });
    await Promise.resolve();
    await handlers.get('settings')?.();
    await handlers.get('providers')?.();
    await handlers.get('hotkeys')?.();
    await handlers.get('history')?.();
    await handlers.get('favorites')?.();
    await handlers.get('screenshotFavorites')?.();

    expect(durableSettings.getSettingsSnapshot).toHaveBeenCalledTimes(1);
    expect(providers.listTranslation).toHaveBeenCalledTimes(1);
    expect(providers.listOcr).toHaveBeenCalledTimes(1);
    expect(hotkeyPort.getHotkeySnapshot).toHaveBeenCalledTimes(1);
    expect(onSettingsChanged).toHaveBeenCalledWith(initialSettings);
    expect(invalidateHistory).toHaveBeenCalledTimes(1);
    expect(invalidateFavorites).toHaveBeenCalledTimes(1);
    expect(invalidateScreenshotFavorites).toHaveBeenCalledTimes(1);

    dispose();
    expect(unsubscribers).toHaveLength(6);
    unsubscribers.forEach((unsubscribe) =>
      expect(unsubscribe).toHaveBeenCalledTimes(1),
    );
  });
});

function createFixture(overrides: { events?: SettingsConfigurationEventsPort } = {}) {
  const durableSettings = {
    getSettingsSnapshot: vi.fn(async () => initialSettings),
    updateGeneralSettings: vi.fn(async () => initialSettings),
    updateScreenshotSettings: vi.fn(async () => initialSettings),
    updateAnnotationColors: vi.fn(async () => initialSettings),
    updateTranslationSettings: vi.fn(async () => initialSettings),
    updateOcrSettings: vi.fn(async () => initialSettings),
    updateHistorySettings: vi.fn(async () => initialSettings),
  };
  const providers = createProviderPort();
  const hotkeyPort = {
    getHotkeySnapshot: vi.fn(async () => hotkeys),
    getDefaultHotkeySnapshot: vi.fn(async () => hotkeys),
    updateHotkey: vi.fn(async () => ({
      snapshot: hotkeys,
      accelerator: null as string | null,
    })),
    resetHotkey: vi.fn(async () => ({
      snapshot: hotkeys,
      accelerator: null as string | null,
    })),
    resetHotkeyCategory: vi.fn(async () => hotkeys),
  };
  const events = overrides.events ?? createEvents(() => () => undefined);
  const configuration = createSettingsConfiguration({
    durableSettings,
    providers,
    hotkeys: hotkeyPort,
    events,
  });

  return { configuration, durableSettings, providers, hotkeyPort, events };
}

function createEvents(
  subscribe: (
    name: string,
    handler: () => void | Promise<void>,
  ) => () => void,
): SettingsConfigurationEventsPort {
  return {
    subscribeSettingsChanged: async (handler) => subscribe('settings', handler),
    subscribeHotkeysChanged: async (handler) => subscribe('hotkeys', handler),
    subscribeProvidersChanged: async (handler) => subscribe('providers', handler),
    subscribeHistoryChanged: async (handler) => subscribe('history', handler),
    subscribeFavoritesChanged: async (handler) => subscribe('favorites', handler),
    subscribeScreenshotFavoritesChanged: async (handler) =>
      subscribe('screenshotFavorites', handler),
  };
}

function createProviderPort() {
  return {
    listTranslation: vi.fn(async () => [] as ProviderInfo[]),
    activateTranslation: vi.fn(async () => undefined),
    deactivateTranslation: vi.fn(async () => undefined),
    reorderActiveTranslation: vi.fn(async () => undefined),
    getTranslationCredentialSchema: vi.fn(async () => []),
    getOcrCredentialSchema: vi.fn(async () => []),
    configureTranslationCredentials: vi.fn(async () => undefined),
    addCustomTranslation: vi.fn(),
    updateCustomTranslation: vi.fn(),
    removeCustomTranslation: vi.fn(async () => undefined),
    testCustomTranslation: vi.fn(async () => undefined),
    listTranslationPromptStrategies: vi.fn(),
    saveTranslationPromptStrategies: vi.fn(),
    listOpenAICompatibleModels: vi.fn(),
    testOpenAICompatible: vi.fn(),
    testOpenAIResponses: vi.fn(),
    listAnthropicModels: vi.fn(),
    testAnthropic: vi.fn(),
    listGeminiModels: vi.fn(),
    testGemini: vi.fn(),
    listOcr: vi.fn(async () => [] as OcrProviderInfo[]),
    activateOcr: vi.fn(async () => undefined),
    configureOcrCredentials: vi.fn(async () => undefined),
  };
}
