import { useEffect, useState } from 'react';
import { useProviderStore } from '../../stores/providerStore';
import { useSettingsConfigStore } from '../../stores/settingsConfigStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useHotkeyConfigStore } from '../../stores/hotkeyConfigStore';
import { useRequiredPermissions } from '../PermissionControls';
import { FeatureHotkeysSection } from './Hotkey/FeatureHotkeysSection';
import { SCREENSHOT_HOTKEYS, TRANSLATION_HOTKEYS, OCR_HOTKEYS } from './Hotkey/actions';
import { AnnotationShortcutSettings } from './Screenshot/AnnotationShortcutSettings';
import { PinBorderShortcutSettings } from './Screenshot/PinBorderShortcutSettings';
import { SettingsGroup } from './SettingsControls';
import { SettingsScrollPage } from './SettingsScrollPage';
import { useSettingsRuntime } from './runtimeContext';

export function OverviewPage() {
  const runtime = useSettingsRuntime();
  const permissions = useRequiredPermissions(runtime.permissions);
  const providers = useProviderStore((state) => state.translationProviders);
  const activeIds = useProviderStore((state) => state.activeTranslationProviders);
  const ocrProviders = useProviderStore((state) => state.ocrProviders);
  const activeOcr = useProviderStore((state) => state.activeOcrProvider);
  const screenshot = useSettingsConfigStore((state) => state.screenshot);
  const updateScreenshot = useSettingsConfigStore((state) => state.updateScreenshotSettings);
  const snapshot = useHotkeyConfigStore((state) => state.snapshot);
  const resetCategory = useHotkeyConfigStore((state) => state.resetCategory);
  const requestedSection = useSettingsStore((state) => state.requestedSection);
  const consumeRequestedSection = useSettingsStore((state) => state.consumeRequestedSection);
  const navigate = useSettingsStore((state) => state.navigate);
  const [error, setError] = useState<string | null>(null);
  const loadTranslation = useProviderStore((state) => state.loadTranslationProviders);
  const loadOcr = useProviderStore((state) => state.loadOcrProviders);
  useEffect(() => {
    let disposed = false;
    void Promise.all([loadTranslation(), loadOcr()]).catch((cause) => { if (!disposed) setError(String(cause)); });
    return () => { disposed = true; };
  }, [loadTranslation, loadOcr]);
  const ocr = ocrProviders.find((provider) => provider.id === activeOcr);
  const ready = permissions.error === null && permissions.status?.screenRecording;

  const resetAll = async () => {
    if (!confirm('恢复全部全局快捷键、贴图边框快捷键与编辑器单键到默认值？')) return;
    try {
      for (const category of ['screenshot', 'translation', 'ocr'] as const) await resetCategory(category);
      await updateScreenshot({ annotationToolShortcuts: {}, pinBorderShortcut: 'b' });
      setError(null);
    } catch (cause) { setError(String(cause)); }
  };
  const detectConflicts = () => {
    if (!snapshot) return;
    const configured = Object.values(snapshot).flatMap((category) => Object.values(category)).filter((key) => key && key !== '未设置');
    const duplicates = configured.filter((key, index) => configured.indexOf(key) !== index);
    alert(duplicates.length ? `发现冲突：${[...new Set(duplicates)].join('、')}` : '未发现全局快捷键冲突');
  };

  return (
    <SettingsScrollPage
      title="概览与快捷键"
      description="系统状态与全部快捷键管理，点击按键即可重新录制"
      requestedSectionId={requestedSection}
      onRequestedSectionHandled={consumeRequestedSection}
      actions={<div className="flex gap-2"><button className="settings-action" onClick={detectConflicts}>检测冲突</button><button className="settings-action" onClick={() => void resetAll()}>全部恢复默认</button></div>}
      intro={<>
        {error && <p role="alert" className="mb-4 text-xs text-danger">{error}</p>}
        <div className="settings-bento-grid">
          <button className="settings-bento" onClick={() => navigate({ tab: 'general', section: 'permissions' })}>
            <span className="flex justify-between gap-2 text-xs text-muted">系统权限<span className={ready ? 'text-good' : 'text-warn'}>{permissions.error ? '检测失败' : !permissions.status ? '检测中' : ready ? '已就绪' : '待授权'}</span></span>
            <strong>屏幕录制与辅助功能</strong><span>查看授权状态与权限修复</span>
          </button>
          <button className="settings-bento" onClick={() => navigate({ tab: 'services', section: 'ocr' })}>
            <span className="flex justify-between gap-2 text-xs text-muted">当前 OCR 引擎<span className="text-good">{ocr?.isLocal ? '本地' : '云端'}</span></span>
            <strong>{ocr?.name ?? '尚未启用'}</strong><span>{ocr?.isLocal ? '离线识别 · 图像在设备端处理' : '管理识别引擎与配置'}</span>
          </button>
          <button className="settings-bento" onClick={() => navigate({ tab: 'services', section: 'translation' })}>
            <span className="flex justify-between gap-2 text-xs text-muted">激活翻译引擎<span className="text-accent">{activeIds.length} 个</span></span>
            <strong>{activeIds.map((id) => providers.find((p) => p.id === id)?.name ?? id).join(' + ') || '尚未启用'}</strong><span>支持并发对比、拖拽排序与自定义 AI</span>
          </button>
        </div>
      </>}
      sections={[
        { id: 'screenshot', label: '截图与贴图快捷键', content: <><FeatureHotkeysSection category="screenshot" actions={SCREENSHOT_HOTKEYS} /><SettingsGroup><PinBorderShortcutSettings value={screenshot?.pinBorderShortcut} onUpdate={(pinBorderShortcut) => updateScreenshot({ pinBorderShortcut })} /></SettingsGroup></> },
        { id: 'translation', label: '翻译快捷键', content: <FeatureHotkeysSection category="translation" actions={TRANSLATION_HOTKEYS} /> },
        { id: 'ocr', label: 'OCR 快捷键', content: <FeatureHotkeysSection category="ocr" actions={OCR_HOTKEYS} /> },
        { id: 'editor', label: '截图编辑器单键', description: '仅在截图编辑器激活时生效', content: <AnnotationShortcutSettings value={screenshot?.annotationToolShortcuts} onUpdate={(annotationToolShortcuts) => updateScreenshot({ annotationToolShortcuts })} /> },
      ]}
    />
  );
}
