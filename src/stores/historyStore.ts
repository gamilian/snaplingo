import { create } from 'zustand';
import type {
  OcrHistoryEntry,
  TranslationHistoryEntry,
} from '../application/settings/ports';
import type { SettingsRuntime } from '../application/settings/runtime';

type HistoryRuntime = SettingsRuntime['history'];

let historyRuntime: HistoryRuntime | null = null;

export function initializeHistoryStore(runtime: HistoryRuntime) {
  historyRuntime = runtime;
}

function runtime() {
  if (!historyRuntime) {
    throw new Error('History store runtime has not been initialized');
  }

  return historyRuntime;
}

// 前端展示用的扁平结构
export interface TranslationHistoryItem {
  id: string;
  entryId: number; // 后端 entry ID
  resultIndex: number; // 在该 entry 的哪个结果
  type: 'selection' | 'screenshot' | 'input';
  sourceText: string;
  targetText: string;
  sourceLang: string;
  targetLang: string;
  provider: string;
  timestamp: number;
  favorite: boolean;
  note?: string;
  tags?: string[];
}

export interface OcrHistoryItem {
  id: string;
  type: 'screenshot' | 'silent' | 'file';
  text: string;
  imageThumbnail?: string;
  language: string;
  providerUsed: string;
  confidence: number | null;
  timestamp: number;
  favorite: boolean;
  note?: string;
  tags?: string[];
}

interface HistoryState {
  revision: number;
  invalidate: () => void;
  // 原始后端数据
  rawTranslationEntries: TranslationHistoryEntry[];
  rawOcrEntries: OcrHistoryEntry[];

  // 展开后的视图数据（computed）
  translationHistory: TranslationHistoryItem[];
  ocrHistory: OcrHistoryItem[];
  translationFavorites: TranslationHistoryItem[];
  ocrFavorites: OcrHistoryItem[];
  translationHistoryTotal: number;
  ocrHistoryTotal: number;
  translationFavoritesTotal: number;
  ocrFavoritesTotal: number;

  // 翻译历史
  loadTranslationHistory: (limit?: number, offset?: number) => Promise<void>;
  queryTranslationHistory: (options: HistoryListOptions) => Promise<void>;
  loadTranslationFavorites: (options: HistoryListOptions) => Promise<void>;
  addTranslationHistory: (item: Omit<TranslationHistoryItem, 'id' | 'timestamp' | 'favorite' | 'entryId' | 'resultIndex'>) => void;
  deleteTranslationHistory: (id: string) => Promise<void>;
  deleteTranslationEntry: (entryId: number) => Promise<void>; // 删除整个 entry
  toggleTranslationFavorite: (id: string) => Promise<void>;
  updateTranslationNote: (id: string, note: string) => Promise<void>;
  updateTranslationTags: (id: string, tags: string[]) => Promise<void>;
  clearTranslationHistory: () => Promise<void>;

  // OCR 历史
  loadOcrHistory: (limit?: number, offset?: number) => Promise<void>;
  queryOcrHistory: (options: HistoryListOptions) => Promise<void>;
  loadOcrFavorites: (options: HistoryListOptions) => Promise<void>;
  addOcrHistory: (item: Omit<OcrHistoryItem, 'id' | 'timestamp' | 'favorite'>) => void;
  deleteOcrHistory: (id: string) => Promise<void>;
  toggleOcrFavorite: (id: string) => Promise<void>;
  updateOcrNote: (id: string, note: string) => Promise<void>;
  updateOcrTags: (id: string, tags: string[]) => Promise<void>;
  clearOcrHistory: () => Promise<void>;
}

interface HistoryListOptions {
  search: string;
  tag?: string;
  limit: number;
  offset: number;
}

// 辅助函数：将后端 entries 展平为前端历史记录
function flattenTranslationEntries(entries: TranslationHistoryEntry[]): TranslationHistoryItem[] {
  return entries.flatMap((entry) => {
    return entry.results.map((result, index) => ({
      id: `${entry.id}-${index}`,
      entryId: entry.id,
      resultIndex: index,
      type: 'input' as const,
      sourceText: entry.sourceText,
      targetText: result.translatedText,
      sourceLang: entry.sourceLang,
      targetLang: entry.targetLang,
      provider: result.providerId || entry.providersUsed[index] || 'Unknown',
      timestamp: new Date(entry.timestamp).getTime(),
      favorite: entry.favorite,
      note: entry.note ?? undefined,
      tags: entry.tags,
    }));
  });
}

function flattenOcrEntries(entries: OcrHistoryEntry[]): OcrHistoryItem[] {
  return entries.map((entry) => ({
    id: String(entry.id),
    type: 'screenshot' as const,
    text: entry.recognizedText,
    imageThumbnail: entry.thumbnailDataUrl ?? undefined,
    language: entry.language || 'Unknown',
    providerUsed: entry.providerUsed,
    confidence: entry.confidence,
    timestamp: new Date(entry.timestamp).getTime(),
    favorite: entry.favorite,
    note: entry.note ?? undefined,
    tags: entry.tags,
  }));
}

export const useHistoryStore = create<HistoryState>((set, get) => ({
      revision: 0,
      invalidate: () => set((state) => ({ revision: state.revision + 1 })),
      // 原始数据
      rawTranslationEntries: [],
      rawOcrEntries: [],

      // 展开后的视图数据
      translationHistory: [],
      ocrHistory: [],
      translationFavorites: [],
      ocrFavorites: [],
      translationHistoryTotal: 0,
      ocrHistoryTotal: 0,
      translationFavoritesTotal: 0,
      ocrFavoritesTotal: 0,

      // 从后端加载翻译历史
      loadTranslationHistory: async (limit = 100, offset = 0) => {
        try {
          const entries = await runtime().loadTranslation(limit, offset);
          set({
            rawTranslationEntries: entries,
            translationHistory: flattenTranslationEntries(entries),
            translationHistoryTotal: entries.length,
          });
        } catch (error) {
          console.error('Failed to load translation history:', error);
        }
      },

      queryTranslationHistory: async ({ search, tag, limit, offset }) => {
        const page = await runtime().queryTranslation({
          search,
          tag,
          favoriteOnly: false,
          limit,
          offset,
        });
        set({
          rawTranslationEntries: page.items,
          translationHistory: flattenTranslationEntries(page.items),
          translationHistoryTotal: page.total,
        });
      },

      loadTranslationFavorites: async ({ search, tag, limit, offset }) => {
        const page = await runtime().queryTranslation({
          search,
          tag,
          favoriteOnly: true,
          limit,
          offset,
        });
        set({
          translationFavorites: flattenTranslationEntries(page.items),
          translationFavoritesTotal: page.total,
        });
      },

      // 从后端加载 OCR 历史
      loadOcrHistory: async (limit = 100, offset = 0) => {
        try {
          const entries = await runtime().loadOcr(limit, offset);
          set({
            rawOcrEntries: entries,
            ocrHistory: flattenOcrEntries(entries),
            ocrHistoryTotal: entries.length,
          });
        } catch (error) {
          console.error('Failed to load OCR history:', error);
        }
      },

      queryOcrHistory: async ({ search, tag, limit, offset }) => {
        const page = await runtime().queryOcr({
          search,
          tag,
          favoriteOnly: false,
          limit,
          offset,
        });
        set({
          rawOcrEntries: page.items,
          ocrHistory: flattenOcrEntries(page.items),
          ocrHistoryTotal: page.total,
        });
      },

      loadOcrFavorites: async ({ search, tag, limit, offset }) => {
        const page = await runtime().queryOcr({
          search,
          tag,
          favoriteOnly: true,
          limit,
          offset,
        });
        set({
          ocrFavorites: flattenOcrEntries(page.items),
          ocrFavoritesTotal: page.total,
        });
      },

      // 翻译历史
      addTranslationHistory: (item) =>
        set((state) => ({
          translationHistory: [
            {
              ...item,
              id: `t-${Date.now()}`,
              entryId: -1, // 临时 ID
              resultIndex: 0,
              timestamp: Date.now(),
              favorite: false,
            },
            ...state.translationHistory,
          ],
        })),

      // 删除单个展示项（删除整个 entry）
      deleteTranslationHistory: async (id: string) => {
        try {
          // Parse and validate ID format (expected: "entryId-resultIndex")
          const parts = id.split('-');
          if (parts.length !== 2) {
            console.error(`Invalid history ID format: expected "number-number", got "${id}"`);
            throw new Error(`Invalid history ID format: ${id}`);
          }

          const entryId = parseInt(parts[0]);
          const resultIndex = parseInt(parts[1]);

          if (isNaN(entryId) || isNaN(resultIndex)) {
            console.error(`Invalid history ID: cannot parse numbers from "${id}"`);
            throw new Error(`Invalid history ID: ${id}`);
          }

          await runtime().deleteEntry(entryId);

          // 从原始数据中移除
          const newRawEntries = get().rawTranslationEntries.filter(e => e.id !== entryId);
          set({
            rawTranslationEntries: newRawEntries,
            translationHistory: flattenTranslationEntries(newRawEntries),
          });
        } catch (error) {
          console.error('Failed to delete translation history:', error);
          throw error;
        }
      },

      // 删除整个 entry（提供给需要明确删除整个翻译请求的场景）
      deleteTranslationEntry: async (entryId: number) => {
        try {
          await runtime().deleteEntry(entryId);

          const newRawEntries = get().rawTranslationEntries.filter(e => e.id !== entryId);
          set({
            rawTranslationEntries: newRawEntries,
            translationHistory: flattenTranslationEntries(newRawEntries),
          });
        } catch (error) {
          console.error('Failed to delete translation entry:', error);
          throw error;
        }
      },

      toggleTranslationFavorite: async (id) => {
        const item = [
          ...get().translationHistory,
          ...get().translationFavorites,
        ].find((entry) => entry.id === id);
        if (!item) return;

        const favorite = !item.favorite;
        await runtime().setFavorite(item.entryId, favorite);
        set((state) => ({
          rawTranslationEntries: state.rawTranslationEntries.map((entry) =>
            entry.id === item.entryId ? { ...entry, favorite } : entry
          ),
          translationHistory: state.translationHistory.map((entry) =>
            entry.entryId === item.entryId ? { ...entry, favorite } : entry
          ),
          translationFavorites: favorite
            ? mergeTranslationFavorite(state.translationFavorites, item)
            : state.translationFavorites.filter(
                (entry) => entry.entryId !== item.entryId,
              ),
          translationFavoritesTotal: Math.max(
            0,
            state.translationFavoritesTotal + (favorite ? 1 : -1),
          ),
        }));
      },

      updateTranslationNote: async (id, note) => {
        const item = get().translationHistory.find((entry) => entry.id === id);
        if (!item) return;

        const persistedNote = note.trim() || null;
        await runtime().updateNote(item.entryId, persistedNote);
        set((state) => ({
          rawTranslationEntries: state.rawTranslationEntries.map((entry) =>
            entry.id === item.entryId ? { ...entry, note: persistedNote } : entry
          ),
          translationHistory: state.translationHistory.map((entry) =>
            entry.entryId === item.entryId ? { ...entry, note: persistedNote ?? undefined } : entry
          ),
          translationFavorites: state.translationFavorites.map((entry) =>
            entry.entryId === item.entryId
              ? { ...entry, note: persistedNote ?? undefined }
              : entry,
          ),
        }));
      },

      updateTranslationTags: async (id, tags) => {
        const item = [...get().translationHistory, ...get().translationFavorites].find(
          (entry) => entry.id === id,
        );
        if (!item) return;
        await runtime().replaceTags(item.entryId, tags);
        set((state) => ({
          rawTranslationEntries: state.rawTranslationEntries.map((entry) =>
            entry.id === item.entryId ? { ...entry, tags } : entry,
          ),
          translationHistory: state.translationHistory.map((entry) =>
            entry.entryId === item.entryId ? { ...entry, tags } : entry,
          ),
          translationFavorites: state.translationFavorites.map((entry) =>
            entry.entryId === item.entryId ? { ...entry, tags } : entry,
          ),
        }));
      },

      clearTranslationHistory: async () => {
        try {
          await runtime().clearKind('translation');
          set({
            rawTranslationEntries: [],
            translationHistory: [],
            translationFavorites: [],
            translationHistoryTotal: 0,
            translationFavoritesTotal: 0,
          });
        } catch (error) {
          console.error('Failed to clear history:', error);
          throw error;
        }
      },

      // OCR 历史
      addOcrHistory: (item) =>
        set((state) => ({
          ocrHistory: [
            {
              ...item,
              id: `o-${Date.now()}`,
              timestamp: Date.now(),
              favorite: false,
            },
            ...state.ocrHistory,
          ],
        })),

      deleteOcrHistory: async (id: string) => {
        try {
          const dbId = parseInt(id);
          await runtime().deleteEntry(dbId);

          const newRawEntries = get().rawOcrEntries.filter(e => e.id !== dbId);
          set({
            rawOcrEntries: newRawEntries,
            ocrHistory: flattenOcrEntries(newRawEntries),
          });
        } catch (error) {
          console.error('Failed to delete OCR history:', error);
          throw error;
        }
      },

      toggleOcrFavorite: async (id) => {
        const item = [...get().ocrHistory, ...get().ocrFavorites].find(
          (entry) => entry.id === id,
        );
        if (!item) return;

        const historyId = Number(id);
        const favorite = !item.favorite;
        await runtime().setFavorite(historyId, favorite);
        set((state) => ({
          rawOcrEntries: state.rawOcrEntries.map((entry) =>
            entry.id === historyId ? { ...entry, favorite } : entry
          ),
          ocrHistory: state.ocrHistory.map((entry) =>
            entry.id === id ? { ...entry, favorite } : entry
          ),
          ocrFavorites: favorite
            ? mergeOcrFavorite(state.ocrFavorites, item)
            : state.ocrFavorites.filter((entry) => entry.id !== id),
          ocrFavoritesTotal: Math.max(
            0,
            state.ocrFavoritesTotal + (favorite ? 1 : -1),
          ),
        }));
      },

      updateOcrNote: async (id, note) => {
        const item = get().ocrHistory.find((entry) => entry.id === id);
        if (!item) return;

        const historyId = Number(id);
        const persistedNote = note.trim() || null;
        await runtime().updateNote(historyId, persistedNote);
        set((state) => ({
          rawOcrEntries: state.rawOcrEntries.map((entry) =>
            entry.id === historyId ? { ...entry, note: persistedNote } : entry
          ),
          ocrHistory: state.ocrHistory.map((entry) =>
            entry.id === id ? { ...entry, note: persistedNote ?? undefined } : entry
          ),
          ocrFavorites: state.ocrFavorites.map((entry) =>
            entry.id === id
              ? { ...entry, note: persistedNote ?? undefined }
              : entry,
          ),
        }));
      },

      updateOcrTags: async (id, tags) => {
        const item = [...get().ocrHistory, ...get().ocrFavorites].find(
          (entry) => entry.id === id,
        );
        if (!item) return;
        const historyId = Number(id);
        await runtime().replaceTags(historyId, tags);
        set((state) => ({
          rawOcrEntries: state.rawOcrEntries.map((entry) =>
            entry.id === historyId ? { ...entry, tags } : entry,
          ),
          ocrHistory: state.ocrHistory.map((entry) =>
            entry.id === id ? { ...entry, tags } : entry,
          ),
          ocrFavorites: state.ocrFavorites.map((entry) =>
            entry.id === id ? { ...entry, tags } : entry,
          ),
        }));
      },

      clearOcrHistory: async () => {
        try {
          await runtime().clearKind('ocr');
          set({
            rawOcrEntries: [],
            ocrHistory: [],
            ocrFavorites: [],
            ocrHistoryTotal: 0,
            ocrFavoritesTotal: 0,
          });
        } catch (error) {
          console.error('Failed to clear OCR history:', error);
          throw error;
        }
      },
}));

function mergeTranslationFavorite(
  favorites: TranslationHistoryItem[],
  item: TranslationHistoryItem,
) {
  if (favorites.some((entry) => entry.id === item.id)) return favorites;
  return [{ ...item, favorite: true }, ...favorites];
}

function mergeOcrFavorite(favorites: OcrHistoryItem[], item: OcrHistoryItem) {
  if (favorites.some((entry) => entry.id === item.id)) return favorites;
  return [{ ...item, favorite: true }, ...favorites];
}
