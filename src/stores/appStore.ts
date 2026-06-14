import { create } from 'zustand';
import { TranslationResult } from '../types';

interface AppState {
  sourceText: string;
  sourceLang: string;
  targetLang: string;
  translations: TranslationResult[];
  isTranslating: boolean;
  resultWindowVisible: boolean;

  setSourceText: (text: string) => void;
  setSourceLang: (lang: string) => void;
  setTargetLang: (lang: string) => void;
  setTranslations: (results: TranslationResult[]) => void;
  setTranslating: (value: boolean) => void;
  showResultWindow: () => void;
  hideResultWindow: () => void;
  reset: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  sourceText: '',
  sourceLang: 'auto',
  targetLang: 'zh-CN',
  translations: [],
  isTranslating: false,
  resultWindowVisible: false,

  setSourceText: (text) => set({ sourceText: text }),
  setSourceLang: (lang) => set({ sourceLang: lang }),
  setTargetLang: (lang) => set({ targetLang: lang }),
  setTranslations: (results) => set({ translations: results }),
  setTranslating: (value) => set({ isTranslating: value }),
  showResultWindow: () => set({ resultWindowVisible: true }),
  hideResultWindow: () => set({ resultWindowVisible: false }),
  reset: () => set({
    sourceText: '',
    sourceLang: 'auto',
    targetLang: 'zh-CN',
    translations: [],
    isTranslating: false,
  }),
}));
