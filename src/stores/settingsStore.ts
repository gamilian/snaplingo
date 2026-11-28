import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type { MainTab } from '../views/SettingsWindow/navigationModel';
import type { SettingsNavigationRequest } from '../application/settings/navigation';

interface SettingsState {
  activeMainTab: MainTab;
  requestedSection: string | null;

  setActiveMainTab: (tab: MainTab) => void;
  navigate: (request: SettingsNavigationRequest) => void;
  consumeRequestedSection: () => void;
}

interface PersistedSettingsState {
  activeMainTab?: MainTab;
}

function mergePersistedState(state: SettingsState, persistedState: unknown): SettingsState {
  const persisted = (persistedState ?? {}) as PersistedSettingsState;
  const activeMainTab = isMainTab(persisted.activeMainTab)
    ? persisted.activeMainTab
    : state.activeMainTab;

  return {
    ...state,
    activeMainTab,
    requestedSection: null,
  };
}

function isMainTab(value: unknown): value is MainTab {
  return [
    'overview',
    'general',
    'screenshot',
    'translation',
    'ocr',
    'services',
    'favorites',
    'history',
  ].includes(String(value));
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      activeMainTab: 'overview',
      requestedSection: null,
      setActiveMainTab: (tab) =>
        set({ activeMainTab: tab, requestedSection: null }),
      navigate: ({ tab, section }) =>
        set({ activeMainTab: section === 'hotkeys' ? 'overview' : tab, requestedSection: section === 'hotkeys' ? tab : section ?? null }),
      consumeRequestedSection: () => set({ requestedSection: null }),
    }),
    {
      name: 'snaplingo-settings',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        activeMainTab: state.activeMainTab,
      }),
      merge: (persistedState, currentState) =>
        mergePersistedState(currentState, persistedState),
    }
  )
);
