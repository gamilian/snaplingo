// Visual review harness: production views and application runtimes, in-memory ports.
import { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { SettingsWindow } from '../../src/views/SettingsWindow';
import ResultWindow from '../../src/views/ResultWindow';
import { PinnedImageWindow } from '../../src/views/PinnedImageWindow';
import { CaptureEditorToolbar } from '../../src/views/CaptureWorkspace/captureEditorToolbar';
import { ANNOTATION_COLORS, DEFAULT_ANNOTATION_STYLE, type AnnotationTool } from '../../src/application/capture-workspace/annotationStyle';
import { createPinnedImageRuntime } from '../../src/application/pinned-image/runtime';
import { createResultWindowRuntime } from '../../src/application/result-window/runtime';
import { useSettingsConfigStore } from '../../src/stores/settingsConfigStore';
import { useSettingsStore } from '../../src/stores/settingsStore';
import { useProviderStore } from '../../src/stores/providerStore';
import { initializeResultWindowStore } from '../../src/stores/resultWindowStore';
import { useApplicationTheme } from '../../src/views/useApplicationTheme';
import { runtime as settingsRuntime, ready } from '../ui-refresh/themes-preview-runtime';
import '../../src/styles/index.css';

const noop = async () => undefined;
const source = 'Good design is as little design as possible. Less, but better.\nDieter Rams · Ten principles for good design';
const result = '好的设计是尽可能少的设计。更少，却更好——因为它专注于本质。';
const surface = new URLSearchParams(location.search).get('surface');

function sampleImage() {
  const canvas = document.createElement('canvas');
  canvas.width = 600; canvas.height = 360;
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = '#f4f1e7'; ctx.fillRect(0, 0, 600, 360);
  ctx.fillStyle = '#707865'; ctx.font = '12px sans-serif'; ctx.fillText('N O T E S  O N  D E S I G N', 40, 45);
  ctx.fillStyle = '#354337'; ctx.font = 'bold 64px Georgia'; ctx.fillText('Less,', 40, 135);
  ctx.font = 'italic 64px Georgia'; ctx.fillText('but better.', 40, 205);
  ctx.font = '18px Georgia'; ctx.fillText('Good design is as little design as possible.', 40, 265);
  ctx.font = '11px sans-serif'; ctx.fillText('D I E T E R   R A M S', 40, 320);
  return canvas.toDataURL('image/png');
}
const image = sampleImage();
const pinned = createPinnedImageRuntime({
  imageId: 'review',
  commands: {
    getPinnedImage: async () => ({ id: 'review', image_base64: image.split(',')[1], width: 600, height: 360 }),
    defaultCaptureSavePath: async () => null, quickCaptureSavePath: async () => '',
    copyPinnedImage: noop, savePinnedImage: noop,
    replacePinnedImageFromClipboard: async () => ({ id: 'review', image_base64: image.split(',')[1], width: 600, height: 360 }),
    closePinnedImage: noop, removePinnedImage: noop, movePinnedImageToNextGroup: async () => 1,
    hidePinnedImageGroup: async () => [], destroyPinnedImageGroup: async () => [], closeAllPinnedImages: async () => [],
  },
  clipboard: { writeText: noop }, settings: { open: noop },
  window: { resize: noop, moveBy: noop, startDragging: noop, setShadow: noop, close: noop },
});
const resultRuntime = createResultWindowRuntime({
  platform: {
    commands: {
      currentPayloadRequestId: async () => null, takePayload: async () => null, selectImageFile: async () => null,
      recognizeImageFile: async () => ({ text: source, confidence: .984, imageDataUrl: image }),
      translateTextWithProvider: async (provider_id) => ({ provider_id, translated_text: result, detected_language: 'en', confidence: null }),
      recordTranslationHistory: noop, favoriteTranslationResult: async () => 1, favoriteOcrResult: async () => 1,
    },
    reveal: noop, clipboard: { copyText: noop }, onPayloadReady: async () => () => undefined, resizeTo: noop,
    placeAt: noop, dismiss: noop, beginDrag: async () => ({ x: 0, y: 0 }), setAlwaysOnTop: noop,
  },
  speech: { speak: noop },
  providers: {
    getState: () => useProviderStore.getState(),
    loadTranslation: () => useProviderStore.getState().loadTranslationProviders(),
    loadOcr: () => useProviderStore.getState().loadOcrProviders(),
  },
  getTranslationSettings: () => useSettingsConfigStore.getState().translation ?? undefined,
  getOcrSettings: () => useSettingsConfigStore.getState().ocr ?? undefined,
  positionStore: { load: () => undefined, save: noop },
});
initializeResultWindowStore(resultRuntime);

function CaptureReview() {
  const [tool, setTool] = useState<AnnotationTool | null>('rectangle');
  const [style, setStyle] = useState(DEFAULT_ANNOTATION_STYLE);
  const [colors, setColors] = useState(ANNOTATION_COLORS);
  const [fontSize, setFontSize] = useState(24);
  return <div style={{ height: '100vh', background: '#1a2130', padding: '32px', display: 'grid', placeItems: 'start center' }}>
    <img src={image} alt="截图示例" style={{ width: 'min(600px, 90vw)', marginTop: 20 }} />
    <CaptureEditorToolbar position={{ x: Math.max(8, (innerWidth - 680) / 2), y: 455 }} width={680} activeAnnotationTool={tool} annotationStyle={style} annotationColorPresets={colors} textFontSize={fontSize} textDraftActive={false} isTextSizingActive={tool === 'text'} isFillModeActive={tool === 'rectangle' || tool === 'ellipse'} canUndo canRedo isRenderingOutput={false} onSelectMove={() => setTool(null)} onToggleAnnotationTool={setTool} onApplyAnnotationStyle={(next, size) => { setStyle(next); setFontSize(size); }} onUpdateAnnotationColorPresets={setColors} onTextDraftFontSizeChange={setFontSize} onCommitSizeDefault={() => undefined} onUndo={() => undefined} onRedo={() => undefined} onClear={() => undefined} onCancel={noop} onRunOcr={noop} onBeginScrollingCapture={noop} onCopy={noop} onSave={noop} onQuickSave={noop} onPin={noop} onFavorite={noop} />
  </div>;
}

function Review() {
  const general = useSettingsConfigStore((state) => state.general);
  useApplicationTheme(general, 'application');
  const [scene, setScene] = useState('settings');
  const [dark, setDark] = useState(new URLSearchParams(location.search).get('dark') === 'true');
  useEffect(() => { document.documentElement.dataset.theme = dark ? 'dark' : 'light'; }, [dark]);
  if (surface === 'settings') return <SettingsWindow runtime={settingsRuntime} />;
  if (surface === 'capture') return <CaptureReview />;
  if (surface === 'pin') return <PinnedImageWindow runtime={pinned} />;
  if (surface === 'translate' || surface === 'ocr') return <ResultWindow runtime={resultRuntime} presentation="standalone" />;
  const scenes = { settings: '设置与资料库', capture: '截图编辑器', pin: '桌面贴图', translate: '翻译浮窗', ocr: 'OCR 浮窗' };
  return <main style={{ minHeight: '100vh', background: 'rgb(var(--sunken))', padding: 24 }}>
    <header style={{ display: 'flex', alignItems: 'center', gap: 12, flexWrap: 'wrap', marginBottom: 18 }}>
      <strong style={{ marginRight: 'auto' }}>SnapLingo · 正式组件预览</strong>
      {Object.entries(scenes).map(([id, label]) => <button className="settings-action" key={id} aria-pressed={scene === id} onClick={() => setScene(id)}>{label}</button>)}
      <button className="settings-action" onClick={() => setDark(!dark)}>切换{dark ? '浅色' : '深色'}</button>
    </header>
    <iframe key={`${scene}-${dark}`} title="正式界面" src={`./production.html?surface=${scene}&dark=${dark}`} style={{ display: 'block', width: scene === 'settings' ? '100%' : scene === 'capture' ? 900 : scene === 'pin' ? 650 : 700, maxWidth: '100%', height: scene === 'pin' ? 480 : 700, margin: 'auto', border: '1px solid rgb(var(--line-strong))', borderRadius: 14, background: scene === 'pin' ? '#e5e9ef' : 'rgb(var(--canvas))' }} />
    <p style={{ marginTop: 14, fontSize: 12, color: 'rgb(var(--muted))' }}>正式 React 组件 · 示例数据与本地端口 · 系统剪贴板、原生窗口和在线服务请在桌面应用中验证。</p>
  </main>;
}

void ready.then(async () => {
  await Promise.all([useProviderStore.getState().loadTranslationProviders(), useProviderStore.getState().loadOcrProviders()]);
  if (surface === 'translate' || surface === 'ocr') {
    await resultRuntime.applyPayload(surface === 'ocr' ? { mode: 'ocr', origin: 'ocr', text: source, autoTranslate: false, ocrIntent: 'display-text', imageBase64: image, confidence: .984, detectedLanguage: 'en' } : { mode: 'translation', origin: 'input', text: source, autoTranslate: true });
  }
  useSettingsStore.getState().setActiveMainTab('overview');
  const dark = new URLSearchParams(location.search).get('dark') === 'true';
  await useSettingsConfigStore.getState().updateGeneralSettings({ theme: dark ? 'dark' : 'light', interfaceTheme: 'native' });
  createRoot(document.getElementById('root')!).render(<Review />);
});
