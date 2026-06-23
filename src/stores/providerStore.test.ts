import { beforeEach, describe, expect, it, vi } from 'vitest';

const providerApi = vi.hoisted(() => ({
  configureTranslationProviderCredentials: vi.fn(),
  configureTranslationProvider: vi.fn(),
  listTranslationProviders: vi.fn(),
}));

vi.mock('../tauri/providers', () => ({
  ...providerApi,
}));

describe('providerStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
});
