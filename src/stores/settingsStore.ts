import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type {
  MainTab,
  OcrSubTab,
  ScreenshotSubTab,
  ServicesSubTab,
  TranslationSubTab,
} from '../components/SettingsWindow/navigationModel';

interface HotkeyMap {
  [key: string]: string;
}

type HotkeyCategory = 'screenshot' | 'translation' | 'ocr';

interface SettingsState {
  activeMainTab: MainTab;
  screenshotSubTab: ScreenshotSubTab;
  translationSubTab: TranslationSubTab;
  ocrSubTab: OcrSubTab;
  servicesSubTab: ServicesSubTab;
  hotkeys: {
    screenshot: HotkeyMap;
    translation: HotkeyMap;
    ocr: HotkeyMap;
  };

  setActiveMainTab: (tab: MainTab) => void;
  setScreenshotSubTab: (tab: SettingsState['screenshotSubTab']) => void;
  setTranslationSubTab: (tab: SettingsState['translationSubTab']) => void;
  setOcrSubTab: (tab: SettingsState['ocrSubTab']) => void;
  setServicesSubTab: (tab: SettingsState['servicesSubTab']) => void;
  setHotkey: (category: HotkeyCategory, key: string, value: string) => void;
  clearHotkey: (category: HotkeyCategory, key: string) => void;
  resetHotkeys: (category: HotkeyCategory) => void;
}

// 默认快捷键
export const DEFAULT_HOTKEYS = {
  screenshot: {
    screenshot: '⇧⌘R',
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

interface PersistedSettingsState {
  activeMainTab?: MainTab;
  screenshotSubTab?: ScreenshotSubTab;
  translationSubTab?: TranslationSubTab;
  ocrSubTab?: OcrSubTab;
  servicesSubTab?: ServicesSubTab;
  hotkeys?: Partial<Record<HotkeyCategory, HotkeyMap>>;
}

function mergeHotkeys(
  category: HotkeyCategory,
  hotkeys?: Partial<Record<HotkeyCategory, HotkeyMap>>,
) {
  return {
    ...DEFAULT_HOTKEYS[category],
    ...(hotkeys?.[category] ?? {}),
  };
}

function mergePersistedState(state: SettingsState, persistedState: unknown): SettingsState {
  const persisted = (persistedState ?? {}) as PersistedSettingsState;

  return {
    ...state,
    activeMainTab: persisted.activeMainTab ?? state.activeMainTab,
    screenshotSubTab: persisted.screenshotSubTab ?? state.screenshotSubTab,
    translationSubTab: persisted.translationSubTab ?? state.translationSubTab,
    ocrSubTab: persisted.ocrSubTab ?? state.ocrSubTab,
    servicesSubTab: persisted.servicesSubTab ?? state.servicesSubTab,
    hotkeys: {
      screenshot: mergeHotkeys('screenshot', persisted.hotkeys),
      translation: mergeHotkeys('translation', persisted.hotkeys),
      ocr: mergeHotkeys('ocr', persisted.hotkeys),
    },
  };
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      activeMainTab: 'screenshot',
      screenshotSubTab: 'hotkeys',
      translationSubTab: 'hotkeys',
      ocrSubTab: 'hotkeys',
      servicesSubTab: 'ocr',
      hotkeys: DEFAULT_HOTKEYS,
      setActiveMainTab: (tab) => set({ activeMainTab: tab }),
      setScreenshotSubTab: (tab) => set({ screenshotSubTab: tab }),
      setTranslationSubTab: (tab) => set({ translationSubTab: tab }),
      setOcrSubTab: (tab) => set({ ocrSubTab: tab }),
      setServicesSubTab: (tab) => set({ servicesSubTab: tab }),

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
            [category]: { ...DEFAULT_HOTKEYS[category] },
          },
        })),
    }),
    {
      name: 'snaplingo-settings',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        activeMainTab: state.activeMainTab,
        screenshotSubTab: state.screenshotSubTab,
        translationSubTab: state.translationSubTab,
        ocrSubTab: state.ocrSubTab,
        servicesSubTab: state.servicesSubTab,
        hotkeys: state.hotkeys,
      }),
      merge: (persistedState, currentState) =>
        mergePersistedState(currentState, persistedState),
    }
  )
);
