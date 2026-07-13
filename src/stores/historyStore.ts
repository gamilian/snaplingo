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
  timestamp: number;
  favorite: boolean;
  note?: string;
  tags?: string[];
}

interface HistoryState {
  // 原始后端数据
  rawTranslationEntries: TranslationHistoryEntry[];
  rawOcrEntries: OcrHistoryEntry[];

  // 展开后的视图数据（computed）
  translationHistory: TranslationHistoryItem[];
  ocrHistory: OcrHistoryItem[];

  // 翻译历史
  loadTranslationHistory: (limit?: number, offset?: number) => Promise<void>;
  addTranslationHistory: (item: Omit<TranslationHistoryItem, 'id' | 'timestamp' | 'favorite' | 'entryId' | 'resultIndex'>) => void;
  deleteTranslationHistory: (id: string) => Promise<void>;
  deleteTranslationEntry: (entryId: number) => Promise<void>; // 删除整个 entry
  toggleTranslationFavorite: (id: string) => Promise<void>;
  updateTranslationNote: (id: string, note: string) => Promise<void>;
  clearTranslationHistory: () => Promise<void>;

  // OCR 历史
  loadOcrHistory: (limit?: number, offset?: number) => Promise<void>;
  addOcrHistory: (item: Omit<OcrHistoryItem, 'id' | 'timestamp' | 'favorite'>) => void;
  deleteOcrHistory: (id: string) => Promise<void>;
  toggleOcrFavorite: (id: string) => Promise<void>;
  updateOcrNote: (id: string, note: string) => Promise<void>;
  clearOcrHistory: () => Promise<void>;
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
    language: entry.language || 'Unknown',
    timestamp: new Date(entry.timestamp).getTime(),
    favorite: entry.favorite,
    note: entry.note ?? undefined,
    tags: entry.tags,
  }));
}

export const useHistoryStore = create<HistoryState>((set, get) => ({
      // 原始数据
      rawTranslationEntries: [],
      rawOcrEntries: [],

      // 展开后的视图数据
      translationHistory: [],
      ocrHistory: [],

      // 从后端加载翻译历史
      loadTranslationHistory: async (limit = 100, offset = 0) => {
        try {
          const entries = await runtime().loadTranslation(limit, offset);
          set({
            rawTranslationEntries: entries,
            translationHistory: flattenTranslationEntries(entries),
          });
        } catch (error) {
          console.error('Failed to load translation history:', error);
        }
      },

      // 从后端加载 OCR 历史
      loadOcrHistory: async (limit = 100, offset = 0) => {
        try {
          const entries = await runtime().loadOcr(limit, offset);
          set({
            rawOcrEntries: entries,
            ocrHistory: flattenOcrEntries(entries),
          });
        } catch (error) {
          console.error('Failed to load OCR history:', error);
        }
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
        const item = get().translationHistory.find((entry) => entry.id === id);
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
        }));
      },

      clearTranslationHistory: async () => {
        try {
          await runtime().clear();
          set({
            rawTranslationEntries: [],
            rawOcrEntries: [],
            translationHistory: [],
            ocrHistory: [],
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
        const item = get().ocrHistory.find((entry) => entry.id === id);
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
        }));
      },

      clearOcrHistory: async () => {
        try {
          await runtime().clear();
          set({
            rawTranslationEntries: [],
            rawOcrEntries: [],
            translationHistory: [],
            ocrHistory: [],
          });
        } catch (error) {
          console.error('Failed to clear OCR history:', error);
          throw error;
        }
      },
}));
