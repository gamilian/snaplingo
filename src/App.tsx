import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { SettingsWindow } from './components/SettingsWindow';
import ResultWindow from './components/ResultWindow';
import { runOcrFileWorkflow } from './components/ResultWindow/ocrFileWorkflow';
import {
  shouldApplyOcrPayloadText,
  shouldClearOcrResultsForPayload,
  shouldStartFileOcrForPayload,
  shouldApplyTranslationPayloadText,
  shouldClearTranslationResultsForPayload,
} from './components/ResultWindow/resultPayload';
import {
  PinnedImageWindow,
  readPinnedImageLaunch,
} from './components/PinnedImageWindow';
import ScreenshotSession from './components/ScreenshotSession';
import { hideInactiveCaptureWindow } from './components/ScreenshotSession/captureSessionLifecycle';
import {
  CAPTURE_WINDOW_LABEL,
  readCaptureLaunch,
} from './components/ScreenshotSession/windowMode';
import { useAppStore } from './stores/appStore';
import { useSettingsStore } from './stores/settingsStore';
import {
  configureHotkey,
  type HotkeyCategory,
} from './tauri/hotkeys';
import { subscribeMainWindowEvents } from './tauri/appEvents';
import { recognizeImageFile, selectImageFile } from './tauri/ocr';
import { takeCaptureResultWindowPayload } from './tauri/captureSession';
import {
  isCaptureResultWindowLaunch,
  isSettingsWindowLaunch,
} from './appWindowRouting';

const currentWindow = getCurrentWebviewWindow();
const captureLaunch = readCaptureLaunch(window.location.search);
const pinnedImageId = readPinnedImageLaunch(window.location.search);
const isCaptureResultWindow = isCaptureResultWindowLaunch(
  currentWindow.label,
  window.location.search,
);
const isSettingsWindow = isSettingsWindowLaunch(
  currentWindow.label,
  window.location.search,
);

function App() {
  const resultWindowVisible = useAppStore((state) => state.resultWindowVisible);
  const setSourceText = useAppStore((state) => state.setSourceText);
  const clearTranslationResults = useAppStore((state) => state.clearTranslationResults);
  const setOcrText = useAppStore((state) => state.setOcrText);
  const setOcrRunning = useAppStore((state) => state.setOcrRunning);
  const setOcrError = useAppStore((state) => state.setOcrError);
  const requestAutoTranslate = useAppStore((state) => state.requestAutoTranslate);
  const showResultWindow = useAppStore((state) => state.showResultWindow);
  const showOcrWindow = useAppStore((state) => state.showOcrWindow);
  const setActiveMainTab = useSettingsStore((state) => state.setActiveMainTab);
  const setScreenshotSubTab = useSettingsStore((state) => state.setScreenshotSubTab);
  const setCapturedScreenshot = useSettingsStore((state) => state.setCapturedScreenshot);
  const hotkeys = useSettingsStore((state) => state.hotkeys);
  const setHotkey = useSettingsStore((state) => state.setHotkey);
  const isCaptureWindow =
    currentWindow.label === CAPTURE_WINDOW_LABEL || captureLaunch !== null;
  const [hasLoadedCaptureResultPayload, setHasLoadedCaptureResultPayload] =
    useState(false);

  const startFileOcr = useCallback(() => {
    showOcrWindow();
    setOcrText('');
    setOcrError(null);
    void runOcrFileWorkflow({
      selectImageFile,
      recognizeImageFile,
      setText: setOcrText,
      setRunning: setOcrRunning,
      setError: setOcrError,
    });
  }, [
    setOcrError,
    setOcrRunning,
    setOcrText,
    showOcrWindow,
  ]);

  useEffect(() => {
    if (!isSettingsWindow) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    subscribeMainWindowEvents({
      onScreenshotCaptured: (base64) => {
        setCapturedScreenshot(`data:image/png;base64,${base64}`);
        setActiveMainTab('screenshot');
        setScreenshotSubTab('editor');
      },
      onScreenshotError: (message) => {
        alert(message);
      },
      onInputTranslation: (input) => {
        setSourceText(input.text);
        if (input.autoTranslate) {
          requestAutoTranslate();
        }
        showResultWindow();
      },
      onInputOcr: (text) => {
        setOcrText(text);
        setOcrError(null);
        showOcrWindow();
      },
      onShowOcrWindow: showOcrWindow,
      onStartFileOcr: startFileOcr,
      onShowTranslationWindow: showResultWindow,
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlisten = nextUnlisten;
        }
      })
      .catch((err) => {
        console.error('Failed to subscribe to main window events:', err);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [
    isSettingsWindow,
    requestAutoTranslate,
    setActiveMainTab,
    setCapturedScreenshot,
    setOcrError,
    setOcrText,
    setScreenshotSubTab,
    setSourceText,
    showOcrWindow,
    showResultWindow,
    startFileOcr,
  ]);

  useEffect(() => {
    if (!isSettingsWindow) return;

    (Object.entries(hotkeys) as [HotkeyCategory, Record<string, string>][]).forEach(
      ([category, actionHotkeys]) => {
        Object.entries(actionHotkeys).forEach(([action, hotkey]) => {
          void configureHotkey(category, action, hotkey).catch((err) => {
            console.warn(`Failed to configure hotkey ${category}:${action}:`, err);
            setHotkey(category, action, '未设置');
          });
        });
      },
    );
  }, [hotkeys, isSettingsWindow, setHotkey]);

  const loadCaptureResultPayload = useCallback(async () => {
    const payload = await takeCaptureResultWindowPayload();
    if (!payload) return;

    setHasLoadedCaptureResultPayload(true);

    if (payload.mode === 'translation') {
      if (shouldClearTranslationResultsForPayload(payload)) {
        clearTranslationResults();
      }
      if (shouldApplyTranslationPayloadText(payload)) {
        setSourceText(payload.text);
      }
      if (payload.autoTranslate) {
        requestAutoTranslate();
      }
      showResultWindow();
      return;
    }

    if (shouldClearOcrResultsForPayload(payload)) {
      setOcrText('');
      setOcrError(null);
    }
    if (shouldApplyOcrPayloadText(payload)) {
      setOcrText(payload.text);
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

    listen('capture-result-payload-ready', () => {
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

    void currentWindow.hide();
  }, [
    hasLoadedCaptureResultPayload,
    isCaptureResultWindow,
    resultWindowVisible,
  ]);

  if (isCaptureWindow) {
    return (
      <ScreenshotSession
        initialMode={captureLaunch?.mode}
        initialSessionId={captureLaunch?.sessionId}
        onInactive={() => hideInactiveCaptureWindow(currentWindow)}
      />
    );
  }

  if (pinnedImageId) {
    return <PinnedImageWindow imageId={pinnedImageId} />;
  }

  if (isCaptureResultWindow) {
    return <ResultWindow presentation="standalone" />;
  }

  if (!isSettingsWindow) {
    return null;
  }

  return (
    <>
      {/* 主设置窗口 */}
      <SettingsWindow />

      {/* 翻译结果窗口（浮动） */}
      {resultWindowVisible && <ResultWindow />}
    </>
  );
}

export default App;
