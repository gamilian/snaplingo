import { describe, expect, it, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('Tauri translation command adapter', () => {
  it('maps auto source language to null for translate_text_v2', async () => {
    const { translateText } = await import('./translation');
    invoke.mockResolvedValueOnce([
      { provider_id: 'google', translated_text: '你好' },
    ]);

    await translateText({
      text: 'hello',
      sourceLang: 'auto',
      targetLang: 'zh-CN',
    });

    expect(invoke).toHaveBeenCalledWith('translate_text_v2', {
      request: {
        text: 'hello',
        source_lang: null,
        target_lang: 'zh-CN',
      },
    });
  });

  it('records one translation history entry with all provider results', async () => {
    const { recordTranslationHistory } = await import('./translation');
    invoke.mockResolvedValueOnce(undefined);

    await recordTranslationHistory({
      text: 'hello',
      sourceLang: 'auto',
      targetLang: 'zh-CN',
      results: [
        {
          provider_id: 'google',
          translated_text: '你好',
          detected_language: 'en',
          confidence: null,
        },
      ],
      durationMs: 42,
    });

    expect(invoke).toHaveBeenCalledWith('record_translation_history', {
      request: {
        text: 'hello',
        source_lang: null,
        target_lang: 'zh-CN',
      },
      results: [
        {
          provider_id: 'google',
          translated_text: '你好',
          detected_language: 'en',
          confidence: null,
        },
      ],
      durationMs: 42,
    });
  });
});
