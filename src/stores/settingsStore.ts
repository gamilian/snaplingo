import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type {
  MainTab,
  ServicesSubTab,
} from '../views/SettingsWindow/navigationModel';
import type { SettingsNavigationRequest } from '../application/settings/navigation';

interface SettingsState {
  activeMainTab: MainTab;
  servicesSubTab: ServicesSubTab;
  requestedSection: string | null;

  setActiveMainTab: (tab: MainTab) => void;
  setServicesSubTab: (tab: SettingsState['servicesSubTab']) => void;
  navigate: (request: SettingsNavigationRequest) => void;
  consumeRequestedSection: () => void;
}

interface PersistedSettingsState {
  activeMainTab?: MainTab;
  servicesSubTab?: ServicesSubTab;
}

function mergePersistedState(state: SettingsState, persistedState: unknown): SettingsState {
  const persisted = (persistedState ?? {}) as PersistedSettingsState;
  const activeMainTab = isMainTab(persisted.activeMainTab)
    ? persisted.activeMainTab
    : state.activeMainTab;

  return {
    ...state,
    activeMainTab,
    servicesSubTab: persisted.servicesSubTab ?? state.servicesSubTab,
    requestedSection: null,
  };
}

function isMainTab(value: unknown): value is MainTab {
  return [
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
      activeMainTab: 'screenshot',
      servicesSubTab: 'ocr',
      requestedSection: null,
      setActiveMainTab: (tab) =>
        set({ activeMainTab: tab, requestedSection: null }),
      setServicesSubTab: (tab) => set({ servicesSubTab: tab }),
      navigate: ({ tab, section }) =>
        set({ activeMainTab: tab, requestedSection: section ?? null }),
      consumeRequestedSection: () => set({ requestedSection: null }),
    }),
    {
      name: 'snaplingo-settings',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        activeMainTab: state.activeMainTab,
        servicesSubTab: state.servicesSubTab,
      }),
      merge: (persistedState, currentState) =>
        mergePersistedState(currentState, persistedState),
    }
  )
);
