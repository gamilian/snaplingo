import { useEffect } from 'react';
import { createDurableSettingsConfiguration } from '../application/settings/configuration';
import { createPinnedImageRuntime } from '../application/pinned-image/runtime';
import { persistentStateEvents } from '../platform/tauri/appEvents';
import * as durableSettings from '../platform/tauri/settings';
import { pinnedWindow } from '../platform/tauri/pinnedWindow';
import { pinnedImageCommands } from '../platform/tauri/pinnedImage';
import { writeClipboardText } from '../platform/tauri/clipboard';
import { settingsWindow } from '../platform/tauri/settingsWindow';
import {
  initializeSettingsConfigStore,
  useSettingsConfigStore,
} from '../stores/settingsConfigStore';
import { PinnedImageWindow, readPinnedImageLaunch } from '../views/PinnedImageWindow';
import { useApplicationTheme } from '../views/useApplicationTheme';

const settings = createDurableSettingsConfiguration(durableSettings);
initializeSettingsConfigStore(settings);
const imageId = readPinnedImageLaunch(window.location.search);
const runtime = imageId
  ? createPinnedImageRuntime({
    imageId,
    window: pinnedWindow,
    commands: pinnedImageCommands,
    clipboard: { writeText: writeClipboardText },
    settings: { open: settingsWindow.openSettings },
  })
  : null;

export default function PinnedApp() {
  const general = useSettingsConfigStore((state) => state.general);
  useEffect(() => settings.synchronize(persistentStateEvents), []);
  useApplicationTheme(general, 'application');
  return runtime ? <PinnedImageWindow runtime={runtime} /> : null;
}
