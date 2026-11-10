import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { SettingsWindow } from '../../src/views/SettingsWindow';
import { useSettingsConfigStore } from '../../src/stores/settingsConfigStore';
import { useSettingsStore } from '../../src/stores/settingsStore';
import { useApplicationTheme } from '../../src/views/useApplicationTheme';
import TranslationCard from '../../src/views/ResultWindow/TranslationCard';
import { CaptureEditorToolbar } from '../../src/views/CaptureWorkspace/captureEditorToolbar';
import { DEFAULT_ANNOTATION_STYLE, ANNOTATION_COLORS, type AnnotationTool } from '../../src/application/capture-workspace/annotationStyle';
import { DemoCanvas } from './capture.jsx';
import { sourceText, translatedText, alternateTranslation } from './data.js';
import { runtime, ready } from './themes-preview-runtime';
import '../../src/styles/index.css';
import './themes-preview.css';

const names = { native: '澄明 · Native', studio: '夜航 · Studio', paper: '松间 · Paper' };
const noop = () => undefined;
const asyncNoop = async () => undefined;

function ThemePreview() {
  const general = useSettingsConfigStore(state => state.general);
  const updateGeneral = useSettingsConfigStore(state => state.updateGeneralSettings);
  const [scene, setScene] = useState<'settings' | 'tools'>('settings');
  useApplicationTheme(general, scene === 'settings' ? 'settings' : 'application');
  return <div className="theme-preview">
    <header className="theme-preview-header">
      <strong>SnapLingo <span>主题预览</span></strong>
      <nav aria-label="预览窗口"><button onClick={() => setScene('settings')} aria-pressed={scene === 'settings'}>设置窗口</button><button onClick={() => setScene('tools')} aria-pressed={scene === 'tools'}>浮窗与工具</button></nav>
      <div className="theme-preview-options"><select aria-label="预览主题" value={general?.interfaceTheme ?? 'native'} onChange={event => void updateGeneral({ interfaceTheme: event.target.value as keyof typeof names })}>{Object.entries(names).map(([id,label]) => <option key={id} value={id}>{label}</option>)}</select><select aria-label="预览外观" value={general?.theme ?? 'light'} onChange={event => void updateGeneral({ theme: event.target.value })}><option value="light">浅色</option><option value="dark">深色</option><option value="system">跟随系统</option></select></div>
    </header>
    <main className={scene === 'settings' ? 'theme-preview-window' : 'theme-preview-tools'}>{scene === 'settings' ? <SettingsWindow runtime={runtime} /> : <ToolsPreview />}</main>
    <footer className="theme-preview-footer"><span>{names[general?.interfaceTheme ?? 'native']}</span><span>示例数据 · 偏好仅保存在本页</span><a href="./index.html?view=compare">查看原始设计</a></footer>
  </div>;
}

function ToolsPreview() {
  const [activeTool, setActiveTool] = useState<AnnotationTool | null>(null);
  const [style, setStyle] = useState(DEFAULT_ANNOTATION_STYLE);
  const [textSize, setTextSize] = useState(24);
  return <>
    <section className="result-panel theme-preview-result"><header><span>SnapLingo</span><strong>翻译</strong></header><textarea aria-label="示例原文" readOnly value={sourceText} /><div className="theme-preview-results"><TranslationCard providerId="google-translate" providerName="Google 翻译" status="success" text={translatedText} languageCode="zh-CN" copyText={asyncNoop} speakText={asyncNoop} /><TranslationCard providerId="deeplx" providerName="DeepLX" status="success" text={alternateTranslation} languageCode="zh-CN" copyText={asyncNoop} speakText={asyncNoop} /></div></section>
    <section className="theme-preview-capture"><h2>截图工具</h2><DemoCanvas /><CaptureEditorToolbar position={{x:16,y:285}} width={655} activeAnnotationTool={activeTool} annotationStyle={style} annotationColorPresets={ANNOTATION_COLORS} textFontSize={textSize} textDraftActive={false} isTextSizingActive={activeTool === 'text'} isFillModeActive={false} canUndo={false} canRedo={false} isRenderingOutput={false} onSelectMove={() => setActiveTool(null)} onToggleAnnotationTool={tool => setActiveTool(value => value === tool ? null : tool)} onApplyAnnotationStyle={(next,size) => { setStyle(next); setTextSize(size); }} onUpdateAnnotationColorPresets={noop} onTextDraftFontSizeChange={setTextSize} onCommitSizeDefault={noop} onUndo={noop} onRedo={noop} onCancel={noop} onRunOcr={noop} onCopy={noop} onSave={noop} onQuickSave={noop} onPin={noop} onFavorite={noop} /></section>
  </>;
}

void ready.then(() => {
  useSettingsStore.getState().setActiveMainTab('general');
  createRoot(document.getElementById('root')!).render(<ThemePreview />);
});
