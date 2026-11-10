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
      beginHotkeyRecording: vi.fn(),
      endHotkeyRecording: vi.fn(),
      subscribeRecordedHotkey: vi.fn(),
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
      queryFavoriteKeys: vi.fn(async () => []),
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
    permissions: {
      subscribe: vi.fn(), context: vi.fn(), request: vi.fn(), reset: vi.fn(),
      restart: vi.fn(), refresh: vi.fn(),
    },
    updates: {
      subscribe: vi.fn(), ensureChecked: vi.fn(), check: vi.fn(),
      downloadAndOpen: vi.fn(), openReleasePage: vi.fn(),
    },
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
    reorderInactiveTranslation: vi.fn(),
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
