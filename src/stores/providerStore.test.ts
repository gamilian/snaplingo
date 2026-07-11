import { beforeEach, describe, expect, it, vi } from 'vitest';

const providersRuntime = vi.hoisted(() => ({
  configureOcrCredentials: vi.fn(),
  configureTranslationCredentials: vi.fn(),
  updateCustomTranslation: vi.fn(),
  testCustomTranslation: vi.fn(),
  listOcr: vi.fn(),
  listTranslation: vi.fn(),
}));

describe('providerStore', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    providersRuntime.listOcr.mockResolvedValue([]);
    providersRuntime.listTranslation.mockResolvedValue([]);
  });

  it('passes DeepLX standard DeepL mode credentials through the credential map command', async () => {
    const { initializeProviderStore, useProviderStore } =
      await import('./providerStore');
    initializeProviderStore(providersRuntime as never);

    await useProviderStore.getState().updateProviderConfig('translation', 'deeplx', {
      mode: 'deepl',
      api_key: 'secret',
    });

    expect(providersRuntime.configureTranslationCredentials).toHaveBeenCalledWith('deeplx', {
      mode: 'deepl',
      api_key: 'secret',
    });
  });

  it('configures OCR providers through the credential map command', async () => {
    const { initializeProviderStore, useProviderStore } =
      await import('./providerStore');
    initializeProviderStore(providersRuntime as never);

    await useProviderStore.getState().configureOcrProvider('baidu-ocr', {
      api_key: 'key',
      secret_key: 'secret',
    });

    expect(providersRuntime.configureOcrCredentials).toHaveBeenCalledWith('baidu-ocr', {
      api_key: 'key',
      secret_key: 'secret',
    });
  });

  it('updates custom translation providers through the custom provider command', async () => {
    const { initializeProviderStore, useProviderStore } =
      await import('./providerStore');
    initializeProviderStore(providersRuntime as never);

    await useProviderStore.getState().updateCustomTranslationProvider('custom-gpt', {
      name: 'gpt-5-mini',
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      model: 'gpt-5-mini',
      api_key: undefined,
      reasoning_level: 'minimal',
    });

    expect(providersRuntime.updateCustomTranslation).toHaveBeenCalledWith('custom-gpt', {
      name: 'gpt-5-mini',
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      model: 'gpt-5-mini',
      apiKey: undefined,
      reasoningLevel: 'minimal',
    });
  });

  it('tests custom translation providers through the provider id command', async () => {
    const { initializeProviderStore, useProviderStore } =
      await import('./providerStore');
    initializeProviderStore(providersRuntime as never);

    await useProviderStore.getState().testCustomTranslationProvider('custom-gpt');

    expect(providersRuntime.testCustomTranslation).toHaveBeenCalledWith('custom-gpt');
  });

  it('uses the model as display name for stale custom providers named with their generated id', async () => {
    const { initializeProviderStore, useProviderStore } =
      await import('./providerStore');
    initializeProviderStore(providersRuntime as never);
    providersRuntime.listTranslation.mockResolvedValueOnce([
      {
        id: 'custom-llm-1782661440679036000',
        name: 'custom-llm-1782661440679036000',
        isActive: true,
        isConfigured: true,
        isBuiltin: false,
        requiresApiKey: true,
        protocol: 'openai',
        endpoint: 'https://api.openai.com',
        model: 'gpt-5-mini',
        reasoningLevel: null,
        promptStrategyId: null,
        promptFallbackStrategyId: null,
      },
    ]);

    await useProviderStore.getState().loadTranslationProviders();

    expect(useProviderStore.getState().translationProviders[0].name).toBe('gpt-5-mini');
  });
});
