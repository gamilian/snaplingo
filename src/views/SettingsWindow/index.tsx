import { useEffect } from 'react';

import { readSettingsNavigationLaunch } from '../../application/settings/navigation';
import type { SettingsRuntime } from '../../application/settings/runtime';
import { useSettingsStore } from '../../stores/settingsStore';
import { MainNav } from './Navigation/MainNav';
import { SecondaryNav } from './Navigation/SecondaryNav';
import {
  findSettingsSection,
  type SettingsSection,
} from './navigationModel';
import { createSettingsNavigationState } from './settingsNavigationState';
import { SettingsRuntimeProvider, useSettingsRuntime } from './runtimeContext';

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
  const navigate = useSettingsStore((state) => state.navigate);
  const activeSection = findSettingsSection(activeMainTab);

  const runtime = useSettingsRuntime();
  useEffect(() => {
    const initialRequest = readSettingsNavigationLaunch(window.location.search);
    if (initialRequest) navigate(initialRequest);

    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    runtime.window
      .subscribeNavigationRequested(navigate)
      .then((dispose) => {
        if (disposed) dispose();
        else unsubscribe = dispose;
      })
      .catch((error) => {
        console.warn('Failed to subscribe to settings navigation:', error);
      });

    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, [navigate, runtime]);

  return (
    <div className="flex h-screen bg-[#f5f5f7]">
      <MainNav
        activeTab={activeMainTab}
        onTabChange={setActiveMainTab}
        library={runtime.library}
      />

      <div className="flex-1 overflow-hidden flex">
        <SettingsSectionContent section={activeSection} />
      </div>
    </div>
  );
}

function SettingsSectionContent({ section }: { section: SettingsSection }) {
  const servicesSubTab = useSettingsStore((state) => state.servicesSubTab);
  const setServicesSubTab = useSettingsStore((state) => state.setServicesSubTab);

  if (!('secondary' in section)) {
    if (section.key === 'favorites' || section.key === 'history') {
      return section.render();
    }
    return section.render();
  }

  const navigationState = createSettingsNavigationState(
    section,
    { services: servicesSubTab },
    { services: setServicesSubTab },
  );

  if (section.key === 'services') {
    return (
      <div className="h-full flex-1 overflow-y-auto bg-[#f4f5f7]">
        <main className="mx-auto w-full max-w-[1080px] px-8 pb-16">
          <header className="sticky top-0 z-20 -mx-[22px] mb-[18px] flex items-center justify-between gap-7 border-b border-gray-200/95 bg-[#f4f5f7]/95 px-[22px] pb-[15px] pt-[22px] backdrop-blur-xl">
            <div className="min-w-0">
              <div className="flex items-center gap-2.5">
                <h1 className="text-[26px] font-bold tracking-[-0.045em] text-gray-950">
                  服务
                </h1>
                <span className="flex items-center gap-1.5 text-[10px] font-medium text-gray-500">
                  <i className="h-1.5 w-1.5 rounded-full bg-green-500 ring-[3px] ring-green-100" />
                  自动保存
                </span>
              </div>
              <p className="mt-1 text-[12px] text-gray-500">
                配置 OCR、翻译与语音合成引擎
              </p>
            </div>

            <SecondaryNav
              items={section.secondary}
              activeItem={navigationState.activeKey}
              onItemChange={navigationState.setActiveKey}
              orientation="horizontal"
            />
          </header>

          <div className="relative">{navigationState.activeItem?.render()}</div>
        </main>
      </div>
    );
  }
}
