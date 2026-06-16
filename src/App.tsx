import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { SettingsWindow } from './components/SettingsWindow';
import ResultWindow from './components/ResultWindow';
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

const currentWindow = getCurrentWebviewWindow();
const captureLaunch = readCaptureLaunch(window.location.search);
const pinnedImageId = readPinnedImageLaunch(window.location.search);

function App() {
  const resultWindowVisible = useAppStore((state) => state.resultWindowVisible);
  const setSourceText = useAppStore((state) => state.setSourceText);
  const showResultWindow = useAppStore((state) => state.showResultWindow);
  const isCaptureWindow =
    currentWindow.label === CAPTURE_WINDOW_LABEL || captureLaunch !== null;
  const isPinnedImageWindow = pinnedImageId !== null;

  useEffect(() => {
    if (isCaptureWindow || isPinnedImageWindow) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    listen<string>('input-translation', (event) => {
      setSourceText(event.payload);
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
  }, [isCaptureWindow, isPinnedImageWindow, setSourceText, showResultWindow]);

  if (isCaptureWindow) {
    return (
      <ScreenshotSession
        initialMode={captureLaunch?.mode ?? 'screenshot'}
        initialSessionId={captureLaunch?.sessionId}
        onInactive={() => {
          void currentWindow.close();
        }}
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
