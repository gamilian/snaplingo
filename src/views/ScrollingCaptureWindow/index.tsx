import { useEffect, useSyncExternalStore } from 'react';
import type { ScrollingCaptureRuntime } from '../../application/scrolling-capture/runtime';

export function ScrollingCaptureWindow({
  runtime,
}: {
  runtime: ScrollingCaptureRuntime;
}) {
  const { view, working, error, previewExpanded } = useSyncExternalStore(
    runtime.subscribe,
    runtime.getSnapshot,
    runtime.getSnapshot,
  );
  useEffect(() => {
    void runtime.mount();
    return () => runtime.dispose();
  }, [runtime]);
  const actionClass =
    'h-8 rounded border border-line bg-surface px-2 text-xs font-medium text-ink hover:bg-raised disabled:opacity-40';
  return (
    <div className="flex flex-col overflow-hidden rounded-xl border border-line bg-surface p-3 text-ink shadow-lg" style={{ width: `${100 / (view?.ui_scale ?? 1)}vw`, height: `${100 / (view?.ui_scale ?? 1)}vh`, transform: `scale(${view?.ui_scale ?? 1})`, transformOrigin: 'top left' }}>
      <div
        className="mb-2 flex cursor-move items-center justify-between text-sm font-semibold"
        onPointerDown={() => {
          void runtime.startDragging();
        }}
      >
        <span>长截图</span>
        <span className="text-xs font-normal text-muted">
          {view ? `${view.width} × ${view.height}` : '正在准备…'}
        </span>
      </div>
      <div className="flex flex-wrap gap-1.5">
        <button
          type="button"
          className={actionClass}
          disabled={!view?.can_undo || working}
          onClick={() => {
            void runtime.undo();
          }}
          title={
            view?.shortcut_actions.includes('undo') ? 'Ctrl/Cmd+Z' : undefined
          }
        >
          撤销
        </button>
        <button
          type="button"
          className={actionClass}
          disabled={!view || working || view.preview_available === false}
          onClick={() => {
            void runtime.togglePreview();
          }}
          title={view?.preview_available === false ? '当前屏幕没有足够的预览空间' : '显示或隐藏鼠标穿透预览'}
        >
          {previewExpanded ? '隐藏预览' : '预览'}
        </button>
        <button
          type="button"
          className={actionClass}
          disabled={!view || working}
          onClick={() => {
            void runtime.finish({ type: 'copy' });
          }}
        >
          复制
        </button>
        <button
          type="button"
          className={actionClass}
          disabled={!view || working}
          onClick={() => {
            void runtime.save();
          }}
          title="另存为 PNG、JPEG 或 WebP"
        >
          另存为
        </button>
        <button
          type="button"
          className={actionClass}
          disabled={!view || working}
          onClick={() => {
            void runtime.finish({ type: 'pin' });
          }}
        >
          贴图
        </button>
        <button type="button" className="h-8 rounded border border-accent bg-accent px-3 text-xs font-semibold text-accent-on hover:bg-accent-hover disabled:opacity-40" disabled={!view || working} onClick={() => void runtime.finish({ type: 'complete' })} title={view?.shortcut_actions.includes('complete') ? 'Enter · 复制并保存 PNG 到默认目录' : '复制并保存 PNG 到默认目录'}>
          完成
        </button>
        <button
          type="button"
          className={actionClass}
          disabled={working}
          onClick={() => {
            void runtime.cancel();
          }}
          title={view?.shortcut_actions.includes('cancel') ? 'Esc' : undefined}
        >
          取消
        </button>
      </div>
      <p
        className={`mt-2 min-h-0 overflow-y-auto text-xs ${error ? 'text-bad' : 'text-muted'}`}
        role={error ? 'alert' : 'status'}
      >
        {working
          ? '正在处理…'
          : (error ?? view?.hint ?? '手动滚动页面，完成后复制并保存 PNG。')}
        {error && (
          <button
            type="button"
            className="ml-2 underline"
            onClick={runtime.resume}
          >
            重试
          </button>
        )}
      </p>
    </div>
  );
}

export function ScrollingCapturePreviewWindow({ runtime }: { runtime: Pick<ScrollingCaptureRuntime, 'subscribe' | 'getSnapshot' | 'mount' | 'dispose'> }) {
  const { view } = useSyncExternalStore(runtime.subscribe, runtime.getSnapshot, runtime.getSnapshot);
  useEffect(() => { void runtime.mount(); return () => runtime.dispose(); }, [runtime]);
  return <div className="pointer-events-none flex h-screen w-screen items-end justify-center overflow-hidden">
    {view && <img className="max-h-full max-w-full object-contain" src={`data:image/png;base64,${view.preview_base64}`} alt="长截图实时预览" draggable={false} />}
  </div>;
}

export function ScrollingCaptureOutlineWindow({ scale = 1, desktopScale }: { scale?: number; desktopScale?: number }) {
  const borderWidth = 2 * scale * (desktopScale ? desktopScale / window.devicePixelRatio : 1);
  return <div aria-label="长截图选区边框" className="pointer-events-none h-screen w-screen border-blue-500" style={{ borderStyle: 'solid', borderWidth }} />;
}
