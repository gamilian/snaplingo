import { beforeEach, describe, expect, it, vi } from 'vitest';
import { translationFavoriteKey } from '../application/favorites/identity';

const favoritesRuntime = vi.hoisted(() => ({
  addTranslation: vi.fn(),
  addOcr: vi.fn(),
  query: vi.fn(),
  updateMetadata: vi.fn(),
  delete: vi.fn(),
  rerunOcr: vi.fn(),
  listTags: vi.fn(),
}));

describe('favoritesStore identities', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  it('hydrates translation favorite state by content instead of history id', async () => {
    favoritesRuntime.query.mockResolvedValueOnce({
      total: 1,
      items: [
        {
          id: 7,
          createdAt: '2026-07-14T00:00:00Z',
          sourceHistoryId: null,
          content: {
            contentKind: 'translation',
            snapshot: {
              sourceText: 'hello',
              sourceLang: 'en',
              targetLang: 'zh-CN',
              result: {
                provider_id: 'google',
                translated_text: '你好',
                detected_language: 'en',
                confidence: null,
              },
            },
          },
          note: null,
          tags: [],
          thumbnailDataUrl: null,
        },
      ],
    });
    const { initializeFavoritesStore, useFavoritesStore } =
      await import('./favoritesStore');
    initializeFavoritesStore(favoritesRuntime);

    await useFavoritesStore.getState().hydrateKeys('translation');

    expect(useFavoritesStore.getState().keys).toContain(
      translationFavoriteKey({
        sourceText: 'hello',
        sourceLang: 'en',
        targetLang: 'zh-CN',
        providerId: 'google',
        translatedText: '你好',
      }),
    );
  });
});
