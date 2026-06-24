import { beforeEach, describe, expect, it, vi } from 'vitest';

const providerApi = vi.hoisted(() => ({
  configureOcrProvider: vi.fn(),
  configureOcrProviderCredentials: vi.fn(),
  configureTranslationProviderCredentials: vi.fn(),
  configureTranslationProvider: vi.fn(),
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

  it('normalizes legacy apiKey config to credential map', async () => {
    const { useProviderStore } = await import('./providerStore');

    await useProviderStore.getState().updateProviderConfig('translation', 'deepl', {
      apiKey: 'secret',
    });

    expect(providerApi.configureTranslationProviderCredentials).toHaveBeenCalledWith('deepl', {
      api_key: 'secret',
    });
    expect(providerApi.configureTranslationProvider).not.toHaveBeenCalled();
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
});
