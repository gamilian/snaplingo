import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type {
  MainTab,
  OcrSubTab,
  ScreenshotSubTab,
  ServicesSubTab,
  TranslationSubTab,
} from '../views/SettingsWindow/navigationModel';

interface SettingsState {
  activeMainTab: MainTab;
  screenshotSubTab: ScreenshotSubTab;
  translationSubTab: TranslationSubTab;
  ocrSubTab: OcrSubTab;
  servicesSubTab: ServicesSubTab;

  setActiveMainTab: (tab: MainTab) => void;
  setScreenshotSubTab: (tab: SettingsState['screenshotSubTab']) => void;
  setTranslationSubTab: (tab: SettingsState['translationSubTab']) => void;
  setOcrSubTab: (tab: SettingsState['ocrSubTab']) => void;
  setServicesSubTab: (tab: SettingsState['servicesSubTab']) => void;
}

interface PersistedSettingsState {
  activeMainTab?: MainTab;
  screenshotSubTab?: ScreenshotSubTab;
  translationSubTab?: TranslationSubTab;
  ocrSubTab?: OcrSubTab;
  servicesSubTab?: ServicesSubTab;
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
      setActiveMainTab: (tab) => set({ activeMainTab: tab }),
      setScreenshotSubTab: (tab) => set({ screenshotSubTab: tab }),
      setTranslationSubTab: (tab) => set({ translationSubTab: tab }),
      setOcrSubTab: (tab) => set({ ocrSubTab: tab }),
      setServicesSubTab: (tab) => set({ servicesSubTab: tab }),
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
      }),
      merge: (persistedState, currentState) =>
        mergePersistedState(currentState, persistedState),
    }
  )
);
