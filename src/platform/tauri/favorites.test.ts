import { beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('favorites platform adapter', () => {
  beforeEach(() => invoke.mockReset());

  it('creates provider-specific translation and OCR favorite snapshots', async () => {
    const { addOcrFavorite, addTranslationFavorite } = await import('./favorites');
    invoke.mockResolvedValue(8);

    await addTranslationFavorite({
      sourceHistoryId: 4,
      sourceText: 'hello',
      sourceLang: 'en',
      targetLang: 'zh-CN',
      providerId: 'google',
      translatedText: '你好',
    });
    await addOcrFavorite({
      sourceHistoryId: 7,
      recognizedText: 'recognized',
      providerUsed: 'system-ocr',
    });

    expect(invoke).toHaveBeenNthCalledWith(1, 'favorite_translation_result', {
      sourceHistoryId: 4,
      request: { text: 'hello', source_lang: 'en', target_lang: 'zh-CN' },
      result: {
        provider_id: 'google',
        translated_text: '你好',
        detected_language: null,
        confidence: null,
      },
    });
    expect(invoke).toHaveBeenNthCalledWith(2, 'favorite_ocr_result', {
      sourceHistoryId: 7,
      request: { image_data: [], language: null },
      result: { text: 'recognized', confidence: null },
      providerUsed: 'system-ocr',
    });
  });

  it('queries, edits, and deletes independent favorite records', async () => {
    const {
      deleteFavorite,
      listFavoriteTags,
      queryFavorites,
      updateFavoriteMetadata,
    } = await import('./favorites');
    invoke.mockResolvedValue({ items: [], total: 0 });

    await queryFavorites({ kind: 'translation', search: 'hello', limit: 20, offset: 0 });
    await updateFavoriteMetadata(3, 'note', ['work']);
    await deleteFavorite(3);
    await listFavoriteTags('translation');

    expect(invoke).toHaveBeenCalledWith('query_favorites', {
      query: { kind: 'translation', search: 'hello', limit: 20, offset: 0 },
    });
    expect(invoke).toHaveBeenCalledWith('update_favorite_metadata', {
      id: 3,
      note: 'note',
      tags: ['work'],
    });
    expect(invoke).toHaveBeenCalledWith('delete_favorite', { id: 3 });
    expect(invoke).toHaveBeenCalledWith('list_favorite_tags', {
      kind: 'translation',
    });
  });
});
