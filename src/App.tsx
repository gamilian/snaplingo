import { useCallback, useEffect, useState } from 'react';
import { SettingsWindow } from './views/SettingsWindow';
import ResultWindow from './views/ResultWindow';
import { runOcrFileWorkflow } from './views/ResultWindow/ocrFileWorkflow';
import {
  ocrPayloadDisplayText,
  shouldApplyOcrPayloadText,
  shouldClearOcrResultsForPayload,
  shouldStartFileOcrForPayload,
  shouldApplyTranslationPayloadText,
  shouldClearTranslationResultsForPayload,
  translationPayloadSourceText,
} from './views/ResultWindow/resultPayload';
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
import { initializeProviderStore } from './stores/providerStore';
import { initializeHistoryStore } from './stores/historyStore';
import {
  isCaptureResultWindowLaunch,
  isSettingsWindowLaunch,
} from './appWindowRouting';
import { captureWorkspaceEvents, resultWindowEvents } from './platform/tauri/appEvents';
import {
  captureWorkspaceCommands,
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
const resultWindowRuntime = createResultWindowPlatformRuntime({
  events: resultWindowEvents,
  window: resultWindow,
  clipboard: { writeText: writeClipboardText },
  commands: {
    takePayload: takeCaptureResultWindowPayload,
    selectImageFile,
    recognizeImageFile,
    recognizeImageData,
    translateTextWithProvider,
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
  const setSourceText = useAppStore((state) => state.setSourceText);
  const clearTranslationResults = useAppStore((state) => state.clearTranslationResults);
  const setOcrText = useAppStore((state) => state.setOcrText);
  const setOcrImageBase64 = useAppStore((state) => state.setOcrImageBase64);
  const setOcrRunning = useAppStore((state) => state.setOcrRunning);
  const setOcrError = useAppStore((state) => state.setOcrError);
  const requestAutoTranslate = useAppStore((state) => state.requestAutoTranslate);
  const showResultWindow = useAppStore((state) => state.showResultWindow);
  const showOcrWindow = useAppStore((state) => state.showOcrWindow);
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

  const startFileOcr = useCallback(() => {
    showOcrWindow();
    setOcrText('');
    setOcrImageBase64(null);
    setOcrError(null);
    void runOcrFileWorkflow({
      selectImageFile: resultWindowRuntime.commands.selectImageFile,
      recognizeImageFile: resultWindowRuntime.commands.recognizeImageFile,
      setText: setOcrText,
      setRunning: setOcrRunning,
      setError: setOcrError,
    });
  }, [
    setOcrError,
    setOcrImageBase64,
    setOcrRunning,
    setOcrText,
    showOcrWindow,
  ]);

  useEffect(() => {
    if (!isSettingsWindow) return;

    hydrateHotkeys().catch((err) => {
      console.warn('Failed to hydrate hotkey configuration:', err);
    });
  }, [hydrateHotkeys, isSettingsWindow]);

  const loadCaptureResultPayload = useCallback(async () => {
    const payload = await resultWindowRuntime.commands.takePayload();
    if (!payload) return;

    setHasLoadedCaptureResultPayload(true);

    if (payload.mode === 'translation') {
      if (shouldClearTranslationResultsForPayload(payload)) {
        clearTranslationResults();
      }
      if (shouldApplyTranslationPayloadText(payload)) {
        setSourceText(translationPayloadSourceText(payload));
      }
      if (payload.autoTranslate) {
        requestAutoTranslate();
      }
      showResultWindow();
      return;
    }

    if (shouldClearOcrResultsForPayload(payload)) {
      setOcrText('');
      setOcrImageBase64(null);
      setOcrError(null);
    }
    if (shouldApplyOcrPayloadText(payload)) {
      setOcrText(ocrPayloadDisplayText(payload));
      setOcrImageBase64(payload.imageBase64 ?? null);
    }
    if (shouldStartFileOcrForPayload(payload)) {
      startFileOcr();
      return;
    }
    showOcrWindow();
  }, [
    requestAutoTranslate,
    clearTranslationResults,
    setOcrError,
    setOcrImageBase64,
    setOcrText,
    setSourceText,
    showOcrWindow,
    showResultWindow,
    startFileOcr,
  ]);

  useEffect(() => {
    if (!isCaptureResultWindow) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    loadCaptureResultPayload().catch((err) => {
      console.error('Failed to load capture result window payload:', err);
    });

    resultWindowRuntime.onPayloadReady(() => {
      if (disposed) return;
      void loadCaptureResultPayload().catch((err) => {
        console.error('Failed to reload capture result window payload:', err);
      });
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
  }, [isCaptureResultWindow, loadCaptureResultPayload]);

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
