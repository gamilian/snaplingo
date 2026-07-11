import { describe, expect, it, vi } from 'vitest';
import { createSettingsRuntime } from './runtime';

describe('settings runtime', () => {
  it('opens the Settings window through its portable port', async () => {
    const window = { openSettings: vi.fn(async () => undefined) };
    const runtime = createSettingsRuntime(createPorts({ window }));

    await runtime.window.open();

    expect(window.openSettings).toHaveBeenCalledTimes(1);
  });

  it('translates durable settings actions into portable port calls', async () => {
    const snapshot = {
      general: { language: 'zh-CN', theme: 'system', startOnBoot: false },
      screenshot: { savePath: '/tmp', format: 'png', quality: 90 },
      translation: { defaultSourceLang: 'auto', defaultTargetLang: 'en' },
    };
    const durableSettings = {
      getSettingsSnapshot: vi.fn(async () => snapshot),
      updateGeneralSettings: vi.fn(async () => snapshot),
      updateScreenshotSettings: vi.fn(async () => snapshot),
      updateTranslationSettings: vi.fn(async () => snapshot),
    };
    const runtime = createSettingsRuntime(createPorts({ durableSettings }));

    await expect(runtime.durableSettings.load()).resolves.toBe(snapshot);
    await runtime.durableSettings.updateGeneral(snapshot.general);
    await runtime.durableSettings.updateScreenshot(snapshot.screenshot);
    await runtime.durableSettings.updateTranslation(snapshot.translation);

    expect(durableSettings.getSettingsSnapshot).toHaveBeenCalledTimes(1);
    expect(durableSettings.updateGeneralSettings).toHaveBeenCalledWith(
      snapshot.general,
    );
    expect(durableSettings.updateScreenshotSettings).toHaveBeenCalledWith(
      snapshot.screenshot,
    );
    expect(durableSettings.updateTranslationSettings).toHaveBeenCalledWith(
      snapshot.translation,
    );
  });

  it('translates provider actions into the provider port vocabulary', async () => {
    const providers = createProviderPort();
    const runtime = createSettingsRuntime(createPorts({ providers }));
    const credentials = { api_key: 'secret' };
    const addRequest = {
      name: 'Custom',
      protocol: 'openai',
      endpoint: 'https://example.com',
      model: 'model',
      api_key: 'secret',
    };
    const updateRequest = { ...addRequest, api_key: undefined };
    const modelRequest = {
      endpoint: 'https://example.com',
      api_key: 'secret',
    };
    const testRequest = { ...modelRequest, model: 'model' };
    const strategyConfig = { strategies: [] };

    await runtime.providers.listTranslation();
    await runtime.providers.activateTranslation('provider');
    await runtime.providers.deactivateTranslation('provider');
    await runtime.providers.reorderActiveTranslation(['one', 'two']);
    await runtime.providers.getTranslationCredentialSchema('provider');
    await runtime.providers.getOcrCredentialSchema('ocr');
    await runtime.providers.configureTranslationCredentials(
      'provider',
      credentials,
    );
    await runtime.providers.addCustomTranslation(addRequest);
    await runtime.providers.updateCustomTranslation('provider', updateRequest);
    await runtime.providers.removeCustomTranslation('provider');
    await runtime.providers.testCustomTranslation('provider');
    await runtime.providers.listTranslationPromptStrategies();
    await runtime.providers.saveTranslationPromptStrategies(strategyConfig);
    await runtime.providers.listOpenAICompatibleModels(modelRequest);
    await runtime.providers.testOpenAICompatible(testRequest);
    await runtime.providers.testOpenAIResponses(testRequest);
    await runtime.providers.listAnthropicModels(modelRequest);
    await runtime.providers.testAnthropic(testRequest);
    await runtime.providers.listGeminiModels(modelRequest);
    await runtime.providers.testGemini(testRequest);
    await runtime.providers.listOcr();
    await runtime.providers.activateOcr('ocr');
    await runtime.providers.configureOcrCredentials('ocr', credentials);

    expect(providers.listTranslationProviders).toHaveBeenCalledTimes(1);
    expect(providers.activateTranslationProvider).toHaveBeenCalledWith(
      'provider',
    );
    expect(providers.deactivateTranslationProvider).toHaveBeenCalledWith(
      'provider',
    );
    expect(providers.reorderActiveTranslationProviders).toHaveBeenCalledWith([
      'one',
      'two',
    ]);
    expect(providers.getProviderCredentialSchema).toHaveBeenCalledWith(
      'provider',
    );
    expect(providers.getOcrProviderCredentialSchema).toHaveBeenCalledWith('ocr');
    expect(
      providers.configureTranslationProviderCredentials,
    ).toHaveBeenCalledWith('provider', credentials);
    expect(providers.addCustomTranslationProvider).toHaveBeenCalledWith(
      addRequest,
    );
    expect(providers.updateCustomTranslationProvider).toHaveBeenCalledWith(
      'provider',
      updateRequest,
    );
    expect(providers.removeCustomTranslationProvider).toHaveBeenCalledWith(
      'provider',
    );
    expect(providers.testCustomTranslationProvider).toHaveBeenCalledWith(
      'provider',
    );
    expect(providers.listTranslationPromptStrategies).toHaveBeenCalledTimes(1);
    expect(providers.saveTranslationPromptStrategies).toHaveBeenCalledWith(
      strategyConfig,
    );
    expect(providers.listOpenAICompatibleModels).toHaveBeenCalledWith(
      modelRequest,
    );
    expect(providers.testOpenAICompatibleProvider).toHaveBeenCalledWith(
      testRequest,
    );
    expect(providers.testOpenAIResponsesProvider).toHaveBeenCalledWith(
      testRequest,
    );
    expect(providers.listAnthropicModels).toHaveBeenCalledWith(modelRequest);
    expect(providers.testAnthropicProvider).toHaveBeenCalledWith(testRequest);
    expect(providers.listGeminiModels).toHaveBeenCalledWith(modelRequest);
    expect(providers.testGeminiProvider).toHaveBeenCalledWith(testRequest);
    expect(providers.listOcrProviders).toHaveBeenCalledTimes(1);
    expect(providers.activateOcrProvider).toHaveBeenCalledWith('ocr');
    expect(providers.configureOcrProviderCredentials).toHaveBeenCalledWith(
      'ocr',
      credentials,
    );
  });

  it('translates hotkey actions into portable port calls', async () => {
    const hotkeys = {
      getHotkeySnapshot: vi.fn(async () => ({
        screenshot: {},
        translation: {},
        ocr: {},
      })),
      updateHotkey: vi.fn(async () => ({
        snapshot: { screenshot: {}, translation: {}, ocr: {} },
        accelerator: 'CommandOrControl+Shift+S',
      })),
    };
    const runtime = createSettingsRuntime(createPorts({ hotkeys }));
    const input = {
      category: 'screenshot' as const,
      action: 'capture',
      hotkey: '⇧⌘S',
    };

    await runtime.hotkeys.load();
    await runtime.hotkeys.update(input);

    expect(hotkeys.getHotkeySnapshot).toHaveBeenCalledTimes(1);
    expect(hotkeys.updateHotkey).toHaveBeenCalledWith(input);
  });

  it('translates history actions into portable port calls', async () => {
    const history = {
      getTranslationHistory: vi.fn(async () => []),
      getOcrHistory: vi.fn(async () => []),
      deleteHistory: vi.fn(async () => undefined),
      clearAllHistory: vi.fn(async () => undefined),
    };
    const runtime = createSettingsRuntime(createPorts({ history }));

    await runtime.history.loadTranslation(20, 40);
    await runtime.history.loadOcr(10, 0);
    await runtime.history.deleteEntry(42);
    await runtime.history.clear();

    expect(history.getTranslationHistory).toHaveBeenCalledWith(20, 40);
    expect(history.getOcrHistory).toHaveBeenCalledWith(10, 0);
    expect(history.deleteHistory).toHaveBeenCalledWith(42);
    expect(history.clearAllHistory).toHaveBeenCalledTimes(1);
  });

  it('copies Settings text through the clipboard port', async () => {
    const clipboard = { writeText: vi.fn(async () => undefined) };
    const runtime = createSettingsRuntime(createPorts({ clipboard }));

    await runtime.clipboard.copyText('recognized text');

    expect(clipboard.writeText).toHaveBeenCalledWith('recognized text');
  });

  it('enters capture from Advanced settings through the capture port', async () => {
    const capture = { triggerScreenshot: vi.fn(async () => undefined) };
    const runtime = createSettingsRuntime(createPorts({ capture }));

    await runtime.advanced.triggerCapture();

    expect(capture.triggerScreenshot).toHaveBeenCalledTimes(1);
  });
});

function createPorts(overrides: Record<string, unknown> = {}) {
  return {
    window: { openSettings: vi.fn(async () => undefined) },
    durableSettings: {
      getSettingsSnapshot: vi.fn(),
      updateGeneralSettings: vi.fn(),
      updateScreenshotSettings: vi.fn(),
      updateTranslationSettings: vi.fn(),
    },
    providers: createProviderPort(),
    hotkeys: {
      getHotkeySnapshot: vi.fn(),
      updateHotkey: vi.fn(),
    },
    history: {
      getTranslationHistory: vi.fn(),
      getOcrHistory: vi.fn(),
      deleteHistory: vi.fn(),
      clearAllHistory: vi.fn(),
    },
    clipboard: { writeText: vi.fn() },
    capture: { triggerScreenshot: vi.fn() },
    ...overrides,
  };
}

function createProviderPort() {
  return {
    listTranslationProviders: vi.fn(),
    activateTranslationProvider: vi.fn(),
    deactivateTranslationProvider: vi.fn(),
    reorderActiveTranslationProviders: vi.fn(),
    getProviderCredentialSchema: vi.fn(),
    getOcrProviderCredentialSchema: vi.fn(),
    configureTranslationProviderCredentials: vi.fn(),
    addCustomTranslationProvider: vi.fn(),
    updateCustomTranslationProvider: vi.fn(),
    removeCustomTranslationProvider: vi.fn(),
    testCustomTranslationProvider: vi.fn(),
    listTranslationPromptStrategies: vi.fn(),
    saveTranslationPromptStrategies: vi.fn(),
    listOpenAICompatibleModels: vi.fn(),
    testOpenAICompatibleProvider: vi.fn(),
    testOpenAIResponsesProvider: vi.fn(),
    listAnthropicModels: vi.fn(),
    testAnthropicProvider: vi.fn(),
    listGeminiModels: vi.fn(),
    testGeminiProvider: vi.fn(),
    listOcrProviders: vi.fn(),
    activateOcrProvider: vi.fn(),
    configureOcrProviderCredentials: vi.fn(),
  };
}
