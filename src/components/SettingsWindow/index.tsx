import type { ReactNode } from 'react';

import { useSettingsStore } from '../../stores/settingsStore';
import { MainNav } from './Navigation/MainNav';
import { SecondaryNav } from './Navigation/SecondaryNav';
import {
  findSecondaryNavItem,
  findSettingsSection,
  isOcrSubTab,
  isScreenshotSubTab,
  isServicesSubTab,
  isTranslationSubTab,
  type MainTab,
  type SettingsSection,
} from './navigationModel';

export function SettingsWindow() {
  const activeMainTab = useSettingsStore((state) => state.activeMainTab);
  const setActiveMainTab = useSettingsStore((state) => state.setActiveMainTab);
  const activeSection = findSettingsSection(activeMainTab);

  return (
    <div className="flex h-screen bg-[#f5f5f7]">
      <MainNav activeTab={activeMainTab} onTabChange={setActiveMainTab} />

      <div className="flex-1 overflow-hidden flex">
        <SettingsSectionContent section={activeSection} />
      </div>
    </div>
  );
}

function SettingsSectionContent({ section }: { section: SettingsSection }) {
  const screenshotSubTab = useSettingsStore((state) => state.screenshotSubTab);
  const translationSubTab = useSettingsStore((state) => state.translationSubTab);
  const ocrSubTab = useSettingsStore((state) => state.ocrSubTab);
  const servicesSubTab = useSettingsStore((state) => state.servicesSubTab);
  const setScreenshotSubTab = useSettingsStore((state) => state.setScreenshotSubTab);
  const setTranslationSubTab = useSettingsStore((state) => state.setTranslationSubTab);
  const setOcrSubTab = useSettingsStore((state) => state.setOcrSubTab);
  const setServicesSubTab = useSettingsStore((state) => state.setServicesSubTab);

  if (!('secondary' in section)) {
    return <ContentFrame>{section.render()}</ContentFrame>;
  }

  const activeKey = getActiveSecondaryKey(section.key, {
    screenshot: screenshotSubTab,
    translation: translationSubTab,
    ocr: ocrSubTab,
    services: servicesSubTab,
  });
  const activeItem = findSecondaryNavItem(section, activeKey);

  return (
    <>
      <SecondaryNav
        items={section.secondary}
        activeItem={activeItem?.key ?? ''}
        onItemChange={(key) =>
          setSecondaryTab(section.key, key, {
            screenshot: setScreenshotSubTab,
            translation: setTranslationSubTab,
            ocr: setOcrSubTab,
            services: setServicesSubTab,
          })
        }
      />
      <ContentFrame>{activeItem?.render()}</ContentFrame>
    </>
  );
}

function ContentFrame({ children }: { children: ReactNode }) {
  return <div className="flex-1 overflow-y-auto p-12">{children}</div>;
}

function getActiveSecondaryKey(
  section: MainTab,
  activeKeys: {
    screenshot: string;
    translation: string;
    ocr: string;
    services: string;
  },
): string {
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
  setTab: {
    screenshot: ReturnType<typeof useSettingsStore.getState>['setScreenshotSubTab'];
    translation: ReturnType<typeof useSettingsStore.getState>['setTranslationSubTab'];
    ocr: ReturnType<typeof useSettingsStore.getState>['setOcrSubTab'];
    services: ReturnType<typeof useSettingsStore.getState>['setServicesSubTab'];
  },
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
