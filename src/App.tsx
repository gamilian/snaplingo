import { SettingsWindow } from './components/SettingsWindow';
import ResultWindow from './components/ResultWindow';
import { useAppStore } from './stores/appStore';

function App() {
  const resultWindowVisible = useAppStore((state) => state.resultWindowVisible);

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
