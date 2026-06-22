import {
  findSecondaryNavItem,
  isOcrSubTab,
  isScreenshotSubTab,
  isServicesSubTab,
  isTranslationSubTab,
  type MainTab,
  type OcrSubTab,
  type ScreenshotSubTab,
  type SecondaryNavItem,
  type ServicesSubTab,
  type SettingsSection,
  type TranslationSubTab,
} from './navigationModel';

export interface SettingsSecondaryKeys {
  screenshot: string;
  translation: string;
  ocr: string;
  services: string;
}

export interface SettingsSecondarySetters {
  screenshot: (key: ScreenshotSubTab) => void;
  translation: (key: TranslationSubTab) => void;
  ocr: (key: OcrSubTab) => void;
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

  const activeKey = resolveActiveSecondaryKey(section, activeKeys);

  return {
    activeKey,
    activeItem: findSecondaryNavItem(section, activeKey),
    setActiveKey: (key) => setSecondaryTab(section.key, key, setTab),
  };
}

function resolveActiveSecondaryKey(
  section: SettingsSection,
  activeKeys: SettingsSecondaryKeys,
): string {
  if (!('secondary' in section)) {
    return '';
  }

  return findSecondaryNavItem(section, getActiveSecondaryKey(section.key, activeKeys))?.key ?? '';
}

function getActiveSecondaryKey(section: MainTab, activeKeys: SettingsSecondaryKeys): string {
  switch (section) {
    case 'screenshot':
      return activeKeys.screenshot;
    case 'translation':
      return activeKeys.translation;
    case 'ocr':
      return activeKeys.ocr;
    case 'services':
      return activeKeys.services;
    case 'general':
    case 'advanced':
      return '';
  }
}

function setSecondaryTab(
  section: MainTab,
  key: string,
  setTab: SettingsSecondarySetters,
) {
  switch (section) {
    case 'screenshot':
      if (isScreenshotSubTab(key)) {
        setTab.screenshot(key);
      }
      return;
    case 'translation':
      if (isTranslationSubTab(key)) {
        setTab.translation(key);
      }
      return;
    case 'ocr':
      if (isOcrSubTab(key)) {
        setTab.ocr(key);
      }
      return;
    case 'services':
      if (isServicesSubTab(key)) {
        setTab.services(key);
      }
      return;
    case 'general':
    case 'advanced':
      return;
  }
}
