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

  it('hydrates favorite keys beyond the first 1000 records', async () => {
    const firstPage = Array.from({ length: 1000 }, (_, id) => favorite(id + 1));
    favoritesRuntime.query
      .mockResolvedValueOnce({ total: 1001, items: firstPage })
      .mockResolvedValueOnce({ total: 1001, items: [favorite(1001)] });
    const { initializeFavoritesStore, useFavoritesStore } =
      await import('./favoritesStore');
    initializeFavoritesStore(favoritesRuntime);

    await useFavoritesStore.getState().hydrateKeys('translation');

    expect(favoritesRuntime.query).toHaveBeenNthCalledWith(2, {
      kind: 'translation',
      limit: 1000,
      offset: 1000,
    });
    expect(useFavoritesStore.getState().keys).toContain(
      translationFavoriteKey({
        sourceText: 'source 1001',
        sourceLang: 'en',
        targetLang: 'zh-CN',
        providerId: 'test',
        translatedText: 'translated 1001',
      }),
    );
  });
});

function favorite(id: number) {
  return {
    id,
    createdAt: '2026-07-14T00:00:00Z',
    sourceHistoryId: null,
    content: {
      contentKind: 'translation' as const,
      snapshot: {
        sourceText: `source ${id}`,
        sourceLang: 'en',
        targetLang: 'zh-CN',
        result: {
          provider_id: 'test',
          translated_text: `translated ${id}`,
          detected_language: 'en',
          confidence: null,
        },
      },
    },
    note: null,
    tags: [],
    thumbnailDataUrl: null,
  };
}
