import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import {
  areRequiredPermissionsGranted,
  type RequiredPermissionsContext,
  type RequiredPermissionsRuntime,
} from '../application/permissions/runtime';
import { permissionGrantExplanation, useRequiredPermissions } from './PermissionControls';
import { ScreenshotIcon } from './SettingsWindow/Icons';

export function RequiredPermissionsGate({
  children,
  runtime,
  onOpenSettings,
}: {
  children: ReactNode;
  runtime: RequiredPermissionsRuntime;
  onOpenSettings: () => void;
}) {
  const [dismissed, setDismissed] = useState(false);
  const [context, setContext] = useState<RequiredPermissionsContext | null>(null);
  const [busy, setBusy] = useState(false);
  const [attempted, setAttempted] = useState(false);
  const [actionError, setActionError] = useState(false);
  const actionPending = useRef(false);
  const dialogRef = useRef<HTMLElement>(null);
  const snapshot = useRequiredPermissions(runtime);
  const { status, error } = snapshot;

  const shouldShowGuide = status !== null && error === null
    && !areRequiredPermissionsGranted(status);

  const showGuide = !dismissed && shouldShowGuide;
  const needsInstallation = context?.needsInstallation === true;

  useEffect(() => {
    if (!showGuide) return;
    let disposed = false;
    runtime.context().then(
      (value) => { if (!disposed) setContext(value); },
      () => { if (!disposed) setActionError(true); },
    );
    return () => { disposed = true; };
  }, [runtime, showGuide]);

  async function authorize() {
    if (actionPending.current) return;
    actionPending.current = true;
    setBusy(true);
    setActionError(false);
    try {
      const currentContext = context ?? await runtime.context();
      setContext(currentContext);
      if (currentContext.needsInstallation) return;
      const currentStatus = await runtime.refresh();
      if (!areRequiredPermissionsGranted(currentStatus)) {
        setAttempted(true);
        await runtime.request('screenRecording');
      }
    } catch {
      setActionError(true);
    } finally {
      actionPending.current = false;
      setBusy(false);
    }
  }

  useEffect(() => {
    if (!showGuide) return;
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialogRef.current?.focus();
    const keepFocusInGuide = (event: FocusEvent) => {
      if (event.target instanceof Node && !dialogRef.current?.contains(event.target)) dialogRef.current?.focus();
    };
    document.addEventListener('focusin', keepFocusInGuide);
    return () => {
      document.removeEventListener('focusin', keepFocusInGuide);
      if (previousFocus?.isConnected) previousFocus.focus();
    };
  }, [showGuide]);

  return (
    <>
      <div style={{ display: 'contents' }} aria-hidden={showGuide || undefined}>{children}</div>
      {showGuide && <div style={styles.overlay}>
        <section
          ref={dialogRef}
          tabIndex={-1}
          aria-labelledby="screen-permission-title"
          aria-describedby="screen-permission-description"
          aria-modal="true"
          role="dialog"
          className="bg-white text-gray-900 outline-none"
          style={styles.card}
          onKeyDown={(event) => {
            if (event.key === 'Escape') setDismissed(true);
            if (event.key !== 'Tab') return;
            const controls = [...event.currentTarget.querySelectorAll<HTMLElement>('button:not(:disabled)')]
              .filter((element) => element.getClientRects().length > 0);
            const first = controls[0];
            const last = controls[controls.length - 1];
            if (event.shiftKey && (document.activeElement === first || document.activeElement === event.currentTarget)) {
              event.preventDefault();
              last?.focus();
            } else if (!event.shiftKey && document.activeElement === last) {
              event.preventDefault();
              first?.focus();
            }
          }}
        >
          <div aria-hidden="true" className="bg-primary-50 text-primary-600" style={styles.icon}><ScreenshotIcon /></div>
          <h1 id="screen-permission-title" style={styles.title}>{needsInstallation ? '先安装 SnapLingo' : '开启屏幕录制'}</h1>
          <p id="screen-permission-description" style={styles.description} className="text-gray-500">
            {needsInstallation
              ? '当前运行的是临时副本。请将应用拖入“应用程序”后打开，再检测是否需要授权。'
              : '当前运行的 SnapLingo 尚未获得屏幕录制权限，截图、识别和翻译屏幕内容需要此权限。'}
          </p>
          {!needsInstallation && <p className="mb-5 text-left text-xs leading-relaxed text-gray-500">{permissionGrantExplanation}</p>}
          {actionError && <p role="alert" className="mb-4 text-xs text-red-600">暂时无法完成操作，请重试。</p>}
          <div style={styles.actions}>
            <button type="button" className="border border-gray-200 bg-white text-gray-700 hover:bg-gray-50" style={styles.button} onClick={() => setDismissed(true)}>
              {needsInstallation ? '知道了' : '稍后'}
            </button>
            {!needsInstallation && <button type="button" className="bg-primary-600 text-white hover:bg-primary-700 disabled:opacity-50" style={styles.button}
              disabled={busy || (!context && !actionError)} onClick={() => void authorize()}>
              {busy ? '请稍候…' : actionError ? '重试' : '去授权'}
            </button>}
          </div>
          {!needsInstallation && (attempted || actionError) && <button type="button" className="mt-4 text-xs text-gray-500 hover:text-primary-600"
            onClick={() => { setDismissed(true); onOpenSettings(); }}>授权遇到问题？</button>}
        </section>
      </div>}
    </>
  );
}

const styles: Record<string, CSSProperties> = {
  overlay: {
    position: 'fixed',
    inset: 0,
    zIndex: 10000,
    display: 'grid',
    placeItems: 'center',
    padding: 24,
    background: 'rgba(15, 23, 42, 0.32)',
    backdropFilter: 'blur(4px)',
  },
  card: { width: 'min(380px, 100%)', maxHeight: 'calc(100vh - 48px)', overflowY: 'auto', padding: 24, borderRadius: 16, textAlign: 'center', boxShadow: '0 20px 50px rgba(15, 23, 42, 0.16)' },
  icon: { display: 'grid', placeItems: 'center', width: 48, height: 48, margin: '0 auto 16px', borderRadius: 12 },
  title: { margin: '0 0 8px', fontSize: 17, fontWeight: 600 },
  description: { margin: '0 0 24px', fontSize: 13, lineHeight: 1.6 },
  actions: { display: 'flex', gap: 8 },
  button: { flex: 1, padding: '9px 16px', borderRadius: 8, fontSize: 13, fontWeight: 500, cursor: 'pointer' },
};
