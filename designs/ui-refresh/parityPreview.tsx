// Preview context: use the production Views and existing SnapLingo tokens.
// Native ports are in-memory fixtures; this page cannot change application data.
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { useSettingsConfigStore } from '../../reference/web/stores/settingsConfigStore';
import { useHotkeyConfigStore } from '../../reference/web/stores/hotkeyConfigStore';
import { useSettingsStore } from '../../reference/web/stores/settingsStore';
import { SettingsWindow } from '../../reference/web/views/SettingsWindow';
import CaptureWorkspace from '../../reference/web/views/CaptureWorkspace';
import { PinnedImageWindow } from '../../reference/web/views/PinnedImageWindow';
import { createPinnedImageRuntime } from '../../reference/web/application/pinned-image/runtime';
import { createScrollingCaptureRuntime } from '../../reference/web/application/scrolling-capture/runtime';
import { ScrollingCaptureWindow, ScrollingCapturePreviewWindow } from '../../reference/web/views/ScrollingCaptureWindow';
import { useApplicationTheme } from '../../reference/web/views/useApplicationTheme';
import { DEFAULT_ANNOTATION_SHORTCUTS } from '../../reference/web/application/capture-workspace/annotationShortcuts';

const params = new URLSearchParams(location.search);
const view = params.get('view');
if (view === 'long-preview') document.documentElement.dataset.scrollingPreview = 'true';
if (view === 'pin') document.documentElement.dataset.pinnedImage = 'true';
const scale = params.get('scale') ?? '100';
const feedback = (text: string) => window.parent.postMessage({ parityPreview: text }, location.origin);
const colors = [[255,77,79,255], [40,167,69,255], [24,144,255,255], [250,219,20,255], [255,255,255,255], [0,0,0,255]];
const screenshot = { savePath: '~/Pictures/SnapLingo', format: 'png', quality: 90, namingRule: 'timestamp', customFileName: 'SnapLingo', autoCopy: false, defaultStrokeWidth: 2, defaultFontSize: 24, rememberLastTool: false, showSelectionSize: true, showMagnifier: true, magnifierZoom: 12, pinOpacity: 100, pinShadow: true, pinBorder: false, pinBorderShortcut: 'b', pinPosition: 'original', captureUiScale: scale, annotationColors: colors, annotationToolShortcuts: DEFAULT_ANNOTATION_SHORTCUTS, longshotMaxHeight: 30000 };
const snapshot = () => useSettingsConfigStore.getState();
useSettingsConfigStore.setState({
  hydrated: true, general: { language: 'zh-CN', theme: params.get('theme') ?? 'light', interfaceTheme: 'native', startOnBoot: false }, screenshot,
  history: { maximumRecords: 5000, maximumFavorites: 1000, autoCleanupEnabled: false, retentionDays: 30 },
  translation: { defaultSourceLang: 'auto', defaultTargetLang: 'zh-CN', autoTranslate: true, autoCopy: false },
  ocr: { recognitionLanguage: 'auto', preserveFormatting: true, removeChineseSpaces: true, showConfidence: false },
  updateScreenshotSettings: async (input) => { useSettingsConfigStore.setState({ screenshot: { ...snapshot().screenshot, ...input } }); feedback('设置已保存到预览'); return snapshot(); },
  updateGeneralSettings: async (input) => { useSettingsConfigStore.setState({ general: { ...snapshot().general, ...input } }); return snapshot(); },
  updateAnnotationColors: async (annotationColors) => { useSettingsConfigStore.setState({ screenshot: { ...snapshot().screenshot, annotationColors } }); return snapshot(); },
});
const hotkeys = { screenshot: { screenshot: 'F1', 'screenshot-copy': '未设置', pin: 'F3', 'pin-toggle-all': '⇧F3', 'pin-switch-group': '⌘F3' }, translation: {}, ocr: {} };
useHotkeyConfigStore.setState({ hydrated: true, snapshot: hotkeys, defaultSnapshot: hotkeys, beginRecording: async () => {}, endRecording: async () => {}, subscribeRecordedHotkey: async () => () => {} });
useSettingsStore.setState({ activeMainTab: params.get('tab') === 'general' ? 'general' : 'screenshot' });
const noop = async () => {};
const runtime: any = {
  window: { version: async () => '0.1.23', displayBounds: async () => ({ x: 0, y: 0, width: innerWidth, height: innerHeight }), applyInterfaceScale: noop, subscribeNavigationRequested: async () => () => {}, selectScreenshotDirectory: async () => null, openExternal: async (url) => feedback(url) },
  library: { queryHistory: async () => ({ total: 0, records: [] }), queryFavorites: async () => ({ total: 0, records: [] }) },
  updates: { subscribe: () => () => {}, ensureChecked: noop, check: noop, openReleasePage: noop },
  permissions: { subscribe: (listener) => { listener({ status: { screenRecording: true, accessibility: true }, error: null }); return () => {}; }, context: async () => ({ platform: 'macos', needsInstallation: false, canReset: false }) },
  maintenance: { listAppLogs: async () => [], clearAppLogs: noop },
};

function sampleImage(width = 1080, height = 680) {
  const canvas = document.createElement('canvas'); canvas.width = width; canvas.height = height;
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = '#eef2f6'; ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = '#fff'; ctx.fillRect(24, 24, width - 48, height - 48);
  ctx.fillStyle = '#344a7a'; ctx.fillRect(24, 24, width - 48, 62);
  ctx.font = '600 21px -apple-system, "PingFang SC", sans-serif'; ctx.fillStyle = '#fff'; ctx.fillText('产品设计 · 评审记录', 48, 63);
  ctx.fillStyle = '#202f46'; ctx.font = '600 32px -apple-system, "PingFang SC", sans-serif'; ctx.fillText('从想法到清晰的表达', 60, 155);
  ctx.font = '18px -apple-system, "PingFang SC", sans-serif'; ctx.fillStyle = '#67748c'; ctx.fillText('用截图记录细节，用标注传达重点。', 60, 195);
  for (let i = 0; i < 3; i++) {
    const x = 60 + i * 320; ctx.fillStyle = ['#e9effe','#e4f3ef','#fdf1dc'][i]; ctx.fillRect(x, 236, 284, 170);
    ctx.fillStyle = '#344665'; ctx.font = '600 21px -apple-system, "PingFang SC", sans-serif'; ctx.fillText(['快速选择','清晰标注','自由输出'][i], x+20, 275);
    ctx.fillStyle = '#6c7a8f'; ctx.font = '16px -apple-system, "PingFang SC", sans-serif'; ctx.fillText(['关注需要的区域','呈现关键的信息','复制、保存与贴图'][i], x+20, 318);
  }
  ctx.fillStyle = '#f3f5f8'; ctx.fillRect(60, 440, width-120, 130);
  ctx.fillStyle = '#59697f'; ctx.font = '18px -apple-system, "PingFang SC", sans-serif'; ctx.fillText('设计反馈  /  联系邮箱 design@example.com', 82, 486);
  ctx.fillText('让每一个操作都有明确的结果。', 82, 529);
  return canvas.toDataURL('image/png');
}
const imageUrl = sampleImage();
const base64 = imageUrl.split(',')[1];
const session = { id: 'preview-capture', monitors: [{ id: 'preview-monitor', logical_bounds: { x: 0, y: 0, width: 1080, height: 680 }, physical_bounds: { x: 0, y: 0, width: 1080, height: 680 }, scale_factor: 1, image_base64: base64 }], candidates: [{ id: 'screen', kind: 'monitor', rect: { x: 0, y: 0, width: 1080, height: 680 }, priority: 1 }] };
const capturePorts: any = {
  commands: { createCaptureSession: async () => session, getCaptureSession: async () => session, hydrateCaptureSessionSnapshots: async () => session, currentCaptureCursorPosition: async () => null, hydrateCaptureMonitorSnapshot: async () => session.monitors[0], listAnnotationFontFamilies: async () => ['PingFang SC', 'Noto Sans CJK SC', 'Helvetica'], logCaptureFrontendPerf: noop, cancelCaptureSession: noop, restoreCaptureSnapshotWindowsForSession: noop, renderCaptureOutput: async () => base64, defaultCaptureSavePath: async () => '/preview/capture.png', outputCapture: async (input) => feedback(`已${input.action.type === 'copy' ? '复制' : input.action.type === 'pin' ? '贴图' : '保存'}选区`), beginScrollingCapture: async () => feedback('长截图已启动') },
  events: { subscribeHotkeyTriggered: async () => () => {} }, window: { signalReady: noop, prepareForReveal: noop, reveal: noop, hide: noop }, clipboard: { writeText: noop }, print: { printImage: noop },
};
const longView = { id: 'preview-long', revision: 1, width: 1080, height: 6240, status: 'capturing', can_undo: true, hint: null, preview_expanded: true, shortcut_actions: ['complete','undo','cancel'], preview_base64: base64, ui_scale: Number(scale) / 100 };
const scrolling = createScrollingCaptureRuntime(longView.id, {
  commands: { get: async () => longView, capture: async () => longView, undo: async () => ({ ...longView, revision: 2, height: 5600 }), finish: async (_, action) => feedback(action.type === 'complete' ? '已复制长图，并保存 PNG 到默认目录' : '已输出长图'), cancel: async () => feedback('已取消长截图'), saveAction: async () => ({ type: 'save', path: '/preview/long.png', format: 'png', quality: 90, copyAfterSave: false }) },
  events: { subscribeUpdates: async () => () => {}, subscribeErrors: async () => () => {} }, window: { resizePreview: async (value) => value, startDragging: noop },
} as any, { sampling: false });
const pinImage = { id: 'preview-pin', width: 480, height: 302, image_base64: base64 };
const pinned = createPinnedImageRuntime({ imageId: pinImage.id,
  commands: { getPinnedImage: async () => pinImage, copyPinnedImage: async () => feedback('已复制当前缩放尺寸'), defaultCaptureSavePath: async () => '/preview/pin.webp', quickCaptureSavePath: async () => '/preview/pin.png', savePinnedImage: async () => feedback('已保存当前尺寸'), closePinnedImage: noop, removePinnedImage: noop, closeAllPinnedImages: async () => { feedback('已关闭全部贴图'); return [pinImage.id]; } } as any,
  clipboard: { writeText: noop }, settings: { open: noop }, window: { resize: noop, moveBy: noop, startDragging: noop, setShadow: noop, close: noop },
});

function Preview() {
  const general = useSettingsConfigStore((state) => state.general);
  useApplicationTheme(general, view === 'settings' ? 'settings' : 'application');
  if (view === 'settings') return <SettingsWindow runtime={runtime} />;
  if (view === 'capture') return <CaptureWorkspace ports={capturePorts} initialMode="screenshot" initialSessionId={session.id} />;
  if (view === 'long') return <ScrollingCaptureWindow runtime={scrolling} />;
  if (view === 'long-preview') return <ScrollingCapturePreviewWindow runtime={scrolling} />;
  if (view === 'pin') return <PinnedImageWindow runtime={pinned} />;
  return <Review />;
}
function Review() {
  const [mode, setMode] = useState('settings'); const [factor, setFactor] = useState('100'); const [theme, setTheme] = useState('light'); const [message, setMessage] = useState('');
  React.useEffect(() => { const listener = (event) => { if (event.origin === location.origin && event.data.parityPreview) setMessage(event.data.parityPreview); }; addEventListener('message', listener); return () => removeEventListener('message', listener); }, []);
  const sizes = { settings: [1200, 820], capture: [1200, 820], long: [408 * Number(factor)/100,132 * Number(factor)/100], pin: [480,360], 'long-preview': [240,360] };
  const size = sizes[mode];
  return <div className="review-page"><header><div><h1>SnapLingo · 功能补齐预览</h1><p>使用实际界面组件。这里的操作只影响预览。</p></div><div className="review-controls">
    <select aria-label="预览界面" value={mode} onChange={(event) => { setMode(event.target.value); setMessage(''); }}><option value="settings">设置与许可</option><option value="capture">截图编辑器</option><option value="long">长截图控制条</option><option value="long-preview">透明长图预览</option><option value="pin">贴图菜单</option></select>
    <select aria-label="预览缩放" value={factor} onChange={(event) => setFactor(event.target.value)}><option value="100">100%</option><option value="150">150%</option><option value="200">200%</option></select>
    <select aria-label="预览主题" value={theme} onChange={(event) => setTheme(event.target.value)}><option value="light">浅色</option><option value="dark">深色</option></select>
  </div></header><main><iframe key={`${mode}/${factor}/${theme}`} title="实际界面预览" style={{ width: size[0], height: size[1], flexShrink: 0, marginInline: 'auto' }} src={`?view=${mode}&scale=${factor}&theme=${theme}`} /></main><p role="status" className="review-feedback">{message || (mode === 'capture' ? '拖动选择区域，然后使用标注工具。' : mode === 'pin' ? '右键打开贴图菜单；滚轮调大小。' : '可切换界面、主题和缩放比例。')}</p></div>;
}
createRoot(document.getElementById('root')!).render(<Preview />);
