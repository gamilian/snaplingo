import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';

// 后端返回的原始结构
interface BackendTranslationEntry {
  id: number;
  timestamp: string;
  source_text: string;
  source_lang: string;
  target_lang: string;
  providers_used: string[];
  results: Array<{
    provider_id?: string;
    translated_text: string;
    detected_language?: string;
    confidence?: number;
  }>;
  duration_ms: number;
}

interface BackendOcrEntry {
  id: number;
  timestamp: string;
  recognized_text: string;
  language?: string;
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
  rawTranslationEntries: BackendTranslationEntry[];
  rawOcrEntries: BackendOcrEntry[];

  // 展开后的视图数据（computed）
  translationHistory: TranslationHistoryItem[];
  ocrHistory: OcrHistoryItem[];

  // 翻译历史
  loadTranslationHistory: (limit?: number, offset?: number) => Promise<void>;
  addTranslationHistory: (item: Omit<TranslationHistoryItem, 'id' | 'timestamp' | 'favorite' | 'entryId' | 'resultIndex'>) => void;
  deleteTranslationHistory: (id: string) => Promise<void>;
  deleteTranslationEntry: (entryId: number) => Promise<void>; // 删除整个 entry
  toggleTranslationFavorite: (id: string) => void;
  updateTranslationNote: (id: string, note: string) => void;
  clearTranslationHistory: () => Promise<void>;

  // OCR 历史
  loadOcrHistory: (limit?: number, offset?: number) => Promise<void>;
  addOcrHistory: (item: Omit<OcrHistoryItem, 'id' | 'timestamp' | 'favorite'>) => void;
  deleteOcrHistory: (id: string) => Promise<void>;
  toggleOcrFavorite: (id: string) => void;
  updateOcrNote: (id: string, note: string) => void;
  clearOcrHistory: () => Promise<void>;
}

// 辅助函数：将后端 entries 展平为前端历史记录
function flattenTranslationEntries(entries: BackendTranslationEntry[]): TranslationHistoryItem[] {
  return entries.flatMap((entry) => {
    return entry.results.map((result, index) => ({
      id: `${entry.id}-${index}`,
      entryId: entry.id,
      resultIndex: index,
      type: 'input' as const,
      sourceText: entry.source_text,
      targetText: result.translated_text,
      sourceLang: entry.source_lang,
      targetLang: entry.target_lang,
      provider: result.provider_id || entry.providers_used[index] || 'Unknown',
      timestamp: new Date(entry.timestamp).getTime(),
      favorite: false,
    }));
  });
}

export const useHistoryStore = create<HistoryState>()(
  persist(
    (set, get) => ({
      // 原始数据
      rawTranslationEntries: [],
      rawOcrEntries: [],

      // 展开后的视图数据
      translationHistory: [],
      ocrHistory: [],

      // 从后端加载翻译历史
      loadTranslationHistory: async (limit = 100, offset = 0) => {
        try {
          const entries = await invoke<BackendTranslationEntry[]>('get_translation_history', { limit, offset });
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
          const entries = await invoke<BackendOcrEntry[]>('get_ocr_history', { limit, offset });
          const converted = entries.map((entry) => ({
            id: String(entry.id),
            type: 'screenshot' as const,
            text: entry.recognized_text,
            language: entry.language || 'Unknown',
            timestamp: new Date(entry.timestamp).getTime(),
            favorite: false,
          }));
          set({
            rawOcrEntries: entries,
            ocrHistory: converted,
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

          await invoke('delete_history', { id: entryId });

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
          await invoke('delete_history', { id: entryId });

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

      toggleTranslationFavorite: (id) =>
        set((state) => ({
          translationHistory: state.translationHistory.map((item) =>
            item.id === id ? { ...item, favorite: !item.favorite } : item
          ),
        })),

      updateTranslationNote: (id, note) =>
        set((state) => ({
          translationHistory: state.translationHistory.map((item) =>
            item.id === id ? { ...item, note } : item
          ),
        })),

      clearTranslationHistory: async () => {
        try {
          await invoke('clear_all_history');
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
          await invoke('delete_history', { id: dbId });

          const newRawEntries = get().rawOcrEntries.filter(e => e.id !== dbId);
          const converted = newRawEntries.map((entry) => ({
            id: String(entry.id),
            type: 'screenshot' as const,
            text: entry.recognized_text,
            language: entry.language || 'Unknown',
            timestamp: new Date(entry.timestamp).getTime(),
            favorite: false,
          }));

          set({
            rawOcrEntries: newRawEntries,
            ocrHistory: converted,
          });
        } catch (error) {
          console.error('Failed to delete OCR history:', error);
          throw error;
        }
      },

      toggleOcrFavorite: (id) =>
        set((state) => ({
          ocrHistory: state.ocrHistory.map((item) =>
            item.id === id ? { ...item, favorite: !item.favorite } : item
          ),
        })),

      updateOcrNote: (id, note) =>
        set((state) => ({
          ocrHistory: state.ocrHistory.map((item) =>
            item.id === id ? { ...item, note } : item
          ),
        })),

      clearOcrHistory: async () => {
        try {
          await invoke('clear_all_history');
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
    }),
    {
      name: 'snaplingo-history',
    }
  )
);
