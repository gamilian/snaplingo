import { ocrFavoriteKey, translationFavoriteKey } from '../favorites/identity';
import type {
  FavoriteItem,
  FavoriteKind,
  OcrHistoryEntry,
  ScreenshotFavoriteItem,
  SettingsClipboardPort,
  SettingsFavoritesPort,
  SettingsHistoryPort,
  SettingsLibraryIndexPort,
  SettingsScreenshotFavoritesPort,
  TranslationHistoryEntry,
} from './ports';

export type LibraryHistoryFilter = 'all' | 'translation' | 'ocr';
export type LibraryFavoritesFilter = 'all' | FavoriteKind | 'screenshot';

export interface LibraryPageRequest<Filter extends string> {
  filter: Filter;
  search: string;
  page: number;
  pageSize: number;
}

export type HistoryLibraryItem =
  | {
      key: string;
      kind: 'translation';
      timestamp: number;
      entry: TranslationHistoryEntry;
    }
  | {
      key: string;
      kind: 'ocr';
      timestamp: number;
      entry: OcrHistoryEntry;
    };

export type FavoriteLibraryItem =
  | {
      key: string;
      kind: 'translation' | 'ocr';
      timestamp: number;
      entry: FavoriteItem;
    }
  | {
      key: string;
      kind: 'screenshot';
      timestamp: number;
      entry: ScreenshotFavoriteItem;
    };

export interface LibraryPage<Item> {
  items: Item[];
  total: number;
}

export interface HistoryLibraryPage extends LibraryPage<HistoryLibraryItem> {
  favoriteKeys: ReadonlySet<string>;
}

export interface SettingsLibrary {
  queryHistory(
    request: LibraryPageRequest<LibraryHistoryFilter>,
  ): Promise<HistoryLibraryPage>;
  queryFavorites(
    request: LibraryPageRequest<LibraryFavoritesFilter>,
  ): Promise<LibraryPage<FavoriteLibraryItem>>;
  deleteHistory(item: HistoryLibraryItem): Promise<void>;
  clearHistory(filter: LibraryHistoryFilter): Promise<void>;
  rerunHistoryOcrAndCopy(id: number): Promise<void>;
  deleteFavorite(item: FavoriteLibraryItem): Promise<void>;
  rerunFavoriteOcrAndCopy(id: number): Promise<void>;
  updateFavoriteMetadata(
    item: FavoriteLibraryItem,
    note: string | null,
    tags: string[],
  ): Promise<void>;
}

export function createSettingsLibrary(ports: {
  history: SettingsHistoryPort;
  favorites: SettingsFavoritesPort;
  screenshotFavorites: SettingsScreenshotFavoritesPort;
  index: SettingsLibraryIndexPort;
  clipboard: SettingsClipboardPort;
}): SettingsLibrary {
  const withFavoriteKeys = async (
    page: LibraryPage<HistoryLibraryItem>,
  ): Promise<HistoryLibraryPage> => {
    const keys = page.items.flatMap((item) =>
      item.kind === 'ocr'
        ? [
            ocrFavoriteKey({
              recognizedText: item.entry.recognizedText,
              language: item.entry.language,
              providerUsed: item.entry.providerUsed,
            }),
          ]
        : item.entry.results.map((result) =>
            translationFavoriteKey({
              sourceText: item.entry.sourceText,
              sourceLang: item.entry.sourceLang,
              targetLang: item.entry.targetLang,
              providerId: result.providerId,
              translatedText: result.translatedText,
            }),
          ),
    );
    return {
      ...page,
      favoriteKeys: new Set(
        keys.length ? await ports.favorites.queryFavoriteKeys(keys) : [],
      ),
    };
  };
  return {
    async queryHistory(request) {
      const offset = request.page * request.pageSize;
      const query = {
        search: request.search,
        limit: request.pageSize,
        offset,
      };

      if (request.filter === 'translation') {
        const page = await ports.history.queryTranslationHistory(query);
        return withFavoriteKeys({
          total: page.total,
          items: page.items.map(toTranslationHistoryItem),
        });
      }
      if (request.filter === 'ocr') {
        const page = await ports.history.queryOcrHistory(query);
        return withFavoriteKeys({
          total: page.total,
          items: page.items.map(toOcrHistoryItem),
        });
      }

      const index = await ports.index.queryHistoryIndex({
        search: request.search,
        limit: request.pageSize,
        offset,
      });
      const translationRefs = index.items.filter(
        (item) => item.kind === 'translation',
      );
      const ocrRefs = index.items.filter((item) => item.kind === 'ocr');
      const [translations, ocr] = await Promise.all([
        loadIndexedItems(
          translationRefs,
          request.search,
          ports.history.queryTranslationHistory,
        ),
        loadIndexedItems(
          ocrRefs,
          request.search,
          ports.history.queryOcrHistory,
        ),
      ]);
      const items = [
        ...translations.map(toTranslationHistoryItem),
        ...ocr.map(toOcrHistoryItem),
      ];
      return withFavoriteKeys({
        total: index.total,
        items: orderByIndex(index.items, items),
      });
    },

    async queryFavorites(request) {
      const offset = request.page * request.pageSize;

      if (request.filter === 'translation' || request.filter === 'ocr') {
        const page = await ports.favorites.queryFavorites({
          kind: request.filter,
          search: request.search,
          limit: request.pageSize,
          offset,
        });
        return {
          total: page.total,
          items: page.items.map(toFavoriteItem),
        };
      }
      if (request.filter === 'screenshot') {
        const page = await ports.screenshotFavorites.queryScreenshotFavorites({
          search: request.search,
          limit: request.pageSize,
          offset,
        });
        return {
          total: page.total,
          items: page.items.map(toScreenshotFavoriteItem),
        };
      }

      const index = await ports.index.queryFavoriteIndex({
        search: request.search,
        limit: request.pageSize,
        offset,
      });
      const favoriteRefs = index.items.filter(
        (item) => item.kind !== 'screenshot',
      );
      const screenshotRefs = index.items.filter(
        (item) => item.kind === 'screenshot',
      );
      const [favorites, screenshots] = await Promise.all([
        loadIndexedItems(
          favoriteRefs,
          request.search,
          ports.favorites.queryFavorites,
        ),
        loadIndexedItems(
          screenshotRefs,
          request.search,
          ports.screenshotFavorites.queryScreenshotFavorites,
        ),
      ]);
      const items = [
        ...favorites.map(toFavoriteItem),
        ...screenshots.map(toScreenshotFavoriteItem),
      ];
      return {
        total: index.total,
        items: orderByIndex(index.items, items),
      };
    },

    deleteHistory: (item) => ports.history.deleteHistory(item.entry.id),

    clearHistory: (filter) =>
      filter === 'all'
        ? ports.history.clearAllHistory()
        : ports.history.clearHistory(filter),

    async rerunHistoryOcrAndCopy(id) {
      const text = await ports.history.rerunOcrHistory(id);
      await ports.clipboard.writeText(text);
    },

    deleteFavorite: (item) =>
      item.kind === 'screenshot'
        ? ports.screenshotFavorites.deleteScreenshotFavorite(item.entry.id)
        : ports.favorites.deleteFavorite(item.entry.id),

    async rerunFavoriteOcrAndCopy(id) {
      const text = await ports.favorites.rerunOcrFavorite(id);
      await ports.clipboard.writeText(text);
    },

    updateFavoriteMetadata: (item, note, tags) =>
      item.kind === 'screenshot'
        ? ports.screenshotFavorites.updateScreenshotFavoriteMetadata(
            item.entry.id,
            note,
            tags,
          )
        : ports.favorites.updateFavoriteMetadata(item.entry.id, note, tags),
  };
}

async function loadIndexedItems<T extends { id: number }>(
  refs: Array<{ id: number }>,
  search: string,
  query: (input: {
    search: string;
    ids: number[];
    limit: number;
    offset: number;
  }) => Promise<{ items: T[]; total: number }>,
): Promise<T[]> {
  if (!refs.length) return [];
  return (
    await query({
      search,
      ids: refs.map((ref) => ref.id),
      limit: refs.length,
      offset: 0,
    })
  ).items;
}

function orderByIndex<T extends { key: string }>(
  index: Array<{ id: number; kind: string }>,
  items: T[],
): T[] {
  const byKey = new Map(items.map((item) => [item.key, item]));
  return index.flatMap((entry) => {
    const item = byKey.get(`${entry.kind}:${entry.id}`);
    return item ? [item] : [];
  });
}

function toTranslationHistoryItem(
  entry: TranslationHistoryEntry,
): HistoryLibraryItem {
  return {
    key: `translation:${entry.id}`,
    kind: 'translation',
    timestamp: new Date(entry.timestamp).getTime(),
    entry,
  };
}

function toOcrHistoryItem(entry: OcrHistoryEntry): HistoryLibraryItem {
  return {
    key: `ocr:${entry.id}`,
    kind: 'ocr',
    timestamp: new Date(entry.timestamp).getTime(),
    entry,
  };
}

function toFavoriteItem(entry: FavoriteItem): FavoriteLibraryItem {
  return {
    key: `${entry.content.contentKind}:${entry.id}`,
    kind: entry.content.contentKind,
    timestamp: new Date(entry.createdAt).getTime(),
    entry,
  };
}

function toScreenshotFavoriteItem(
  entry: ScreenshotFavoriteItem,
): FavoriteLibraryItem {
  return {
    key: `screenshot:${entry.id}`,
    kind: 'screenshot',
    timestamp: new Date(entry.createdAt).getTime(),
    entry,
  };
}
