import { useEffect, useState } from 'react';
import { captureInterfaceScale } from '../../application/capture-workspace/interfaceScale';
import type { LogicalRect } from '../../domain/capture';
import { useSettingsConfigStore } from '../../stores/settingsConfigStore';

import { readSettingsNavigationLaunch } from '../../application/settings/navigation';
import type { SettingsRuntime } from '../../application/settings/runtime';
import { useSettingsStore } from '../../stores/settingsStore';
import { MainNav } from './Navigation/MainNav';
import { findSettingsSection } from './navigationModel';
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
  const configuredScale = useSettingsConfigStore((state) => state.screenshot?.captureUiScale);
  const [display, setDisplay] = useState<LogicalRect | null>(null);
  const uiScale = captureInterfaceScale(configuredScale, display);
  useEffect(() => {
    let disposed = false;
    const refreshDisplay = () => { void runtime.window.displayBounds().then((bounds) => { if (!disposed) setDisplay(bounds); }).catch(() => {}); };
    refreshDisplay();
    window.addEventListener('resize', refreshDisplay);
    return () => { disposed = true; window.removeEventListener('resize', refreshDisplay); };
  }, [runtime]);
  useEffect(() => {
    document.documentElement.style.setProperty('--settings-interface-scale', String(uiScale));
    void runtime.window.applyInterfaceScale(uiScale).catch((error) => console.warn('Failed to apply settings interface scale:', error));
  }, [uiScale, display?.width, display?.height, runtime]);
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
    <div className="settings-shell flex bg-canvas" style={{ width: `${100 / uiScale}vw`, height: `${100 / uiScale}vh`, transform: `scale(${uiScale})`, transformOrigin: 'top left', containerType: 'inline-size' }}>
      <MainNav
        activeTab={activeMainTab}
        onTabChange={setActiveMainTab}
        library={runtime.library}
      />

      <div className="settings-content flex-1 min-h-0 min-w-0 overflow-hidden flex">
        {activeSection.render()}
      </div>
    </div>
  );
}
