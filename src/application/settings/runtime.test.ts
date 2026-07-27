import { describe, expect, it, vi } from 'vitest';
import { createSettingsRuntime } from './runtime';

describe('settings runtime', () => {
  it('opens the Settings window through its portable port', async () => {
    const window = {
      openSettings: vi.fn(async () => undefined),
      selectScreenshotDirectory: vi.fn(async () => null),
      getAppVersion: vi.fn(async () => '1.2.3'),
    };
    const navigationHandler = vi.fn();
    const unsubscribe = vi.fn();
    const windowEvents = {
      subscribeNavigationRequested: vi.fn(async () => unsubscribe),
    };
    const runtime = createSettingsRuntime(createPorts({ window, windowEvents }));

    await runtime.window.open();
    await expect(runtime.window.version()).resolves.toBe('1.2.3');
    await expect(
      runtime.window.subscribeNavigationRequested(navigationHandler),
    ).resolves.toBe(unsubscribe);

    expect(window.openSettings).toHaveBeenCalledTimes(1);
    expect(window.getAppVersion).toHaveBeenCalledTimes(1);
    expect(windowEvents.subscribeNavigationRequested).toHaveBeenCalledWith(
      navigationHandler,
    );
  });

  it('translates history actions into portable port calls', async () => {
    const history = {
      getTranslationHistory: vi.fn(async () => []),
      getOcrHistory: vi.fn(async () => []),
      queryTranslationHistory: vi.fn(async () => ({ items: [], total: 0 })),
      queryOcrHistory: vi.fn(async () => ({ items: [], total: 0 })),
      deleteHistory: vi.fn(async () => undefined),
      updateHistoryNote: vi.fn(async () => undefined),
      replaceHistoryTags: vi.fn(async () => undefined),
      clearAllHistory: vi.fn(async () => undefined),
      clearHistory: vi.fn(async () => undefined),
    };
    const runtime = createSettingsRuntime(createPorts({ history }));

    await runtime.history.loadTranslation(20, 40);
    await runtime.history.loadOcr(10, 0);
    await runtime.history.queryTranslation({
      search: 'hello',
      limit: 20,
      offset: 40,
    });
    await runtime.history.queryOcr({
      search: '',
      limit: 20,
      offset: 0,
    });
    await runtime.history.deleteEntry(42);
    await runtime.history.updateNote(42, 'keep this');
    await runtime.history.replaceTags(42, ['work']);
    await runtime.history.clear();
    await runtime.history.clearKind('translation');

    expect(history.getTranslationHistory).toHaveBeenCalledWith(20, 40);
    expect(history.getOcrHistory).toHaveBeenCalledWith(10, 0);
    expect(history.queryTranslationHistory).toHaveBeenCalledWith({
      search: 'hello',
      limit: 20,
      offset: 40,
    });
    expect(history.queryOcrHistory).toHaveBeenCalledWith({
      search: '',
      limit: 20,
      offset: 0,
    });
    expect(history.deleteHistory).toHaveBeenCalledWith(42);
    expect(history.updateHistoryNote).toHaveBeenCalledWith(42, 'keep this');
    expect(history.replaceHistoryTags).toHaveBeenCalledWith(42, ['work']);
    expect(history.clearAllHistory).toHaveBeenCalledTimes(1);
    expect(history.clearHistory).toHaveBeenCalledWith('translation');
  });

  it('copies Settings text through the clipboard port', async () => {
    const clipboard = { writeText: vi.fn(async () => undefined) };
    const runtime = createSettingsRuntime(createPorts({ clipboard }));

    await runtime.clipboard.copyText('recognized text');

    expect(clipboard.writeText).toHaveBeenCalledWith('recognized text');
  });

  it('propagates rejected portable operations unchanged', async () => {
    const error = new Error('clipboard unavailable');
    const clipboard = { writeText: vi.fn(async () => Promise.reject(error)) };
    const runtime = createSettingsRuntime(createPorts({ clipboard }));

    await expect(runtime.clipboard.copyText('text')).rejects.toBe(error);
  });

});

function createPorts(overrides: Record<string, unknown> = {}) {
  return {
    window: {
      openSettings: vi.fn(async () => undefined),
      selectScreenshotDirectory: vi.fn(async () => null),
      getAppVersion: vi.fn(async () => '0.1.0'),
    },
    windowEvents: {
      subscribeNavigationRequested: vi.fn(async () => () => undefined),
    },
    configurationEvents: {
      subscribeSettingsChanged: vi.fn(async () => () => undefined),
      subscribeHotkeysChanged: vi.fn(async () => () => undefined),
      subscribeProvidersChanged: vi.fn(async () => () => undefined),
      subscribeHistoryChanged: vi.fn(async () => () => undefined),
      subscribeFavoritesChanged: vi.fn(async () => () => undefined),
      subscribeScreenshotFavoritesChanged: vi.fn(
        async () => () => undefined,
      ),
    },
    durableSettings: {
      getSettingsSnapshot: vi.fn(),
      updateGeneralSettings: vi.fn(),
      updateScreenshotSettings: vi.fn(),
      updateAnnotationColors: vi.fn(),
      updateTranslationSettings: vi.fn(),
      updateOcrSettings: vi.fn(),
      updateHistorySettings: vi.fn(),
    },
    providers: createProviderPort(),
    hotkeys: {
      getHotkeySnapshot: vi.fn(),
      getDefaultHotkeySnapshot: vi.fn(),
      updateHotkey: vi.fn(),
      resetHotkey: vi.fn(),
      resetHotkeyCategory: vi.fn(),
    },
    history: {
      getTranslationHistory: vi.fn(),
      getOcrHistory: vi.fn(),
      queryTranslationHistory: vi.fn(),
      queryOcrHistory: vi.fn(),
      deleteHistory: vi.fn(),
      updateHistoryNote: vi.fn(),
      replaceHistoryTags: vi.fn(),
      clearAllHistory: vi.fn(),
      clearHistory: vi.fn(),
      rerunOcrHistory: vi.fn(),
    },
    favorites: {
      addTranslationFavorite: vi.fn(),
      addOcrFavorite: vi.fn(),
      queryFavorites: vi.fn(),
      updateFavoriteMetadata: vi.fn(),
      deleteFavorite: vi.fn(),
      rerunOcrFavorite: vi.fn(),
      listFavoriteTags: vi.fn(),
    },
    screenshotFavorites: {
      queryScreenshotFavorites: vi.fn(),
      updateScreenshotFavoriteMetadata: vi.fn(),
      deleteScreenshotFavorite: vi.fn(),
      copyScreenshotFavorite: vi.fn(),
      revealScreenshotFavorite: vi.fn(),
    },
    libraryIndex: {
      queryHistoryIndex: vi.fn(),
      queryFavoriteIndex: vi.fn(),
    },
    clipboard: { writeText: vi.fn() },
    maintenance: {
      listAppLogs: vi.fn(),
      clearAppLogs: vi.fn(),
    },
    tts: {
      listVoices: vi.fn(),
      speak: vi.fn(),
    },
    ...overrides,
  };
}

function createProviderPort() {
  return {
    listTranslation: vi.fn(),
    activateTranslation: vi.fn(),
    deactivateTranslation: vi.fn(),
    reorderActiveTranslation: vi.fn(),
    getTranslationCredentialSchema: vi.fn(),
    getOcrCredentialSchema: vi.fn(),
    configureTranslationCredentials: vi.fn(),
    addCustomTranslation: vi.fn(),
    updateCustomTranslation: vi.fn(),
    removeCustomTranslation: vi.fn(),
    testCustomTranslation: vi.fn(),
    listTranslationPromptStrategies: vi.fn(),
    saveTranslationPromptStrategies: vi.fn(),
    listOpenAICompatibleModels: vi.fn(),
    testOpenAICompatible: vi.fn(),
    testOpenAIResponses: vi.fn(),
    listAnthropicModels: vi.fn(),
    testAnthropic: vi.fn(),
    listGeminiModels: vi.fn(),
    testGemini: vi.fn(),
    listOcr: vi.fn(),
    activateOcr: vi.fn(),
    configureOcrCredentials: vi.fn(),
  };
}
