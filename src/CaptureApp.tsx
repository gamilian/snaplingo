import { useEffect } from 'react';
import { createDurableSettingsConfiguration } from './application/settings/configuration';
import type { CaptureWorkspacePorts } from './application/capture-workspace/ports';
import { captureWorkspaceCommands } from './platform/tauri/capture';
import {
  captureWorkspaceEvents,
  persistentStateEvents,
} from './platform/tauri/appEvents';
import { captureWindow } from './platform/tauri/captureWindow';
import { writeClipboardText } from './platform/tauri/clipboard';
import * as durableSettings from './platform/tauri/settings';
import {
  initializeSettingsConfigStore,
  useSettingsConfigStore,
} from './stores/settingsConfigStore';
import CaptureWorkspace from './views/CaptureWorkspace';
import { printBase64PngImage } from './views/CaptureWorkspace/capturePrint';
import { readCaptureLaunch } from './views/CaptureWorkspace/windowMode';

// Capture windows start independently of settings, library, and result-window runtimes.
const settings = createDurableSettingsConfiguration(durableSettings);
initializeSettingsConfigStore(settings);
const launch = readCaptureLaunch(window.location.search);
const ports: CaptureWorkspacePorts = {
  events: captureWorkspaceEvents,
  window: captureWindow,
  commands: captureWorkspaceCommands,
  clipboard: { writeText: writeClipboardText },
  print: { printImage: printBase64PngImage },
};

export default function CaptureApp() {
  const general = useSettingsConfigStore((state) => state.general);
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void settings.hydrate().catch((error) => {
      console.warn('Failed to load capture settings:', error);
    });
    void persistentStateEvents
      .subscribeSettingsChanged(async () => {
        if (!disposed) await settings.refresh();
      })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlisten = cleanup;
      })
      .catch((error) => {
        console.warn('Failed to subscribe to capture settings:', error);
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const root = document.documentElement;
    root.dataset.themeScope = 'application';
    root.dataset.theme = 'light';
    root.style.colorScheme = 'light';
    if (general) {
      root.lang = general.language;
      root.dataset.experimentalGpu = String(
        general.experimentalGpuAcceleration ?? false,
      );
    }
  }, [general]);

  return (
    <CaptureWorkspace
      ports={ports}
      initialMode={launch?.mode}
      initialSessionId={launch?.sessionId}
    />
  );
}
