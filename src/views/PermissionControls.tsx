import { useEffect, useRef, useState } from 'react';
import type {
  RequiredPermissionsContext,
  RequiredPermissionsRuntime,
  RequiredPermissionsSnapshot,
  SystemPermission,
} from '../application/permissions/runtime';

export function useRequiredPermissions(runtime: RequiredPermissionsRuntime) {
  const [snapshot, setSnapshot] = useState<RequiredPermissionsSnapshot>({ status: null, error: null });

  useEffect(() => {
    const unsubscribe = runtime.subscribe(setSnapshot);
    const refresh = () => { void runtime.refresh().catch(() => undefined); };
    const onVisibility = () => { if (!document.hidden) refresh(); };
    window.addEventListener('focus', refresh);
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      window.removeEventListener('focus', refresh);
      document.removeEventListener('visibilitychange', onVisibility);
      unsubscribe();
    };
  }, [runtime]);

  return snapshot;
}

const permissions: { key: SystemPermission; label: string; detail: string }[] = [
  { key: 'screenRecording', label: '屏幕录制', detail: '截图与屏幕识别' },
  { key: 'accessibility', label: '辅助功能', detail: '选中文本与界面元素检测（可选）' },
];

const buttonClass = 'rounded-lg border border-gray-200 bg-white px-3 py-2 text-xs font-medium text-gray-700 hover:bg-gray-50 disabled:cursor-default disabled:opacity-50';

export function PermissionControls({ runtime, snapshot: { status, error } }: {
  runtime: RequiredPermissionsRuntime;
  snapshot: RequiredPermissionsSnapshot;
}) {
  const [context, setContext] = useState<RequiredPermissionsContext | null>(null);
  const [busy, setBusy] = useState(false);
  const actionPending = useRef(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [repair, setRepair] = useState<SystemPermission | null>(null);

  useEffect(() => {
    let disposed = false;
    runtime.context().then(
      (value) => { if (!disposed) setContext(value); },
      () => { if (!disposed) setActionError('无法读取应用位置，请重新检测。'); },
    );
    return () => { disposed = true; };
  }, [runtime]);

  async function perform(action: () => Promise<unknown>, successNotice: string | null = null) {
    if (actionPending.current) return;
    actionPending.current = true;
    setBusy(true);
    setActionError(null);
    setNotice(null);
    try {
      await action();
      setNotice(successNotice);
    } catch (cause) {
      setActionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      actionPending.current = false;
      setBusy(false);
    }
  }

  if (context && context.platform !== 'macos') {
    return <p className="py-3 text-xs text-gray-500">当前系统无需单独开启屏幕录制或辅助功能权限。</p>;
  }

  const actionDisabled = busy || !status || !context || context.needsInstallation;
  const repairLabel = permissions.find((permission) => permission.key === repair)?.label;

  return (
    <div className="text-left text-sm text-gray-800" aria-busy={busy}>
      {context?.needsInstallation && (
        <p className="mb-3 rounded-lg bg-amber-50 p-3 text-xs leading-relaxed text-amber-800" role="alert">
          将 SnapLingo 拖入“应用程序”，再打开并授权。
        </p>
      )}
      {permissions.map(({ key, label, detail }) => (
        <div key={key} className="flex items-center justify-between gap-4 border-b border-gray-100 py-3">
          <div>
            <p className="font-semibold">{label}</p>
            <p className="mt-1 text-xs leading-relaxed text-gray-500">{detail}</p>
          </div>
          <div className="flex shrink-0 items-center gap-3">
            <span className={`text-xs ${status?.[key] ? 'text-green-700' : 'text-gray-500'}`} aria-live="polite">
              {!status ? '检测中…' : status[key] ? '已授权' : '待授权'}
            </span>
            {!status?.[key] && (
              <button type="button" className={key === 'screenRecording'
                ? 'rounded-lg bg-primary-600 px-3 py-2 text-xs font-medium text-white hover:bg-primary-700 disabled:opacity-50'
                : buttonClass} disabled={actionDisabled}
                aria-label={`授权${label}`} onClick={() => void perform(() => runtime.request(key))}>
                去授权
              </button>
            )}
          </div>
        </div>
      ))}
      {(actionError || error) && <p role="alert" className="mt-3 text-xs text-red-600">{actionError || error}</p>}
      {notice && <p role="status" className="mt-3 text-xs leading-relaxed text-gray-600">{notice}</p>}
      <details className="mt-4 text-xs leading-relaxed text-gray-500">
        <summary className="cursor-pointer font-medium text-gray-700">授权遇到问题？</summary>
        <p className="mt-3">系统已开启但仍不可用时，可重新检测或重新授权。</p>
        {context?.appPath && <p className="mt-2 break-all">当前应用：{context.appPath}</p>}
        <div className="mt-3 flex flex-wrap gap-2">
          <button type="button" className={buttonClass} disabled={busy}
            onClick={() => void perform(() => Promise.all([runtime.refresh(), runtime.context().then(setContext)]))}>
            重新检测
          </button>
          {context?.canReset && permissions.map(({ key, label }) => (
              <button key={key} type="button" className={buttonClass} disabled={busy}
                onClick={() => { setRepair(key); setNotice(null); }}>
                重新授权{label}
              </button>
          ))}
        </div>
        {repair && (
          <div className="mt-3 rounded-lg border border-amber-200 bg-amber-50 p-3 text-amber-900">
            <p>将重置 SnapLingo 的“{repairLabel}”权限，之后需要重新允许。</p>
            <div className="mt-3 flex gap-2">
              <button type="button" className={buttonClass} disabled={busy} onClick={() => {
                const permission = repair;
                setRepair(null);
                void perform(() => runtime.reset(permission), '请在系统设置中重新允许 SnapLingo。');
              }}>确认重置{repairLabel}</button>
              <button type="button" className={buttonClass} disabled={busy} onClick={() => setRepair(null)}>取消</button>
            </div>
          </div>
        )}
        <p className="mt-3">如果 macOS 要求，请重启一次。</p>
        <button type="button" className={`${buttonClass} mt-2`} disabled={actionDisabled}
          onClick={() => void perform(() => runtime.restart())}>重启 SnapLingo</button>
      </details>
    </div>
  );
}
