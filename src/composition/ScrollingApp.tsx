import { useEffect } from 'react';
import { createDurableSettingsConfiguration } from '../application/settings/configuration';
import { createScrollingCaptureRuntime } from '../application/scrolling-capture/runtime';
import { scrollingCapturePorts } from '../platform/tauri/scrollingCapture';
import { persistentStateEvents } from '../platform/tauri/appEvents';
import * as durableSettings from '../platform/tauri/settings';
import {
  initializeSettingsConfigStore,
  useSettingsConfigStore,
} from '../stores/settingsConfigStore';
import {
  ScrollingCaptureWindow,
  ScrollingCapturePreviewWindow,
} from '../views/ScrollingCaptureWindow';
import { useApplicationTheme } from '../views/useApplicationTheme';

const settings = createDurableSettingsConfiguration(durableSettings);
initializeSettingsConfigStore(settings);
const parameters = new URLSearchParams(window.location.search);
const captureId = parameters.get('captureId');
const preview = parameters.get('window') === 'scrolling-preview';
const runtime = captureId
  ? createScrollingCaptureRuntime(captureId, scrollingCapturePorts, { sampling: !preview })
  : null;

export default function ScrollingApp() {
  const general = useSettingsConfigStore((state) => state.general);
  useEffect(() => settings.synchronize(persistentStateEvents), []);
  useApplicationTheme(general, 'application');
  if (!runtime) return null;
  return preview
    ? <ScrollingCapturePreviewWindow runtime={runtime} />
    : <ScrollingCaptureWindow runtime={runtime} />;
}
