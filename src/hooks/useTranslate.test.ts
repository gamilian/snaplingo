import { beforeEach, describe, expect, it, vi } from 'vitest';

const providerApi = vi.hoisted(() => ({
  listTranslationProviders: vi.fn(),
}));

vi.mock('../tauri/providers', () => ({
  listTranslationProviders: providerApi.listTranslationProviders,
}));

describe('active translation providers for translation sessions', () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    const { useProviderStore } = await import('../stores/providerStore');
    useProviderStore.setState({
      translationProviders: [],
      activeTranslationProviders: [],
    });
  });

  it('refreshes active providers before starting a translation session', async () => {
    const { activeTranslationProviderIds } = await import('./useTranslate');
    const { useProviderStore } = await import('../stores/providerStore');
    useProviderStore.setState({
      activeTranslationProviders: ['google-translate'],
    });
    providerApi.listTranslationProviders.mockResolvedValueOnce([
      {
        id: 'google-translate',
        name: 'Google Translate',
        is_active: true,
        is_configured: true,
        is_builtin: true,
        requires_api_key: false,
      },
      {
        id: 'custom-gpt',
        name: 'gpt-5-mini',
        is_active: true,
        is_configured: true,
        is_builtin: false,
        requires_api_key: true,
        model: 'gpt-5-mini',
      },
      {
        id: 'deeplx',
        name: 'DeepLX',
        is_active: true,
        is_configured: true,
        is_builtin: true,
        requires_api_key: false,
      },
    ]);

    await expect(activeTranslationProviderIds()).resolves.toEqual([
      'google-translate',
      'custom-gpt',
      'deeplx',
    ]);
  });
});
