import { SettingsWindow } from './components/SettingsWindow';
import ResultWindow from './components/ResultWindow';
import { useAppStore } from './stores/appStore';
import { useSettingsStore } from './stores/settingsStore';
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';

function App() {
  const resultWindowVisible = useAppStore((state) => state.resultWindowVisible);
  const setActiveMainTab = useSettingsStore((state) => state.setActiveMainTab);
  const setScreenshotSubTab = useSettingsStore((state) => state.setScreenshotSubTab);
  const setCapturedScreenshot = useSettingsStore((state) => state.setCapturedScreenshot);

  useEffect(() => {
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
  }, [setActiveMainTab, setCapturedScreenshot, setScreenshotSubTab]);

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
