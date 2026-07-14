import { describe, expect, expectTypeOf, it, vi } from 'vitest';
import type { HistoryEntry } from '../../application/settings/ports';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('Tauri history command adapter', () => {
  it('maps paginated translation history into Settings models', async () => {
    const { getTranslationHistory } = await import('./history');
    invoke.mockResolvedValueOnce([
      {
        id: 1,
        timestamp: '2026-07-11T02:00:00Z',
        favorite: false,
        note: null,
        tags: ['work'],
        source_text: 'hello',
        source_lang: 'en',
        target_lang: 'zh-CN',
        providers_used: ['google-translate'],
        results: [
          {
            provider_id: 'google-translate',
            translated_text: '你好',
            detected_language: null,
            confidence: null,
          },
        ],
        duration_ms: 12,
      },
    ]);

    await expect(getTranslationHistory(20, 40)).resolves.toEqual([
      {
        id: 1,
        timestamp: '2026-07-11T02:00:00Z',
        favorite: false,
        note: null,
        tags: ['work'],
        sourceText: 'hello',
        sourceLang: 'en',
        targetLang: 'zh-CN',
        providersUsed: ['google-translate'],
        results: [
          {
            providerId: 'google-translate',
            translatedText: '你好',
            detectedLanguage: null,
            confidence: null,
          },
        ],
        durationMs: 12,
      },
    ]);
    expect(invoke).toHaveBeenCalledWith('get_translation_history', {
      limit: 20,
      offset: 40,
    });
  });

  it('maps OCR history with nullable fields intact', async () => {
    const { getOcrHistory } = await import('./history');
    invoke.mockResolvedValueOnce([
      {
        id: 2,
        timestamp: '2026-07-11T02:01:00Z',
        favorite: true,
        note: 'keep',
        tags: [],
        image_hash: 'sha256:image',
        language: null,
        provider_used: 'system-ocr',
        recognized_text: 'recognized',
        confidence: null,
        duration_ms: 8,
      },
    ]);

    await expect(getOcrHistory(10, 0)).resolves.toEqual([
      {
        id: 2,
        timestamp: '2026-07-11T02:01:00Z',
        favorite: true,
        note: 'keep',
        tags: [],
        imageHash: 'sha256:image',
        language: null,
        providerUsed: 'system-ocr',
        recognizedText: 'recognized',
        confidence: null,
        durationMs: 8,
      },
    ]);
  });

  it('maps Rust-tagged search variants to Application discriminants', async () => {
    const { searchHistory } = await import('./history');
    invoke.mockResolvedValueOnce([
      {
        type: 'Translation',
        id: 1,
        timestamp: '2026-07-11T02:00:00Z',
        favorite: false,
        note: null,
        tags: [],
        source_text: 'hello',
        source_lang: 'en',
        target_lang: 'zh-CN',
        providers_used: ['google-translate'],
        results: [
          {
            provider_id: 'google-translate',
            translated_text: '你好',
            detected_language: 'en',
            confidence: 0.9,
          },
        ],
        duration_ms: 12,
      },
      {
        type: 'Ocr',
        id: 2,
        timestamp: '2026-07-11T02:01:00Z',
        favorite: false,
        note: null,
        tags: [],
        image_hash: 'sha256:image',
        language: 'en',
        provider_used: 'system-ocr',
        recognized_text: 'hello',
        confidence: 0.95,
        duration_ms: 8,
      },
    ]);

    const entries = await searchHistory('hello');

    expect(invoke).toHaveBeenCalledWith('search_history', { query: 'hello' });
    expectTypeOf(entries).toEqualTypeOf<HistoryEntry[]>();
    expect(entries.map((entry) => entry.type)).toEqual(['translation', 'ocr']);
    if (entries[0].type === 'translation') {
      expect(entries[0].results[0].providerId).toBe('google-translate');
      expectTypeOf(entries[0].results[0].detectedLanguage).toEqualTypeOf<
        string | null
      >();
    }
    if (entries[1].type === 'ocr') {
      expect(entries[1].providerUsed).toBe('system-ocr');
      expectTypeOf(entries[1].confidence).toEqualTypeOf<number | null>();
    }
  });

  it('delegates history metadata and destructive actions', async () => {
    const {
      clearAllHistory,
      clearHistory,
      deleteHistory,
      queryTranslationHistory,
      replaceHistoryTags,
      setHistoryFavorite,
      updateHistoryNote,
    } = await import('./history');
    invoke.mockResolvedValue(undefined);

    invoke.mockResolvedValueOnce({ items: [], total: 0 });
    await queryTranslationHistory({
      search: 'hello',
      favoriteOnly: true,
      limit: 20,
      offset: 0,
    });

    await deleteHistory(7);
    await setHistoryFavorite(7, true);
    await updateHistoryNote(7, 'keep this');
    await replaceHistoryTags(7, ['work']);
    await clearAllHistory();
    await clearHistory('translation');

    expect(invoke).toHaveBeenCalledWith('query_translation_history', {
      query: {
        search: 'hello',
        favoriteOnly: true,
        limit: 20,
        offset: 0,
      },
    });

    expect(invoke).toHaveBeenCalledWith('delete_history', { id: 7 });
    expect(invoke).toHaveBeenCalledWith('set_history_favorite', { id: 7, favorite: true });
    expect(invoke).toHaveBeenCalledWith('update_history_note', { id: 7, note: 'keep this' });
    expect(invoke).toHaveBeenCalledWith('replace_history_tags', { id: 7, tags: ['work'] });
    expect(invoke).toHaveBeenCalledWith('clear_all_history');
    expect(invoke).toHaveBeenCalledWith('clear_history', {
      kind: 'translation',
    });
  });
});
