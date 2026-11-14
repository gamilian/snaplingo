import { useEffect } from 'react';
import { createRequiredPermissionsRuntime } from '../application/permissions/runtime';
import { requiredPermissions } from '../platform/tauri/permissions';
import { RequiredPermissionsGate } from '../views/RequiredPermissionsGate';
import { createAppUpdatesRuntime } from '../application/updates/runtime';
import { appUpdates } from '../platform/tauri/updates';
import { SettingsWindow } from '../views/SettingsWindow';
import ResultWindow from '../views/ResultWindow';
import { createSettingsRuntime } from '../application/settings/runtime';
import { useApplicationTheme } from '../views/useApplicationTheme';
import { initializeResultWindowStore, useResultWindowStore } from '../stores/resultWindowStore';
import { initializeHotkeyConfigStore } from '../stores/hotkeyConfigStore';
import { initializeSettingsConfigStore, useSettingsConfigStore } from '../stores/settingsConfigStore';
import { useSettingsStore } from '../stores/settingsStore';
import { initializeProviderStore } from '../stores/providerStore';
import { useHistoryStore } from '../stores/historyStore';
import { useScreenshotFavoritesStore } from '../stores/screenshotFavoritesStore';
import { initializeFavoritesStore, useFavoritesStore } from '../stores/favoritesStore';
import { persistentStateEvents, settingsWindowEvents } from '../platform/tauri/appEvents';
import { writeClipboardText } from '../platform/tauri/clipboard';
import * as history from '../platform/tauri/history';
import { libraryIndex } from '../platform/tauri/libraryIndex';
import { favorites } from '../platform/tauri/favorites';
import { screenshotFavorites } from '../platform/tauri/screenshotFavorites';
import * as hotkeys from '../platform/tauri/hotkeys';
import { settingsProviders } from '../platform/tauri/providers';
import * as durableSettings from '../platform/tauri/settings';
import { systemTts } from '../platform/tauri/tts';
import { settingsWindow } from '../platform/tauri/settingsWindow';
import { createWindowResultRuntime } from './resultRuntime';

const requiredPermissionsRuntime =
  createRequiredPermissionsRuntime(requiredPermissions);
const settingsRuntime = createSettingsRuntime({
  permissions: requiredPermissionsRuntime,
  updates: createAppUpdatesRuntime(appUpdates),
  window: settingsWindow,
  windowEvents: settingsWindowEvents,
  configurationEvents: persistentStateEvents,
  durableSettings,
  maintenance: {
    listAppLogs: durableSettings.listAppLogs,
    clearAppLogs: durableSettings.clearAppLogs,
  },
  tts: systemTts,
  providers: settingsProviders,
  hotkeys,
  history,
  libraryIndex,
  favorites,
  screenshotFavorites,
  clipboard: { writeText: writeClipboardText },
});
const resultWindowRuntime = createWindowResultRuntime(settingsRuntime.configuration);
initializeResultWindowStore(resultWindowRuntime);
initializeSettingsConfigStore(settingsRuntime.configuration.settings);
initializeHotkeyConfigStore(settingsRuntime.configuration.hotkeys);
initializeProviderStore(settingsRuntime.configuration.providers);
initializeFavoritesStore(settingsRuntime.favorites);

function Application() {
  const resultWindowVisible = useResultWindowStore(
    (state) => state.resultWindowVisible,
  );
  const { applyTranslationDefaults } = resultWindowRuntime;
  const generalSettings = useSettingsConfigStore((state) => state.general);

  useEffect(() => {
    let disposed = false;

    settingsRuntime.configuration.settings
      .hydrate()
      .then((snapshot) => {
        if (!disposed) {
          applyTranslationDefaults(snapshot.translation);
        }
      })
      .catch((err) => {
        console.warn('Failed to hydrate durable settings:', err);
      });

    return () => {
      disposed = true;
    };
  }, [applyTranslationDefaults]);

  useApplicationTheme(generalSettings, 'settings');

  useEffect(() => {
    settingsRuntime.configuration.hotkeys.hydrate().catch((err) => {
      console.warn('Failed to hydrate hotkey configuration:', err);
    });
  }, []);

  useEffect(() => {
    return settingsRuntime.configuration.synchronize({
      settingsWindow: true,
      onSettingsChanged: (snapshot) =>
        applyTranslationDefaults(snapshot.translation),
      invalidateHistory: () => useHistoryStore.getState().invalidate(),
      invalidateFavorites: () => useFavoritesStore.getState().invalidate(),
      invalidateScreenshotFavorites: () =>
        useScreenshotFavoritesStore.getState().invalidate(),
    });
  }, [applyTranslationDefaults]);

  return (
    <>
      {/* 主设置窗口 */}
      <RequiredPermissionsGate runtime={requiredPermissionsRuntime}
        onOpenSettings={() => useSettingsStore.getState().navigate({ tab: 'general', section: 'permissions' })}>
        <SettingsWindow runtime={settingsRuntime} />
      </RequiredPermissionsGate>

      {/* 翻译结果窗口（浮动） */}
      {resultWindowVisible && <ResultWindow runtime={resultWindowRuntime} />}
    </>
  );
}

export default Application;
