import { useEffect, useRef, useState, type ReactNode } from 'react';
import type { NativeSettingsPageKind } from '../../application/settings/ports';
import { useSettingsStore } from '../../stores/settingsStore';
import { useSettingsRuntime } from './runtimeContext';

let nextPage = 0;
let presentationQueue = Promise.resolve();

// The retained Settings shell supplies only the available rectangle. AppKit
// owns the page's controls, scrolling, persistence and asynchronous actions.
export function NativeSettingsPage({ page, fallback: retainedPage, interfaceScale }: { page: NativeSettingsPageKind; fallback: ReactNode; interfaceScale?: number }) {
  const present = useSettingsRuntime().window.presentNativeSettingsPage;
  const requestedSection = useSettingsStore((state) => state.requestedSection);
  const consumeSection = useSettingsStore((state) => state.consumeRequestedSection);
  const element = useRef<HTMLDivElement>(null);
  const section = useRef(requestedSection);
  const scale = useRef(interfaceScale);
  scale.current = interfaceScale;
  const refresh = useRef<(() => void) | null>(null);
  const [fallback, setFallback] = useState(!present);
  const [preview, setPreview] = useState<string | null>(null);

  useEffect(() => {
    section.current = requestedSection;
    refresh.current?.();
  }, [requestedSection, interfaceScale]);

  useEffect(() => {
    if (!present || !element.current) return;
    const target = element.current;
    const token = `${page}-${++nextPage}`;
    let disposed = false;
    let supported = true;
    const update = () => {
      presentationQueue = presentationQueue.then(async () => {
        if (disposed || !supported) return;
        const rect = target.getBoundingClientRect();
        if (rect.width <= 0 || rect.height <= 0 || target.clientWidth <= 0) return;
        const requested = section.current;
        const hidden = document.querySelector('dialog[open], [aria-modal="true"]') !== null;
        const result = await present({ token, page, viewport: {
          x: rect.x, y: rect.y, width: rect.width, height: rect.height,
          scale: scale.current ?? rect.width / target.clientWidth,
        }, ...(requested ? { section: requested } : {}), ...(hidden ? { hidden: true } : {}) });
        if (disposed) return;
        supported = result.supported;
        setPreview(hidden && supported ? result.preview ?? null : null);
        if (!supported) setFallback(true);
        else if (requested && section.current === requested) consumeSection();
      }).catch(async (error) => {
        if (!disposed) {
          setFallback(true);
          supported = false;
          // A failed resize must also remove the previous native surface.
          await present({ token, page, viewport: null }).catch(() => undefined);
          console.warn('Failed to present native settings:', error);
        }
      });
    };
    refresh.current = update;
    const observer = new ResizeObserver(update);
    observer.observe(target);
    // Retained permission/capacity/result dialogs must keep the whole Settings
    // content underneath them while the remaining surfaces are migrated.
    const modals = new MutationObserver((records) => {
      if (records.some((record) => record.type === 'attributes'
        || [...record.addedNodes, ...record.removedNodes].some((node) => node instanceof Element
          && (node.matches('dialog, [aria-modal="true"]') || node.querySelector('dialog, [aria-modal="true"]'))))) update();
    });
    modals.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['open', 'aria-modal'] });
    window.addEventListener('resize', update);
    window.addEventListener('focus', update);
    update();
    return () => {
      disposed = true;
      refresh.current = null;
      observer.disconnect();
      modals.disconnect();
      window.removeEventListener('resize', update);
      window.removeEventListener('focus', update);
      // Serialize disposal after an in-flight presentation. A late IPC result
      // cannot make the old native view cover the next Settings tab.
      presentationQueue = presentationQueue.then(async () => {
        await present({ token, page, viewport: null });
      }).catch(() => undefined);
    };
  }, [present, consumeSection, page]);

  const label = { general: '通用设置', ocr: 'OCR 设置', screenshot: '截图设置', translation: '翻译设置', history: '历史记录', favorites: '收藏', services: '服务' }[page];
  return fallback ? retainedPage : <div ref={element} className="relative min-h-0 min-w-0 flex-1" aria-label={label}>
    {preview && <img src={`data:image/png;base64,${preview}`} alt="" aria-hidden="true" data-native-settings-preview className="pointer-events-none absolute inset-0 h-full w-full select-none" />}
  </div>;
}
