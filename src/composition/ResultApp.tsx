import { useEffect, useState } from 'react';
import { createSettingsConfiguration } from '../application/settings/configuration';
import { persistentStateEvents } from '../platform/tauri/appEvents';
import * as durableSettings from '../platform/tauri/settings';
import * as hotkeys from '../platform/tauri/hotkeys';
import { settingsProviders } from '../platform/tauri/providers';
import {
  initializeSettingsConfigStore,
  useSettingsConfigStore,
} from '../stores/settingsConfigStore';
import { initializeProviderStore } from '../stores/providerStore';
import {
  initializeResultWindowStore,
  useResultWindowStore,
} from '../stores/resultWindowStore';
import { useApplicationTheme } from '../views/useApplicationTheme';
import ResultWindow from '../views/ResultWindow';
import { createWindowResultRuntime } from './resultRuntime';

const configuration = createSettingsConfiguration({
  durableSettings,
  hotkeys,
  providers: settingsProviders,
  events: persistentStateEvents,
});
const runtime = createWindowResultRuntime(configuration);
initializeSettingsConfigStore(configuration.settings);
initializeProviderStore(configuration.providers);
initializeResultWindowStore(runtime);

export default function ResultApp() {
  const general = useSettingsConfigStore((state) => state.general);
  const visible = useResultWindowStore((state) => state.resultWindowVisible);
  const [loaded, setLoaded] = useState(false);
  useApplicationTheme(general, 'application');

  useEffect(() => {
    let disposed = false;
    void configuration.settings.hydrate()
      .then((snapshot) => {
        if (!disposed) runtime.applyTranslationDefaults(snapshot.translation);
      })
      .catch((error) => console.warn('Failed to hydrate result settings:', error));
    const stop = configuration.synchronize({
      settingsWindow: false,
      onSettingsChanged: (snapshot) =>
        runtime.applyTranslationDefaults(snapshot.translation),
      invalidateHistory: () => {},
      invalidateFavorites: () => {},
      invalidateScreenshotFavorites: () => {},
    });
    return () => {
      disposed = true;
      stop();
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void runtime.subscribeToPayloads(() => {
      if (!disposed) setLoaded(true);
    })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlisten = cleanup;
      })
      .catch((error) => console.error('Failed to subscribe to result payloads:', error));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (loaded && !visible) void runtime.dismiss();
  }, [loaded, visible]);
  return <ResultWindow presentation="standalone" runtime={runtime} />;
}
