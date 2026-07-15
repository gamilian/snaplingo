import {
  findSecondaryNavItem,
  isServicesSubTab,
  type SecondaryNavItem,
  type ServicesSubTab,
  type SettingsSection,
} from './navigationModel';

export interface SettingsSecondaryKeys {
  services: string;
}

export interface SettingsSecondarySetters {
  services: (key: ServicesSubTab) => void;
}

export interface SettingsNavigationState {
  activeKey: string;
  activeItem: SecondaryNavItem | null;
  setActiveKey: (key: string) => void;
}

export function createSettingsNavigationState(
  section: SettingsSection,
  activeKeys: SettingsSecondaryKeys,
  setTab: SettingsSecondarySetters,
): SettingsNavigationState {
  if (!('secondary' in section)) {
    return {
      activeKey: '',
      activeItem: null,
      setActiveKey: () => {},
    };
  }

  const activeKey = findSecondaryNavItem(section, activeKeys.services)?.key ?? '';

  return {
    activeKey,
    activeItem: findSecondaryNavItem(section, activeKey),
    setActiveKey: (key) => {
      if (isServicesSubTab(key)) {
        setTab.services(key);
      }
    },
  };
}
