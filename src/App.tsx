import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { SettingsWindow } from './components/SettingsWindow';
import ResultWindow from './components/ResultWindow';
import { runOcrFileWorkflow } from './components/ResultWindow/ocrFileWorkflow';
import { parseInputTranslationPayload } from './components/ResultWindow/translationInput';
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
import { recognizeImageFile, selectImageFile } from './tauri/ocr';

const currentWindow = getCurrentWebviewWindow();
const captureLaunch = readCaptureLaunch(window.location.search);
const pinnedImageId = readPinnedImageLaunch(window.location.search);

function App() {
  const resultWindowVisible = useAppStore((state) => state.resultWindowVisible);
  const setSourceText = useAppStore((state) => state.setSourceText);
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
  const isPinnedImageWindow = pinnedImageId !== null;

  useEffect(() => {
    if (isCaptureWindow || isPinnedImageWindow) return;

    const unlistenCapturedPromise = listen<string>('screenshot-captured', (event) => {
      setCapturedScreenshot(`data:image/png;base64,${event.payload}`);
      setActiveMainTab('screenshot');
      setScreenshotSubTab('editor');
    });
    const unlistenErrorPromise = listen<string>('screenshot-error', (event) => {
      alert(event.payload);
    });

    return () => {
      unlistenCapturedPromise.then((unlisten) => unlisten());
      unlistenErrorPromise.then((unlisten) => unlisten());
    };
  }, [
    isCaptureWindow,
    isPinnedImageWindow,
    setActiveMainTab,
    setCapturedScreenshot,
    setScreenshotSubTab,
  ]);

  useEffect(() => {
    if (isCaptureWindow || isPinnedImageWindow) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    listen<unknown>('input-translation', (event) => {
      const input = parseInputTranslationPayload(event.payload);
      if (!input) return;

      setSourceText(input.text);
      if (input.autoTranslate) {
        requestAutoTranslate();
      }
      showResultWindow();
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlisten = nextUnlisten;
        }
      })
      .catch((err) => {
        console.error('Failed to listen for translation input:', err);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [
    isCaptureWindow,
    isPinnedImageWindow,
    requestAutoTranslate,
    setSourceText,
    showResultWindow,
  ]);

  useEffect(() => {
    if (isCaptureWindow || isPinnedImageWindow) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    listen<string>('input-ocr', (event) => {
      setOcrText(event.payload);
      setOcrError(null);
      showOcrWindow();
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlisten = nextUnlisten;
        }
      })
      .catch((err) => {
        console.error('Failed to listen for OCR input:', err);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [
    isCaptureWindow,
    isPinnedImageWindow,
    setOcrError,
    setOcrText,
    showOcrWindow,
  ]);

  useEffect(() => {
    if (isCaptureWindow || isPinnedImageWindow) return;

    let disposed = false;
    let unlistenShow: (() => void) | undefined;
    let unlistenFile: (() => void) | undefined;

    const startFileOcr = () => {
      showOcrWindow();
      void runOcrFileWorkflow({
        selectImageFile,
        recognizeImageFile,
        setText: setOcrText,
        setRunning: setOcrRunning,
        setError: setOcrError,
      });
    };

    listen('show-ocr-window', () => {
      showOcrWindow();
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlistenShow = nextUnlisten;
        }
      })
      .catch((err) => {
        console.error('Failed to listen for OCR window show events:', err);
      });

    listen('start-file-ocr', startFileOcr)
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlistenFile = nextUnlisten;
        }
      })
      .catch((err) => {
        console.error('Failed to listen for file OCR events:', err);
      });

    return () => {
      disposed = true;
      unlistenShow?.();
      unlistenFile?.();
    };
  }, [
    isCaptureWindow,
    isPinnedImageWindow,
    setOcrError,
    setOcrRunning,
    setOcrText,
    showOcrWindow,
  ]);

  useEffect(() => {
    if (isCaptureWindow || isPinnedImageWindow) return;

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
  }, [hotkeys, isCaptureWindow, isPinnedImageWindow, setHotkey]);

  useEffect(() => {
    if (isCaptureWindow || isPinnedImageWindow) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    listen('show-translation-window', () => {
      showResultWindow();
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlisten = nextUnlisten;
        }
      })
      .catch((err) => {
        console.error('Failed to listen for translation window show events:', err);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [isCaptureWindow, isPinnedImageWindow, showResultWindow]);

  if (isCaptureWindow) {
    return (
      <ScreenshotSession
        initialMode={captureLaunch?.mode ?? 'screenshot'}
        initialSessionId={captureLaunch?.sessionId}
        onInactive={() => hideInactiveCaptureWindow(currentWindow)}
      />
    );
  }

  if (pinnedImageId) {
    return <PinnedImageWindow imageId={pinnedImageId} />;
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
