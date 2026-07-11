import { beforeEach, describe, expect, it, vi } from 'vitest';

const historyRuntime = vi.hoisted(() => ({
  loadTranslation: vi.fn(),
  loadOcr: vi.fn(),
  deleteEntry: vi.fn(),
  clear: vi.fn(),
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
});
