import { describe, expect, expectTypeOf, it, vi } from 'vitest';
import type { BackendHistoryEntry } from './history';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('Tauri history command adapter', () => {
  it('loads paginated translation history', async () => {
    const { getTranslationHistory } = await import('./history');
    const expected = [
      {
        id: 1,
        timestamp: '2026-07-11T02:00:00Z',
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
    ];
    invoke.mockResolvedValueOnce(expected);

    await expect(getTranslationHistory(20, 40)).resolves.toEqual(expected);
    expect(invoke).toHaveBeenCalledWith('get_translation_history', {
      limit: 20,
      offset: 40,
    });
  });

  it('loads OCR history with all serialized backend fields', async () => {
    const { getOcrHistory } = await import('./history');
    const expected = [
      {
        id: 2,
        timestamp: '2026-07-11T02:01:00Z',
        image_hash: 'sha256:image',
        language: null,
        provider_used: 'system-ocr',
        recognized_text: 'recognized',
        confidence: null,
        duration_ms: 8,
      },
    ];
    invoke.mockResolvedValueOnce(expected);

    await expect(getOcrHistory(10, 0)).resolves.toEqual(expected);
    expect(invoke).toHaveBeenCalledWith('get_ocr_history', {
      limit: 10,
      offset: 0,
    });
  });

  it('types search results as the Rust-tagged history variants', async () => {
    const { searchHistory } = await import('./history');
    invoke.mockResolvedValueOnce([
      {
        type: 'Translation',
        id: 1,
        timestamp: '2026-07-11T02:00:00Z',
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
    expectTypeOf(entries).toEqualTypeOf<BackendHistoryEntry[]>();
    expect(entries.map((entry) => entry.type)).toEqual(['Translation', 'Ocr']);
    if (entries[0].type === 'Translation') {
      expect(entries[0].results[0].provider_id).toBe('google-translate');
      expectTypeOf(entries[0].results[0].detected_language).toEqualTypeOf<
        string | null
      >();
    }
    if (entries[1].type === 'Ocr') {
      expect(entries[1].provider_used).toBe('system-ocr');
      expectTypeOf(entries[1].confidence).toEqualTypeOf<number | null>();
    }
  });

  it('deletes a history entry by id', async () => {
    const { deleteHistory } = await import('./history');
    invoke.mockResolvedValueOnce(undefined);
    await deleteHistory(7);
    expect(invoke).toHaveBeenCalledWith('delete_history', { id: 7 });
  });
});
