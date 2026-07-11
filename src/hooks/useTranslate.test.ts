import { beforeEach, describe, expect, it, vi } from 'vitest';

const providersRuntime = vi.hoisted(() => ({
  listTranslation: vi.fn(),
}));

describe('active translation providers for translation sessions', () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    const { initializeProviderStore, useProviderStore } =
      await import('../stores/providerStore');
    initializeProviderStore(providersRuntime as never);
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
    providersRuntime.listTranslation.mockResolvedValueOnce([
      {
        id: 'google-translate',
        name: 'Google Translate',
        isActive: true,
        isConfigured: true,
        isBuiltin: true,
        requiresApiKey: false,
        protocol: null,
        endpoint: null,
        model: null,
        reasoningLevel: null,
        promptStrategyId: null,
        promptFallbackStrategyId: null,
      },
      {
        id: 'custom-gpt',
        name: 'gpt-5-mini',
        isActive: true,
        isConfigured: true,
        isBuiltin: false,
        requiresApiKey: true,
        protocol: null,
        endpoint: null,
        model: 'gpt-5-mini',
      },
      {
        id: 'deeplx',
        name: 'DeepLX',
        isActive: true,
        isConfigured: true,
        isBuiltin: true,
        requiresApiKey: false,
        protocol: null,
        endpoint: null,
        model: null,
        reasoningLevel: null,
        promptStrategyId: null,
        promptFallbackStrategyId: null,
      },
    ]);

    await expect(activeTranslationProviderIds()).resolves.toEqual([
      'google-translate',
      'custom-gpt',
      'deeplx',
    ]);
  });
});
