import { describe, expect, it, vi } from 'vitest';
import type {
  OcrHistoryEntry,
  ScreenshotFavoriteItem,
  TranslationFavoriteItem,
  TranslationHistoryEntry,
} from './ports';
import { createSettingsLibrary } from './library';

describe('settings library', () => {
  it('globally orders and paginates translation and OCR history', async () => {
    const history = createHistoryPort({
      translations: [translation(1, '2026-07-15T08:00:00Z')],
      ocr: [ocr(2, '2026-07-15T09:00:00Z'), ocr(3, '2026-07-15T07:00:00Z')],
    });
    const index = createIndexPort({
      history: {
        total: 3,
        items: [
          { id: 2, kind: 'ocr', sourceOffset: 0 },
          { id: 1, kind: 'translation', sourceOffset: 0 },
        ],
      },
    });
    const library = createSettingsLibrary(createPorts({ history, index }));

    const page = await library.queryHistory({
      filter: 'all',
      search: 'snap',
      page: 0,
      pageSize: 2,
    });

    expect(page.total).toBe(3);
    expect(page.items.map((item) => item.key)).toEqual([
      'ocr:2',
      'translation:1',
    ]);
    expect(history.queryTranslationHistory).toHaveBeenCalledWith({
      search: 'snap',
      limit: 1,
      offset: 0,
    });
  });

  it('hydrates indexed history by id when concurrent writes shift source offsets', async () => {
    const expected = translation(2, '2026-07-15T08:00:00Z');
    const inserted = translation(3, '2026-07-15T10:00:00Z');
    const history = createHistoryPort({ translations: [inserted, expected] });
    const index = createIndexPort({
      history: {
        total: 1,
        items: [{ id: 2, kind: 'translation', sourceOffset: 0 }],
      },
    });
    const library = createSettingsLibrary(createPorts({ history, index }));

    const page = await library.queryHistory({
      filter: 'all', search: '', page: 0, pageSize: 20,
    });

    expect(page.items.map((item) => item.key)).toEqual(['translation:2']);
  });

  it('forwards a filtered history page without cross-source queries', async () => {
    const history = createHistoryPort({
      translations: [translation(1, '2026-07-15T08:00:00Z')],
    });
    const library = createSettingsLibrary(createPorts({ history }));

    await library.queryHistory({
      filter: 'translation',
      search: '',
      page: 2,
      pageSize: 20,
    });

    expect(history.queryTranslationHistory).toHaveBeenCalledWith({
      search: '',
      limit: 20,
      offset: 40,
    });
    expect(history.queryOcrHistory).not.toHaveBeenCalled();
  });

  it('loads only the final screenshot page selected by the backend index', async () => {
    const screenshots = Array.from({ length: 120 }, (_, index) =>
      screenshot(
        index + 1,
        new Date(Date.UTC(2026, 6, 15, 12, 0, -index)).toISOString(),
      ),
    );
    const screenshotFavorites = createScreenshotFavoritesPort(screenshots);
    const index = createIndexPort({
      favorites: {
        total: 120,
        items: screenshots.slice(100).map((item, offset) => ({
          id: item.id,
          kind: 'screenshot' as const,
          sourceOffset: offset + 100,
        })),
      },
    });
    const library = createSettingsLibrary(
      createPorts({ screenshotFavorites, index }),
    );

    const page = await library.queryFavorites({
      filter: 'all',
      search: '',
      page: 5,
      pageSize: 20,
    });

    expect(page.total).toBe(120);
    expect(page.items).toHaveLength(20);
    expect(screenshotFavorites.queryScreenshotFavorites).toHaveBeenCalledTimes(1);
    expect(screenshotFavorites.queryScreenshotFavorites).toHaveBeenCalledWith({
      search: '',
      limit: 20,
      offset: 100,
    });
  });

  it('globally orders regular and screenshot favorites', async () => {
    const favorites = createFavoritesPort([
      translationFavorite(1, '2026-07-15T08:00:00Z'),
    ]);
    const screenshotFavorites = createScreenshotFavoritesPort([
      screenshot(2, '2026-07-15T09:00:00Z'),
      screenshot(3, '2026-07-15T07:00:00Z'),
    ]);
    const index = createIndexPort({
      favorites: {
        total: 3,
        items: [
          { id: 2, kind: 'screenshot', sourceOffset: 0 },
          { id: 1, kind: 'translation', sourceOffset: 0 },
        ],
      },
    });
    const library = createSettingsLibrary(
      createPorts({ favorites, screenshotFavorites, index }),
    );

    const page = await library.queryFavorites({
      filter: 'all',
      search: '',
      page: 0,
      pageSize: 2,
    });

    expect(page.total).toBe(3);
    expect(page.items.map((item) => item.key)).toEqual([
      'screenshot:2',
      'translation:1',
    ]);
  });

  it('owns history mutation sequencing and OCR copy', async () => {
    const history = createHistoryPort({
      ocr: [ocr(7, '2026-07-15T09:00:00Z')],
    });
    const clipboard = { writeText: vi.fn(async () => undefined) };
    const library = createSettingsLibrary(createPorts({ history, clipboard }));
    const page = await library.queryHistory({
      filter: 'ocr',
      search: '',
      page: 0,
      pageSize: 20,
    });

    await library.deleteHistory(page.items[0]);
    await library.clearHistory('ocr');
    await library.rerunHistoryOcrAndCopy(7);

    expect(history.deleteHistory).toHaveBeenCalledWith(7);
    expect(history.clearHistory).toHaveBeenCalledWith('ocr');
    expect(clipboard.writeText).toHaveBeenCalledWith('recognized');
  });

  it('routes favorite mutations by source behind one interface', async () => {
    const favorites = createFavoritesPort([
      translationFavorite(1, '2026-07-15T08:00:00Z'),
    ]);
    const screenshotFavorites = createScreenshotFavoritesPort([
      screenshot(2, '2026-07-15T09:00:00Z'),
    ]);
    const clipboard = { writeText: vi.fn(async () => undefined) };
    const library = createSettingsLibrary(
      createPorts({ favorites, screenshotFavorites, clipboard }),
    );
    const regular = (
      await library.queryFavorites({
        filter: 'translation',
        search: '',
        page: 0,
        pageSize: 20,
      })
    ).items[0];
    const screenshotItem = (
      await library.queryFavorites({
        filter: 'screenshot',
        search: '',
        page: 0,
        pageSize: 20,
      })
    ).items[0];

    await library.deleteFavorite(regular);
    await library.deleteFavorite(screenshotItem);
    await library.updateFavoriteMetadata(screenshotItem, 'note', ['work']);
    await library.rerunFavoriteOcrAndCopy(1);

    expect(favorites.deleteFavorite).toHaveBeenCalledWith(1);
    expect(screenshotFavorites.deleteScreenshotFavorite).toHaveBeenCalledWith(2);
    expect(
      screenshotFavorites.updateScreenshotFavoriteMetadata,
    ).toHaveBeenCalledWith(2, 'note', ['work']);
    expect(clipboard.writeText).toHaveBeenCalledWith('rerun');
  });
});

function createPorts(overrides: Record<string, unknown> = {}) {
  return {
    history: createHistoryPort(),
    favorites: createFavoritesPort([]),
    screenshotFavorites: createScreenshotFavoritesPort([]),
    index: createIndexPort(),
    clipboard: { writeText: vi.fn() },
    ...overrides,
  };
}

function createFavoritesPort(items: TranslationFavoriteItem[]) {
  return {
    addTranslationFavorite: vi.fn(),
    addOcrFavorite: vi.fn(),
    queryFavorites: vi.fn(async ({ limit, offset }) => ({
      items: items.slice(offset, offset + limit),
      total: items.length,
    })),
    updateFavoriteMetadata: vi.fn(),
    deleteFavorite: vi.fn(),
    rerunOcrFavorite: vi.fn(async () => 'rerun'),
    listFavoriteTags: vi.fn(),
  };
}

function createHistoryPort({
  translations = [],
  ocr: ocrEntries = [],
}: {
  translations?: TranslationHistoryEntry[];
  ocr?: OcrHistoryEntry[];
} = {}) {
  return {
    getTranslationHistory: vi.fn(),
    getOcrHistory: vi.fn(),
    queryTranslationHistory: vi.fn(async ({ limit, offset }) => ({
      items: translations.slice(offset, offset + limit),
      total: translations.length,
    })),
    queryOcrHistory: vi.fn(async ({ limit, offset }) => ({
      items: ocrEntries.slice(offset, offset + limit),
      total: ocrEntries.length,
    })),
    deleteHistory: vi.fn(),
    updateHistoryNote: vi.fn(),
    replaceHistoryTags: vi.fn(),
    clearAllHistory: vi.fn(),
    clearHistory: vi.fn(),
    rerunOcrHistory: vi.fn(async () => 'recognized'),
  };
}

function createIndexPort({
  history = { items: [], total: 0 },
  favorites = { items: [], total: 0 },
}: {
  history?: {
    items: Array<{
      id: number;
      kind: 'translation' | 'ocr' | 'screenshot';
      sourceOffset: number;
    }>;
    total: number;
  };
  favorites?: {
    items: Array<{
      id: number;
      kind: 'translation' | 'ocr' | 'screenshot';
      sourceOffset: number;
    }>;
    total: number;
  };
} = {}) {
  return {
    queryHistoryIndex: vi.fn(async () => history),
    queryFavoriteIndex: vi.fn(async () => favorites),
  };
}

function createScreenshotFavoritesPort(items: ScreenshotFavoriteItem[]) {
  return {
    queryScreenshotFavorites: vi.fn(async ({ limit, offset }) => ({
      items: items.slice(offset, offset + limit),
      total: items.length,
    })),
    updateScreenshotFavoriteMetadata: vi.fn(),
    deleteScreenshotFavorite: vi.fn(),
    copyScreenshotFavorite: vi.fn(),
    revealScreenshotFavorite: vi.fn(),
  };
}

function translation(id: number, timestamp: string): TranslationHistoryEntry {
  return {
    id,
    timestamp,
    note: null,
    tags: [],
    sourceText: `source ${id}`,
    sourceLang: 'en',
    targetLang: 'zh-CN',
    providersUsed: [],
    results: [],
    durationMs: 1,
  };
}

function ocr(id: number, timestamp: string): OcrHistoryEntry {
  return {
    id,
    timestamp,
    note: null,
    tags: [],
    imageHash: `${id}`,
    language: null,
    providerUsed: 'system',
    recognizedText: `ocr ${id}`,
    confidence: null,
    durationMs: 1,
    thumbnailDataUrl: null,
  };
}

function screenshot(id: number, createdAt: string): ScreenshotFavoriteItem {
  return {
    id,
    contentKind: 'screenshot',
    createdAt,
    thumbnailDataUrl: '',
    width: 100,
    height: 100,
    note: null,
    tags: [],
  };
}

function translationFavorite(
  id: number,
  createdAt: string,
): TranslationFavoriteItem {
  return {
    id,
    createdAt,
    sourceHistoryId: null,
    content: {
      contentKind: 'translation',
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
