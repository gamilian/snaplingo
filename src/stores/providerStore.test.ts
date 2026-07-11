import { beforeEach, describe, expect, it, vi } from 'vitest';

const providerApi = vi.hoisted(() => ({
  configureOcrProvider: vi.fn(),
  configureOcrProviderCredentials: vi.fn(),
  configureTranslationProviderCredentials: vi.fn(),
  updateCustomTranslationProvider: vi.fn(),
  testCustomTranslationProvider: vi.fn(),
  listOcrProviders: vi.fn(),
  listTranslationProviders: vi.fn(),
}));

vi.mock('../tauri/providers', () => ({
  ...providerApi,
}));

describe('providerStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    providerApi.listOcrProviders.mockResolvedValue([]);
    providerApi.listTranslationProviders.mockResolvedValue([]);
  });

  it('passes DeepLX standard DeepL mode credentials through the credential map command', async () => {
    const { useProviderStore } = await import('./providerStore');

    await useProviderStore.getState().updateProviderConfig('translation', 'deeplx', {
      mode: 'deepl',
      api_key: 'secret',
    });

    expect(providerApi.configureTranslationProviderCredentials).toHaveBeenCalledWith('deeplx', {
      mode: 'deepl',
      api_key: 'secret',
    });
  });

  it('configures OCR providers through the credential map command', async () => {
    const { useProviderStore } = await import('./providerStore');

    await useProviderStore.getState().configureOcrProvider('baidu-ocr', {
      api_key: 'key',
      secret_key: 'secret',
    });

    expect(providerApi.configureOcrProviderCredentials).toHaveBeenCalledWith('baidu-ocr', {
      api_key: 'key',
      secret_key: 'secret',
    });
    expect(providerApi.configureOcrProvider).not.toHaveBeenCalled();
  });

  it('updates custom translation providers through the custom provider command', async () => {
    const { useProviderStore } = await import('./providerStore');

    await useProviderStore.getState().updateCustomTranslationProvider('custom-gpt', {
      name: 'gpt-5-mini',
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      model: 'gpt-5-mini',
      api_key: undefined,
      reasoning_level: 'minimal',
    });

    expect(providerApi.updateCustomTranslationProvider).toHaveBeenCalledWith('custom-gpt', {
      name: 'gpt-5-mini',
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      model: 'gpt-5-mini',
      api_key: undefined,
      reasoning_level: 'minimal',
    });
  });

  it('tests custom translation providers through the provider id command', async () => {
    const { useProviderStore } = await import('./providerStore');

    await useProviderStore.getState().testCustomTranslationProvider('custom-gpt');

    expect(providerApi.testCustomTranslationProvider).toHaveBeenCalledWith('custom-gpt');
  });

  it('uses the model as display name for stale custom providers named with their generated id', async () => {
    const { useProviderStore } = await import('./providerStore');
    providerApi.listTranslationProviders.mockResolvedValueOnce([
      {
        id: 'custom-llm-1782661440679036000',
        name: 'custom-llm-1782661440679036000',
        is_active: true,
        is_configured: true,
        is_builtin: false,
        requires_api_key: true,
        protocol: 'openai',
        endpoint: 'https://api.openai.com',
        model: 'gpt-5-mini',
        reasoning_level: undefined,
      },
    ]);

    await useProviderStore.getState().loadTranslationProviders();

    expect(useProviderStore.getState().translationProviders[0].name).toBe('gpt-5-mini');
  });
});
