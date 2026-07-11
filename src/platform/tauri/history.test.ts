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

  it('delegates destructive history actions', async () => {
    const { clearAllHistory, deleteHistory } = await import('./history');
    invoke.mockResolvedValue(undefined);

    await deleteHistory(7);
    await clearAllHistory();

    expect(invoke).toHaveBeenCalledWith('delete_history', { id: 7 });
    expect(invoke).toHaveBeenCalledWith('clear_all_history');
  });
});
