import { beforeEach, describe, expect, it, vi } from 'vitest';

const historyRuntime = vi.hoisted(() => ({
  loadTranslation: vi.fn(),
  loadOcr: vi.fn(),
  queryTranslation: vi.fn(),
  queryOcr: vi.fn(),
  deleteEntry: vi.fn(),
  setFavorite: vi.fn(),
  updateNote: vi.fn(),
  replaceTags: vi.fn(),
  clear: vi.fn(),
  clearKind: vi.fn(),
  rerunOcr: vi.fn(),
  exportTranslationFavorites: vi.fn(),
  listTags: vi.fn(),
}));

describe('historyStore', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  it('loads and flattens translation history through the injected runtime', async () => {
    historyRuntime.loadTranslation.mockResolvedValueOnce([
      {
        id: 7,
        timestamp: '2026-07-11T03:00:00Z',
        favorite: false,
        note: null,
        tags: [],
        sourceText: 'hello',
        sourceLang: 'en',
        targetLang: 'zh-CN',
        providersUsed: ['provider-1'],
        results: [
          {
            providerId: 'provider-1',
            translatedText: '你好',
            detectedLanguage: 'en',
            confidence: 1,
          },
        ],
        durationMs: 12,
      },
    ]);
    const { initializeHistoryStore, useHistoryStore } =
      await import('./historyStore');
    initializeHistoryStore(historyRuntime);

    await useHistoryStore.getState().loadTranslationHistory(20, 40);

    expect(historyRuntime.loadTranslation).toHaveBeenCalledWith(20, 40);
    expect(useHistoryStore.getState().translationHistory).toMatchObject([
      {
        id: '7-0',
        sourceText: 'hello',
        targetText: '你好',
        provider: 'provider-1',
      },
    ]);
  });

  it('persists a favorite change through the injected runtime', async () => {
    historyRuntime.loadTranslation.mockResolvedValueOnce([
      {
        id: 9,
        timestamp: '2026-07-11T03:00:00Z',
        favorite: false,
        note: null,
        tags: [],
        sourceText: 'hello',
        sourceLang: 'en',
        targetLang: 'zh-CN',
        providersUsed: ['provider-1'],
        results: [
          {
            providerId: 'provider-1',
            translatedText: '你好',
            detectedLanguage: 'en',
            confidence: 1,
          },
        ],
        durationMs: 12,
      },
    ]);
    const { initializeHistoryStore, useHistoryStore } =
      await import('./historyStore');
    initializeHistoryStore(historyRuntime);

    await useHistoryStore.getState().loadTranslationHistory();
    await useHistoryStore.getState().toggleTranslationFavorite('9-0');

    expect(historyRuntime.setFavorite).toHaveBeenCalledWith(9, true);
    expect(useHistoryStore.getState().translationHistory[0]?.favorite).toBe(true);
  });

  it('loads translation favorites independently from the paged history list', async () => {
    historyRuntime.queryTranslation.mockResolvedValueOnce({
      items: [
        {
          id: 11,
          timestamp: '2026-07-11T03:00:00Z',
          favorite: true,
          note: 'keep',
          tags: ['work'],
          sourceText: 'favorite source',
          sourceLang: 'en',
          targetLang: 'zh-CN',
          providersUsed: ['provider-1'],
          results: [
            {
              providerId: 'provider-1',
              translatedText: '收藏译文',
              detectedLanguage: 'en',
              confidence: 1,
            },
          ],
          durationMs: 12,
        },
      ],
      total: 1,
    });
    const { initializeHistoryStore, useHistoryStore } =
      await import('./historyStore');
    initializeHistoryStore(historyRuntime);

    await useHistoryStore.getState().loadTranslationFavorites({
      search: 'favorite',
      limit: 20,
      offset: 0,
    });

    expect(historyRuntime.queryTranslation).toHaveBeenCalledWith({
      search: 'favorite',
      favoriteOnly: true,
      limit: 20,
      offset: 0,
    });
    expect(useHistoryStore.getState().translationFavorites).toMatchObject([
      { entryId: 11, favorite: true, tags: ['work'] },
    ]);
    expect(useHistoryStore.getState().translationFavoritesTotal).toBe(1);
  });

  it('clears only the requested history kind', async () => {
    historyRuntime.clearKind.mockResolvedValueOnce(undefined);
    const { initializeHistoryStore, useHistoryStore } =
      await import('./historyStore');
    initializeHistoryStore(historyRuntime);

    await useHistoryStore.getState().clearTranslationHistory();

    expect(historyRuntime.clearKind).toHaveBeenCalledWith('translation');
    expect(historyRuntime.clear).not.toHaveBeenCalled();
  });
});
