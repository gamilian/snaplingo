import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ProviderConfigurationState } from '../application/settings/configuration';

describe('providerStore projection', () => {
  beforeEach(() => vi.resetModules());

  it('projects Application state and forwards provider intents', async () => {
    const provider = {
      id: 'system-ocr',
      name: 'System OCR',
      type: 'ocr' as const,
      status: 'active' as const,
      isBuiltin: true,
      requiresApiKey: false,
    };
    let listener!: (state: {
      ocrProviders: typeof provider[];
      translationProviders: typeof provider[];
      activeOcrProvider: string | null;
      activeTranslationProviders: string[];
    }) => void;
    const configuration = {
      getState: (): ProviderConfigurationState => ({
        ocrProviders: [],
        translationProviders: [],
        activeOcrProvider: null,
        activeTranslationProviders: [],
      }),
      subscribe: vi.fn((next) => {
        listener = next;
        return () => undefined;
      }),
      loadTranslation: vi.fn(async () => []),
      loadOcr: vi.fn(async () => []),
      refresh: vi.fn(async () => undefined),
      activateTranslation: vi.fn(async () => undefined),
      deactivateTranslation: vi.fn(async () => undefined),
      addCustomTranslation: vi.fn(async () => undefined),
      updateCustomTranslation: vi.fn(async () => undefined),
      removeTranslation: vi.fn(async () => undefined),
      testCustomTranslation: vi.fn(async () => undefined),
      configureTranslation: vi.fn(async () => undefined),
      reorderTranslation: vi.fn(async () => undefined),
      activateOcr: vi.fn(async () => undefined),
      configureOcr: vi.fn(async () => undefined),
    };
    const { initializeProviderStore, useProviderStore } =
      await import('./providerStore');
    initializeProviderStore(configuration);

    listener({
      ocrProviders: [provider],
      translationProviders: [],
      activeOcrProvider: provider.id,
      activeTranslationProviders: [],
    });
    await useProviderStore.getState().activateOcrProvider(provider.id);

    expect(useProviderStore.getState()).toMatchObject({
      ocrProviders: [provider],
      activeOcrProvider: provider.id,
    });
    expect(configuration.activateOcr).toHaveBeenCalledWith(provider.id);
  });
});
