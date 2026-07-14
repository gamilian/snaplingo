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

export interface TranslationHistoryResultItem {
  providerId: string;
  translatedText: string;
  detectedLanguage: string | null;
  confidence: number | null;
}

export interface TranslationHistoryItem {
  id: number;
  sourceText: string;
  sourceLang: string;
  targetLang: string;
  results: TranslationHistoryResultItem[];
  timestamp: number;
  note?: string;
  tags: string[];
}

export interface OcrHistoryItem {
  id: string;
  text: string;
  imageHash: string;
  imageThumbnail?: string;
  language: string;
  providerUsed: string;
  confidence: number | null;
  timestamp: number;
  note?: string;
  tags: string[];
}

interface HistoryListOptions {
  search: string;
  tag?: string;
  limit: number;
  offset: number;
}

interface HistoryState {
  revision: number;
  invalidate(): void;
  translationHistory: TranslationHistoryItem[];
  ocrHistory: OcrHistoryItem[];
  translationHistoryTotal: number;
  ocrHistoryTotal: number;
  loadTranslationHistory(limit?: number, offset?: number): Promise<void>;
  queryTranslationHistory(options: HistoryListOptions): Promise<void>;
  deleteTranslationHistory(id: number): Promise<void>;
  clearTranslationHistory(): Promise<void>;
  loadOcrHistory(limit?: number, offset?: number): Promise<void>;
  queryOcrHistory(options: HistoryListOptions): Promise<void>;
  deleteOcrHistory(id: string): Promise<void>;
  clearOcrHistory(): Promise<void>;
}

function toTranslationItems(
  entries: TranslationHistoryEntry[],
): TranslationHistoryItem[] {
  return entries.map((entry) => ({
    id: entry.id,
    sourceText: entry.sourceText,
    sourceLang: entry.sourceLang,
    targetLang: entry.targetLang,
    results: entry.results.map((result, index) => ({
      providerId:
        result.providerId || entry.providersUsed[index] || 'Unknown',
      translatedText: result.translatedText,
      detectedLanguage: result.detectedLanguage,
      confidence: result.confidence,
    })),
    timestamp: new Date(entry.timestamp).getTime(),
    note: entry.note ?? undefined,
    tags: entry.tags,
  }));
}

function toOcrItems(entries: OcrHistoryEntry[]): OcrHistoryItem[] {
  return entries.map((entry) => ({
    id: String(entry.id),
    text: entry.recognizedText,
    imageHash: entry.imageHash,
    imageThumbnail: entry.thumbnailDataUrl ?? undefined,
    language: entry.language || 'Unknown',
    providerUsed: entry.providerUsed,
    confidence: entry.confidence,
    timestamp: new Date(entry.timestamp).getTime(),
    note: entry.note ?? undefined,
    tags: entry.tags,
  }));
}

export const useHistoryStore = create<HistoryState>((set) => ({
  revision: 0,
  invalidate: () => set((state) => ({ revision: state.revision + 1 })),
  translationHistory: [],
  ocrHistory: [],
  translationHistoryTotal: 0,
  ocrHistoryTotal: 0,

  loadTranslationHistory: async (limit = 100, offset = 0) => {
    const entries = await runtime().loadTranslation(limit, offset);
    set({
      translationHistory: toTranslationItems(entries),
      translationHistoryTotal: entries.length,
    });
  },

  queryTranslationHistory: async ({ search, tag, limit, offset }) => {
    const page = await runtime().queryTranslation({
      search,
      tag,
      limit,
      offset,
    });
    set({
      translationHistory: toTranslationItems(page.items),
      translationHistoryTotal: page.total,
    });
  },

  deleteTranslationHistory: async (id) => {
    await runtime().deleteEntry(id);
    set((state) => ({
      translationHistory: state.translationHistory.filter(
        (entry) => entry.id !== id,
      ),
      translationHistoryTotal: Math.max(0, state.translationHistoryTotal - 1),
    }));
  },

  clearTranslationHistory: async () => {
    await runtime().clearKind('translation');
    set({ translationHistory: [], translationHistoryTotal: 0 });
  },

  loadOcrHistory: async (limit = 100, offset = 0) => {
    const entries = await runtime().loadOcr(limit, offset);
    set({ ocrHistory: toOcrItems(entries), ocrHistoryTotal: entries.length });
  },

  queryOcrHistory: async ({ search, tag, limit, offset }) => {
    const page = await runtime().queryOcr({
      search,
      tag,
      limit,
      offset,
    });
    set({ ocrHistory: toOcrItems(page.items), ocrHistoryTotal: page.total });
  },

  deleteOcrHistory: async (id) => {
    await runtime().deleteEntry(Number(id));
    set((state) => ({
      ocrHistory: state.ocrHistory.filter((entry) => entry.id !== id),
      ocrHistoryTotal: Math.max(0, state.ocrHistoryTotal - 1),
    }));
  },

  clearOcrHistory: async () => {
    await runtime().clearKind('ocr');
    set({ ocrHistory: [], ocrHistoryTotal: 0 });
  },
}));
