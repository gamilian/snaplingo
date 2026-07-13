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
      screenshot: {
        savePath: '/tmp',
        format: 'png',
        quality: 90,
        annotationColors: [[255, 77, 79, 255]] as [number, number, number, number][],
      },
      translation: { defaultSourceLang: 'auto', defaultTargetLang: 'en' },
    };
    const durableSettings = {
      getSettingsSnapshot: vi.fn(async () => snapshot),
      updateGeneralSettings: vi.fn(async () => snapshot),
      updateScreenshotSettings: vi.fn(async () => snapshot),
      updateAnnotationColors: vi.fn(async () => snapshot),
      updateTranslationSettings: vi.fn(async () => snapshot),
    };
    const runtime = createSettingsRuntime(createPorts({ durableSettings }));

    await expect(runtime.durableSettings.load()).resolves.toBe(snapshot);
    await runtime.durableSettings.updateGeneral(snapshot.general);
    await runtime.durableSettings.updateScreenshot(snapshot.screenshot);
    await runtime.durableSettings.updateAnnotationColors(
      snapshot.screenshot.annotationColors,
    );
    await runtime.durableSettings.updateTranslation(snapshot.translation);

    expect(durableSettings.getSettingsSnapshot).toHaveBeenCalledTimes(1);
    expect(durableSettings.updateGeneralSettings).toHaveBeenCalledWith(
      snapshot.general,
    );
    expect(durableSettings.updateScreenshotSettings).toHaveBeenCalledWith(
      snapshot.screenshot,
    );
    expect(durableSettings.updateAnnotationColors).toHaveBeenCalledWith(
      snapshot.screenshot.annotationColors,
    );
    expect(durableSettings.updateTranslationSettings).toHaveBeenCalledWith(
      snapshot.translation,
    );
  });

  it('exposes the injected provider facet without duplicate wrappers', () => {
    const providers = createProviderPort();
    const runtime = createSettingsRuntime(createPorts({ providers }));

    expect(runtime.providers).toBe(providers);
  });

  it('translates hotkey actions into portable port calls', async () => {
    const hotkeys = {
      getHotkeySnapshot: vi.fn(async () => ({
        screenshot: {},
        translation: {},
        ocr: {},
      })),
      getDefaultHotkeySnapshot: vi.fn(async () => ({
        screenshot: { screenshot: '⇧⌘R' },
        translation: {},
        ocr: {},
      })),
      updateHotkey: vi.fn(async () => ({
        snapshot: { screenshot: {}, translation: {}, ocr: {} },
        accelerator: 'CommandOrControl+Shift+S',
      })),
      resetHotkey: vi.fn(async () => ({
        snapshot: { screenshot: {}, translation: {}, ocr: {} },
        accelerator: 'CommandOrControl+Shift+R',
      })),
      resetHotkeyCategory: vi.fn(async () => ({
        screenshot: { screenshot: '⇧⌘R' },
        translation: {},
        ocr: {},
      })),
    };
    const runtime = createSettingsRuntime(createPorts({ hotkeys }));
    const input = {
      category: 'screenshot' as const,
      action: 'capture',
      hotkey: '⇧⌘S',
    };

    const snapshot = await runtime.hotkeys.load();
    const defaults = await runtime.hotkeys.loadDefaults();
    const outcome = await runtime.hotkeys.update(input);
    await runtime.hotkeys.reset('screenshot', 'capture');
    await runtime.hotkeys.resetCategory('screenshot');

    expect(hotkeys.getHotkeySnapshot).toHaveBeenCalledTimes(1);
    expect(hotkeys.getDefaultHotkeySnapshot).toHaveBeenCalledTimes(1);
    expect(snapshot).toEqual({ screenshot: {}, translation: {}, ocr: {} });
    expect(defaults.screenshot.screenshot).toBe('⇧⌘R');
    expect(hotkeys.updateHotkey).toHaveBeenCalledWith(input);
    expect(outcome).toEqual({
      snapshot: { screenshot: {}, translation: {}, ocr: {} },
      accelerator: 'CommandOrControl+Shift+S',
    });
    expect(hotkeys.resetHotkey).toHaveBeenCalledWith('screenshot', 'capture');
    expect(hotkeys.resetHotkeyCategory).toHaveBeenCalledWith('screenshot');
  });

  it('translates history actions into portable port calls', async () => {
    const history = {
      getTranslationHistory: vi.fn(async () => []),
      getOcrHistory: vi.fn(async () => []),
      deleteHistory: vi.fn(async () => undefined),
      setHistoryFavorite: vi.fn(async () => undefined),
      updateHistoryNote: vi.fn(async () => undefined),
      replaceHistoryTags: vi.fn(async () => undefined),
      clearAllHistory: vi.fn(async () => undefined),
    };
    const runtime = createSettingsRuntime(createPorts({ history }));

    await runtime.history.loadTranslation(20, 40);
    await runtime.history.loadOcr(10, 0);
    await runtime.history.deleteEntry(42);
    await runtime.history.setFavorite(42, true);
    await runtime.history.updateNote(42, 'keep this');
    await runtime.history.replaceTags(42, ['work']);
    await runtime.history.clear();

    expect(history.getTranslationHistory).toHaveBeenCalledWith(20, 40);
    expect(history.getOcrHistory).toHaveBeenCalledWith(10, 0);
    expect(history.deleteHistory).toHaveBeenCalledWith(42);
    expect(history.setHistoryFavorite).toHaveBeenCalledWith(42, true);
    expect(history.updateHistoryNote).toHaveBeenCalledWith(42, 'keep this');
    expect(history.replaceHistoryTags).toHaveBeenCalledWith(42, ['work']);
    expect(history.clearAllHistory).toHaveBeenCalledTimes(1);
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
      updateAnnotationColors: vi.fn(),
      updateTranslationSettings: vi.fn(),
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
      deleteHistory: vi.fn(),
      setHistoryFavorite: vi.fn(),
      updateHistoryNote: vi.fn(),
      replaceHistoryTags: vi.fn(),
      clearAllHistory: vi.fn(),
    },
    clipboard: { writeText: vi.fn() },
    capture: { triggerScreenshot: vi.fn() },
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
