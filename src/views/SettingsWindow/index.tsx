import { useEffect, type ReactNode } from 'react';

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
}

function ContentFrame({ children }: { children: ReactNode }) {
  return <div className="flex-1 overflow-y-auto p-12">{children}</div>;
}
