import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface TranslationHistoryItem {
  id: string;
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
  translationHistory: TranslationHistoryItem[];
  ocrHistory: OcrHistoryItem[];

  // 翻译历史
  addTranslationHistory: (item: Omit<TranslationHistoryItem, 'id' | 'timestamp' | 'favorite'>) => void;
  deleteTranslationHistory: (id: string) => void;
  toggleTranslationFavorite: (id: string) => void;
  updateTranslationNote: (id: string, note: string) => void;
  clearTranslationHistory: () => void;

  // OCR 历史
  addOcrHistory: (item: Omit<OcrHistoryItem, 'id' | 'timestamp' | 'favorite'>) => void;
  deleteOcrHistory: (id: string) => void;
  toggleOcrFavorite: (id: string) => void;
  updateOcrNote: (id: string, note: string) => void;
  clearOcrHistory: () => void;
}

// 示例数据
const sampleTranslationHistory: TranslationHistoryItem[] = [
  {
    id: 't1',
    type: 'selection',
    sourceText: 'Hello, world!',
    targetText: '你好，世界！',
    sourceLang: 'en',
    targetLang: 'zh-CN',
    provider: 'Google 翻译',
    timestamp: Date.now() - 3600000,
    favorite: false,
  },
  {
    id: 't2',
    type: 'screenshot',
    sourceText: 'The quick brown fox jumps over the lazy dog.',
    targetText: '敏捷的棕色狐狸跳过了懒狗。',
    sourceLang: 'en',
    targetLang: 'zh-CN',
    provider: 'DeepL',
    timestamp: Date.now() - 7200000,
    favorite: true,
  },
  {
    id: 't3',
    type: 'input',
    sourceText: '今天天气真好',
    targetText: 'The weather is really nice today',
    sourceLang: 'zh-CN',
    targetLang: 'en',
    provider: 'Google 翻译',
    timestamp: Date.now() - 86400000,
    favorite: false,
  },
];

const sampleOcrHistory: OcrHistoryItem[] = [
  {
    id: 'o1',
    type: 'screenshot',
    text: '这是一段识别出来的文字内容，可以用于测试 OCR 历史记录功能。',
    language: '中文简体',
    timestamp: Date.now() - 1800000,
    favorite: false,
  },
  {
    id: 'o2',
    type: 'file',
    text: 'This is recognized text from an image file.',
    language: 'English',
    timestamp: Date.now() - 5400000,
    favorite: true,
  },
];

export const useHistoryStore = create<HistoryState>()(
  persist(
    (set) => ({
      translationHistory: sampleTranslationHistory,
      ocrHistory: sampleOcrHistory,

      // 翻译历史
      addTranslationHistory: (item) =>
        set((state) => ({
          translationHistory: [
            {
              ...item,
              id: `t-${Date.now()}`,
              timestamp: Date.now(),
              favorite: false,
            },
            ...state.translationHistory,
          ],
        })),

      deleteTranslationHistory: (id) =>
        set((state) => ({
          translationHistory: state.translationHistory.filter((item) => item.id !== id),
        })),

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

      clearTranslationHistory: () =>
        set({ translationHistory: [] }),

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

      deleteOcrHistory: (id) =>
        set((state) => ({
          ocrHistory: state.ocrHistory.filter((item) => item.id !== id),
        })),

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

      clearOcrHistory: () =>
        set({ ocrHistory: [] }),
    }),
    {
      name: 'snaplingo-history',
    }
  )
);
