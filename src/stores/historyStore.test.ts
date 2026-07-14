import { beforeEach, describe, expect, it, vi } from 'vitest';

const historyRuntime = vi.hoisted(() => ({
  loadTranslation: vi.fn(),
  loadOcr: vi.fn(),
  queryTranslation: vi.fn(),
  queryOcr: vi.fn(),
  deleteEntry: vi.fn(),
  updateNote: vi.fn(),
  replaceTags: vi.fn(),
  clear: vi.fn(),
  clearKind: vi.fn(),
  rerunOcr: vi.fn(),
}));

describe('historyStore', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  it('keeps one translation session as one history item', async () => {
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
        providersUsed: ['provider-1', 'provider-2'],
        results: [
          {
            providerId: 'provider-1',
            translatedText: '你好',
            detectedLanguage: 'en',
            confidence: 1,
          },
          {
            providerId: 'provider-2',
            translatedText: '您好',
            detectedLanguage: 'en',
            confidence: 0.9,
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
        id: 7,
        sourceText: 'hello',
        results: [
          { providerId: 'provider-1', translatedText: '你好' },
          { providerId: 'provider-2', translatedText: '您好' },
        ],
      },
    ]);
  });

  it('deletes the aggregate translation entry by its real history id', async () => {
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
    await useHistoryStore.getState().deleteTranslationHistory(9);

    expect(historyRuntime.deleteEntry).toHaveBeenCalledWith(9);
    expect(useHistoryStore.getState().translationHistory).toEqual([]);
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
