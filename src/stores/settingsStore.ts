import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type MainTab = 'screenshot' | 'translation' | 'ocr' | 'services' | 'general' | 'advanced';

interface HotkeyMap {
  [key: string]: string;
}

interface SettingsState {
  // 窗口状态
  activeMainTab: MainTab;

  // 截图子页面
  screenshotSubTab: 'hotkeys' | 'save-settings' | 'editor' | 'favorites';

  // 翻译子页面
  translationSubTab: 'hotkeys' | 'translation-settings' | 'history' | 'favorites';

  // OCR 子页面
  ocrSubTab: 'hotkeys' | 'ocr-settings' | 'history' | 'favorites';

  // 快捷键配置
  hotkeys: {
    screenshot: HotkeyMap;
    translation: HotkeyMap;
    ocr: HotkeyMap;
  };

  // 通用设置
  language: string;
  theme: 'light' | 'dark' | 'system';
  startOnBoot: boolean;

  // 截图设置
  screenshotSavePath: string;
  screenshotFormat: 'png' | 'jpg' | 'webp';
  screenshotQuality: number;

  // 翻译设置
  defaultSourceLang: string;
  defaultTargetLang: string;

  // Actions
  setActiveMainTab: (tab: MainTab) => void;
  setScreenshotSubTab: (tab: SettingsState['screenshotSubTab']) => void;
  setTranslationSubTab: (tab: SettingsState['translationSubTab']) => void;
  setOcrSubTab: (tab: SettingsState['ocrSubTab']) => void;

  setHotkey: (category: 'screenshot' | 'translation' | 'ocr', key: string, value: string) => void;
  clearHotkey: (category: 'screenshot' | 'translation' | 'ocr', key: string) => void;
  resetHotkeys: (category: 'screenshot' | 'translation' | 'ocr') => void;

  setLanguage: (lang: string) => void;
  setTheme: (theme: SettingsState['theme']) => void;
  setStartOnBoot: (value: boolean) => void;

  setScreenshotSavePath: (path: string) => void;
  setScreenshotFormat: (format: SettingsState['screenshotFormat']) => void;
  setScreenshotQuality: (quality: number) => void;
}

// 默认快捷键
const defaultHotkeys = {
  screenshot: {
    screenshot: 'F1',
    'screenshot-copy': '⌘F1',
    'screenshot-custom': '⇧F1',
    pin: 'F3',
    'pin-toggle-all': '⇧F3',
    'pin-switch-group': '⌘F3',
  },
  translation: {
    'selection-translate': '⌥D',
    'screenshot-translate': '⌥S',
    'input-translate': '⌥A',
    'show-window': '未设置',
  },
  ocr: {
    'screenshot-ocr': '⇧⌥S',
    'silent-screenshot-ocr': '未设置',
    'file-ocr': '未设置',
    'show-window': '未设置',
  },
};

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      // 初始状态
      activeMainTab: 'screenshot',
      screenshotSubTab: 'hotkeys',
      translationSubTab: 'hotkeys',
      ocrSubTab: 'hotkeys',

      hotkeys: defaultHotkeys,

      language: 'zh-CN',
      theme: 'system',
      startOnBoot: false,

      screenshotSavePath: '~/Pictures/SnapLingo',
      screenshotFormat: 'png',
      screenshotQuality: 90,

      defaultSourceLang: 'auto',
      defaultTargetLang: 'zh-CN',

      // Actions
      setActiveMainTab: (tab) => set({ activeMainTab: tab }),
      setScreenshotSubTab: (tab) => set({ screenshotSubTab: tab }),
      setTranslationSubTab: (tab) => set({ translationSubTab: tab }),
      setOcrSubTab: (tab) => set({ ocrSubTab: tab }),

      setHotkey: (category, key, value) =>
        set((state) => ({
          hotkeys: {
            ...state.hotkeys,
            [category]: {
              ...state.hotkeys[category],
              [key]: value,
            },
          },
        })),

      clearHotkey: (category, key) =>
        set((state) => ({
          hotkeys: {
            ...state.hotkeys,
            [category]: {
              ...state.hotkeys[category],
              [key]: '未设置',
            },
          },
        })),

      resetHotkeys: (category) =>
        set((state) => ({
          hotkeys: {
            ...state.hotkeys,
            [category]: defaultHotkeys[category],
          },
        })),

      setLanguage: (lang) => set({ language: lang }),
      setTheme: (theme) => set({ theme }),
      setStartOnBoot: (value) => set({ startOnBoot: value }),

      setScreenshotSavePath: (path) => set({ screenshotSavePath: path }),
      setScreenshotFormat: (format) => set({ screenshotFormat: format }),
      setScreenshotQuality: (quality) => set({ screenshotQuality: quality }),
    }),
    {
      name: 'snaplingo-settings',
    }
  )
);
