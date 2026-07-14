import type { ReactNode } from 'react';

import { useSettingsStore } from '../../stores/settingsStore';
import { MainNav } from './Navigation/MainNav';
import { SecondaryNav } from './Navigation/SecondaryNav';
import {
  findSettingsSection,
  type SettingsSection,
} from './navigationModel';
import { createSettingsNavigationState } from './settingsNavigationState';
import type { SettingsRuntime } from '../../application/settings/runtime';
import { SettingsRuntimeProvider } from './runtimeContext';

export function SettingsWindow({ runtime }: { runtime: SettingsRuntime }) {
  return (
    <SettingsRuntimeProvider runtime={runtime}>
      <SettingsWindowContent />
    </SettingsRuntimeProvider>
  );
}

function SettingsWindowContent() {
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
    if (section.key === 'favorites' || section.key === 'history') {
      return section.render();
    }
    return <ContentFrame>{section.render()}</ContentFrame>;
  }

  const navigationState = createSettingsNavigationState(
    section,
    {
      screenshot: screenshotSubTab,
      translation: translationSubTab,
      ocr: ocrSubTab,
      services: servicesSubTab,
    },
    {
      screenshot: setScreenshotSubTab,
      translation: setTranslationSubTab,
      ocr: setOcrSubTab,
      services: setServicesSubTab,
    },
  );

  if (section.key === 'services') {
    return (
      <ContentFrame>
        <div className="relative space-y-8">
          <SecondaryNav
            items={section.secondary}
            activeItem={navigationState.activeKey}
            onItemChange={navigationState.setActiveKey}
            orientation="horizontal"
          />
          {navigationState.activeItem?.render()}
        </div>
      </ContentFrame>
    );
  }

  return (
    <>
      <SecondaryNav
        items={section.secondary}
        activeItem={navigationState.activeKey}
        onItemChange={navigationState.setActiveKey}
      />
      <ContentFrame>{navigationState.activeItem?.render()}</ContentFrame>
    </>
  );
}

function ContentFrame({ children }: { children: ReactNode }) {
  return <div className="flex-1 overflow-y-auto p-12">{children}</div>;
}
