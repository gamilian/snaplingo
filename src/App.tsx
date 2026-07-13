import { useEffect, useState } from 'react';
import { SettingsWindow } from './views/SettingsWindow';
import ResultWindow from './views/ResultWindow';
import {
  PinnedImageWindow,
  readPinnedImageLaunch,
} from './views/PinnedImageWindow';
import CaptureWorkspace from './views/CaptureWorkspace';
import {
  CAPTURE_WINDOW_LABEL,
  readCaptureLaunch,
} from './views/CaptureWorkspace/windowMode';
import { createCaptureWorkspacePlatformRuntime } from './application/capture-workspace/platformRuntime';
import { createResultWindowPlatformRuntime } from './application/result-window/platformRuntime';
import { createResultWindowRuntime } from './application/result-window/runtime';
import { createPinnedImagePlatformRuntime } from './application/pinned-image/platformRuntime';
import { createSettingsRuntime } from './application/settings/runtime';
import { useAppStore } from './stores/appStore';
import {
  initializeHotkeyConfigStore,
  useHotkeyConfigStore,
} from './stores/hotkeyConfigStore';
import {
  initializeSettingsConfigStore,
  useSettingsConfigStore,
} from './stores/settingsConfigStore';
import {
  initializeProviderStore,
  useProviderStore,
} from './stores/providerStore';
import {
  initializeHistoryStore,
  useHistoryStore,
} from './stores/historyStore';
import {
  isCaptureResultWindowLaunch,
  isSettingsWindowLaunch,
} from './appWindowRouting';
import {
  captureWorkspaceEvents,
  persistentStateEvents,
  resultWindowEvents,
} from './platform/tauri/appEvents';
import {
  captureWorkspaceCommands,
  currentCaptureResultWindowRequestId,
  takeCaptureResultWindowPayload,
  triggerScreenshot,
} from './platform/tauri/capture';
import { captureWindow } from './platform/tauri/captureWindow';
import { writeClipboardText } from './platform/tauri/clipboard';
import * as history from './platform/tauri/history';
import * as hotkeys from './platform/tauri/hotkeys';
import { recognizeImageData, recognizeImageFile, selectImageFile } from './platform/tauri/ocr';
import { pinnedImageCommands } from './platform/tauri/pinnedImage';
import { pinnedWindow } from './platform/tauri/pinnedWindow';
import { settingsProviders } from './platform/tauri/providers';
import {
  getCurrentWindowLabel,
  resultWindow,
} from './platform/tauri/resultWindow';
import * as durableSettings from './platform/tauri/settings';
import { settingsWindow } from './platform/tauri/settingsWindow';
import { translateTextWithProvider } from './platform/tauri/translation';

const settingsRuntime = createSettingsRuntime({
  window: settingsWindow,
  durableSettings,
  providers: settingsProviders,
  hotkeys,
  history,
  clipboard: { writeText: writeClipboardText },
  capture: { triggerScreenshot },
});
const captureWorkspaceRuntime = createCaptureWorkspacePlatformRuntime({
  events: captureWorkspaceEvents,
  window: captureWindow,
  commands: captureWorkspaceCommands,
  clipboard: { writeText: writeClipboardText },
});
const resultWindowPlatformRuntime = createResultWindowPlatformRuntime({
  events: resultWindowEvents,
  window: resultWindow,
  clipboard: { writeText: writeClipboardText },
  commands: {
    currentPayloadRequestId: currentCaptureResultWindowRequestId,
    takePayload: takeCaptureResultWindowPayload,
    selectImageFile,
    recognizeImageFile,
    recognizeImageData,
    translateTextWithProvider,
  },
});
const resultWindowRuntime = createResultWindowRuntime({
  platform: resultWindowPlatformRuntime,
  state: {
    setSourceText: (text) => useAppStore.getState().setSourceText(text),
    clearTranslationResults: () =>
      useAppStore.getState().clearTranslationResults(),
    setOcrText: (text) => useAppStore.getState().setOcrText(text),
    setOcrImageBase64: (imageBase64) =>
      useAppStore.getState().setOcrImageBase64(imageBase64),
    setOcrRunning: (value) => useAppStore.getState().setOcrRunning(value),
    setOcrError: (message) => useAppStore.getState().setOcrError(message),
    requestAutoTranslate: () => useAppStore.getState().requestAutoTranslate(),
    showResultWindow: () => useAppStore.getState().showResultWindow(),
    showOcrWindow: () => useAppStore.getState().showOcrWindow(),
    hideResultWindow: () => useAppStore.getState().hideResultWindow(),
  },
});
const pinnedImageRuntime = createPinnedImagePlatformRuntime({
  window: pinnedWindow,
  commands: pinnedImageCommands,
  clipboard: { writeText: writeClipboardText },
  settings: settingsRuntime.window,
});

initializeSettingsConfigStore(settingsRuntime.durableSettings);
initializeHotkeyConfigStore(settingsRuntime.hotkeys);
initializeProviderStore(settingsRuntime.providers);
initializeHistoryStore(settingsRuntime.history);

const currentWindowLabel = getCurrentWindowLabel();
const captureLaunch = readCaptureLaunch(window.location.search);
const pinnedImageId = readPinnedImageLaunch(window.location.search);
const isCaptureResultWindow = isCaptureResultWindowLaunch(
  currentWindowLabel,
  window.location.search,
);
const isSettingsWindow = isSettingsWindowLaunch(
  currentWindowLabel,
  window.location.search,
);

function App() {
  const resultWindowVisible = useAppStore((state) => state.resultWindowVisible);
  const applyTranslationDefaults = useAppStore(
    (state) => state.applyTranslationDefaults,
  );
  const hydrateSettings = useSettingsConfigStore((state) => state.hydrate);
  const hydrateHotkeys = useHotkeyConfigStore((state) => state.hydrate);
  const isCaptureWindow =
    currentWindowLabel === CAPTURE_WINDOW_LABEL || captureLaunch !== null;
  const [hasLoadedCaptureResultPayload, setHasLoadedCaptureResultPayload] =
    useState(false);

  useEffect(() => {
    let disposed = false;

    hydrateSettings()
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
  }, [applyTranslationDefaults, hydrateSettings]);

  useEffect(() => {
    if (!isSettingsWindow) return;

    hydrateHotkeys().catch((err) => {
      console.warn('Failed to hydrate hotkey configuration:', err);
    });
  }, [hydrateHotkeys, isSettingsWindow]);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];

    const track = (subscription: Promise<() => void>, label: string) => {
      subscription
        .then((unlisten) => {
          if (disposed) {
            unlisten();
          } else {
            unlisteners.push(unlisten);
          }
        })
        .catch((err) => {
          console.warn(`Failed to subscribe to ${label} changes:`, err);
        });
    };

    track(
      persistentStateEvents.subscribeSettingsChanged(async () => {
        try {
          const snapshot = await useSettingsConfigStore.getState().refresh();
          applyTranslationDefaults(snapshot.translation);
        } catch (err) {
          console.warn('Failed to refresh durable settings:', err);
        }
      }),
      'settings',
    );

    track(
      persistentStateEvents.subscribeProvidersChanged(async () => {
        const providers = useProviderStore.getState();
        await Promise.all([
          providers.loadTranslationProviders(),
          providers.loadOcrProviders(),
        ]);
      }),
      'provider',
    );

    if (isSettingsWindow) {
      track(
        persistentStateEvents.subscribeHotkeysChanged(async () => {
          try {
            await useHotkeyConfigStore.getState().refresh();
          } catch (err) {
            console.warn('Failed to refresh hotkey configuration:', err);
          }
        }),
        'hotkey',
      );
      track(
        persistentStateEvents.subscribeHistoryChanged(async () => {
          const historyState = useHistoryStore.getState();
          await Promise.all([
            historyState.loadTranslationHistory(),
            historyState.loadOcrHistory(),
          ]);
        }),
        'history',
      );
    }

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [applyTranslationDefaults, isSettingsWindow]);

  useEffect(() => {
    if (!isCaptureResultWindow) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    resultWindowRuntime
      .loadCurrentPayload()
      .then((loaded) => {
        if (!disposed && loaded) {
          setHasLoadedCaptureResultPayload(true);
        }
      })
      .catch((err) => {
        console.error('Failed to load capture result window payload:', err);
      });

    resultWindowRuntime
      .subscribeToPayloads(() => {
        if (!disposed) {
          setHasLoadedCaptureResultPayload(true);
        }
      })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlisten = nextUnlisten;
        }
      })
      .catch((err) => {
        console.error('Failed to subscribe to capture result payload event:', err);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [isCaptureResultWindow]);

  useEffect(() => {
    if (
      !isCaptureResultWindow ||
      !hasLoadedCaptureResultPayload ||
      resultWindowVisible
    ) {
      return;
    }

    void resultWindowRuntime.dismiss();
  }, [
    hasLoadedCaptureResultPayload,
    isCaptureResultWindow,
    resultWindowVisible,
  ]);

  if (isCaptureWindow) {
    return (
      <CaptureWorkspace
        runtime={captureWorkspaceRuntime}
        initialMode={captureLaunch?.mode}
        initialSessionId={captureLaunch?.sessionId}
        onInactive={() => captureWorkspaceRuntime.dismiss()}
      />
    );
  }

  if (pinnedImageId) {
    return <PinnedImageWindow imageId={pinnedImageId} runtime={pinnedImageRuntime} />;
  }

  if (isCaptureResultWindow) {
    return <ResultWindow presentation="standalone" runtime={resultWindowRuntime} />;
  }

  if (!isSettingsWindow) {
    return null;
  }

  return (
    <>
      {/* 主设置窗口 */}
      <SettingsWindow runtime={settingsRuntime} />

      {/* 翻译结果窗口（浮动） */}
      {resultWindowVisible && <ResultWindow runtime={resultWindowRuntime} />}
    </>
  );
}

export default App;
