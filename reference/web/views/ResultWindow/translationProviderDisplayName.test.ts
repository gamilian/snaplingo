import { describe, expect, it } from 'vitest';
import type { Provider } from '../../stores/providerStore';
import { getTranslationProviderDisplayName } from './translationProviderDisplayName';

describe('getTranslationProviderDisplayName', () => {
  it('uses the configured custom provider name instead of the generated provider id', () => {
    const providers: Provider[] = [
      {
        id: 'custom-llm-1782661440679036000',
        name: '我的 GPT',
        type: 'translation',
        status: 'active',
        isBuiltin: false,
        requiresApiKey: true,
      },
    ];

    expect(
      getTranslationProviderDisplayName(
        'custom-llm-1782661440679036000',
        providers,
      ),
    ).toBe('我的 GPT');
  });

  it('falls back to the provider id when provider metadata is not loaded', () => {
    expect(getTranslationProviderDisplayName('google-translate', [])).toBe(
      'google-translate',
    );
  });
});
