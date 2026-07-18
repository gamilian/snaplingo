import { useEffect, useState, type CSSProperties, type ReactNode } from 'react';
import {
  areRequiredPermissionsGranted,
  type RequiredPermissionsRuntime,
  type RequiredPermissionsStatus,
} from '../application/permissions/runtime';

export function RequiredPermissionsGate({
  children,
  runtime,
}: {
  children: ReactNode;
  runtime: RequiredPermissionsRuntime;
}) {
  const [status, setStatus] = useState<RequiredPermissionsStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [requestVersion, setRequestVersion] = useState(0);

  useEffect(() => {
    let disposed = false;
    let timer: number | undefined;

    const check = async (request: boolean) => {
      try {
        const next = request ? await runtime.request() : await runtime.status();
        if (disposed) return;
        setStatus(next);
        setError(null);
        if (!areRequiredPermissionsGranted(next)) {
          timer = window.setTimeout(() => void check(false), 750);
        }
      } catch (cause) {
        if (disposed) return;
        setError(cause instanceof Error ? cause.message : String(cause));
        timer = window.setTimeout(() => void check(false), 1500);
      }
    };

    // Initial mount only checks status. Native permission requests are always
    // initiated by an explicit action in the in-app guidance.
    void check(requestVersion > 0);
    return () => {
      disposed = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [requestVersion, runtime]);

  const shouldShowGuide = status
    ? !areRequiredPermissionsGranted(status)
    : error !== null;

  if (!shouldShowGuide) return children;

  return (
    <>
      {children}
      <div style={styles.overlay}>
        <section
          aria-label="SnapLingo 系统权限引导"
          aria-modal="true"
          role="dialog"
          style={styles.card}
        >
          <h1 style={styles.title}>SnapLingo 需要系统权限</h1>
          <p style={styles.description}>
            首次使用前请完成以下 macOS 授权。设置窗口会保持打开，并在授权完成后自动解锁全部功能。
          </p>
          <PermissionRow
            label="屏幕录制"
            granted={status?.screenRecording ?? false}
            detail="用于截图、OCR 和截图翻译"
          />
          <PermissionRow
            label="辅助功能"
            granted={status?.accessibility ?? false}
            detail="用于读取所选文本、全局快捷键和界面元素检测"
          />
          {error && <p style={styles.error}>{error}</p>}
          <button
            style={styles.button}
            onClick={() => setRequestVersion((version) => version + 1)}
          >
            {!status?.screenRecording
              ? '打开屏幕录制设置'
              : '打开辅助功能设置'}
          </button>
          <p style={styles.hint}>
            点击后会打开对应的 macOS 系统设置页面。允许当前权限并返回 SnapLingo，再继续完成下一项。
          </p>
        </section>
      </div>
    </>
  );
}

function PermissionRow({
  label,
  granted,
  detail,
}: {
  label: string;
  granted: boolean;
  detail: string;
}) {
  return (
    <div style={styles.row}>
      <span style={styles.indicator}>{granted ? '✓' : '!'}</span>
      <span>
        <strong>{label}</strong>
        <small style={styles.detail}>{detail}</small>
      </span>
      <span style={granted ? styles.granted : styles.missing}>
        {granted ? '已授权' : '待授权'}
      </span>
    </div>
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
    background: 'rgba(15, 23, 42, 0.48)',
    backdropFilter: 'blur(6px)',
    color: '#111827',
  },
  card: { width: 'min(520px, 100%)', padding: 28, borderRadius: 18, background: '#fff', boxShadow: '0 20px 50px rgba(15, 23, 42, 0.12)' },
  title: { margin: '0 0 10px', fontSize: 24 },
  description: { margin: '0 0 22px', color: '#4b5563', lineHeight: 1.6 },
  row: { display: 'grid', gridTemplateColumns: '32px 1fr auto', gap: 12, alignItems: 'center', padding: '14px 0', borderTop: '1px solid #e5e7eb' },
  indicator: { display: 'grid', placeItems: 'center', width: 26, height: 26, borderRadius: 13, background: '#e5e7eb', fontWeight: 700 },
  detail: { display: 'block', marginTop: 3, color: '#6b7280' },
  granted: { color: '#15803d', fontWeight: 600 },
  missing: { color: '#b45309', fontWeight: 600 },
  button: { width: '100%', marginTop: 20, padding: '11px 16px', border: 0, borderRadius: 10, background: '#2563eb', color: '#fff', fontWeight: 600, cursor: 'pointer' },
  hint: { margin: '14px 0 0', color: '#6b7280', fontSize: 13, lineHeight: 1.5 },
  error: { color: '#b91c1c', fontSize: 13 },
};
