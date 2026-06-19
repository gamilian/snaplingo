import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { SettingsWindow } from './components/SettingsWindow';
import ResultWindow from './components/ResultWindow';
import { parseInputTranslationPayload } from './components/ResultWindow/translationInput';
import {
  PinnedImageWindow,
  readPinnedImageLaunch,
} from './components/PinnedImageWindow';
import ScreenshotSession from './components/ScreenshotSession';
import {
  CAPTURE_WINDOW_LABEL,
  readCaptureLaunch,
} from './components/ScreenshotSession/windowMode';
import { useAppStore } from './stores/appStore';
import { useSettingsStore } from './stores/settingsStore';

const currentWindow = getCurrentWebviewWindow();
const captureLaunch = readCaptureLaunch(window.location.search);
const pinnedImageId = readPinnedImageLaunch(window.location.search);

function App() {
  const resultWindowVisible = useAppStore((state) => state.resultWindowVisible);
  const setSourceText = useAppStore((state) => state.setSourceText);
  const requestAutoTranslate = useAppStore((state) => state.requestAutoTranslate);
  const showResultWindow = useAppStore((state) => state.showResultWindow);
  const setActiveMainTab = useSettingsStore((state) => state.setActiveMainTab);
  const setScreenshotSubTab = useSettingsStore((state) => state.setScreenshotSubTab);
  const setCapturedScreenshot = useSettingsStore((state) => state.setCapturedScreenshot);
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

  if (isCaptureWindow) {
    return (
      <ScreenshotSession
        initialMode={captureLaunch?.mode ?? 'screenshot'}
        initialSessionId={captureLaunch?.sessionId}
        onInactive={() => currentWindow.close()}
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
