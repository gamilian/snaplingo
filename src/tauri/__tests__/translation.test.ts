import { describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('translation tauri adapter', () => {
  it('maps auto source language to null for translate_text_v2', async () => {
    const { translateText } = await import('../translation');
    invoke.mockResolvedValueOnce([{ provider_id: 'google', translated_text: '你好' }]);

    await translateText({ text: 'hello', sourceLang: 'auto', targetLang: 'zh-CN' });

    expect(invoke).toHaveBeenCalledWith('translate_text_v2', {
      request: {
        text: 'hello',
        source_lang: null,
        target_lang: 'zh-CN',
      },
    });
  });
});
