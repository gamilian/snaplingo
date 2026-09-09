import { useEffect, useState } from 'react';
import type { AppUpdatesSnapshot } from '../../../application/updates/runtime';
import { SettingRow } from '../SettingsControls';
import { useSettingsRuntime } from '../runtimeContext';

const buttonClass = 'rounded-lg border border-gray-200 bg-white px-3 py-2 text-xs font-medium text-gray-700 hover:bg-gray-50 disabled:cursor-default disabled:opacity-50';

export function SoftwareUpdateSettings() {
  const runtime = useSettingsRuntime().updates;
  const [{ status, phase, error, downloadedPath }, setSnapshot] = useState<AppUpdatesSnapshot>({
    status: null, phase: 'idle', error: null, downloadedPath: null,
  });

  useEffect(() => {
    const unsubscribe = runtime.subscribe(setSnapshot);
    void runtime.ensureChecked().catch(() => undefined);
    return unsubscribe;
  }, [runtime]);

  const busy = phase !== 'idle';
  let description = '打开设置时自动查找最新正式版';
  if (phase === 'checking') description = '正在检查更新…';
  else if (phase === 'downloading') description = '正在下载并校验安装包…';
  else if (downloadedPath) description = '安装包已下载并打开';
  else if (status?.available) description = `发现新版本 ${status.latestVersion}`;
  else if (status?.latestVersion) description = '当前已是最新版本';
  else if (status) description = '暂时没有已发布的正式版本';

  return (
    <>
      <SettingRow label="软件更新" description={description}>
        <div className="flex flex-wrap justify-end gap-2" aria-busy={busy}>
          <button type="button" className={buttonClass} disabled={busy}
            onClick={() => void runtime.check().catch(() => undefined)}>检查更新</button>
          <button type="button" className={buttonClass} disabled={busy}
            onClick={() => void runtime.openReleasePage().catch(() => undefined)}>发布页面</button>
        </div>
      </SettingRow>
      <div aria-live="polite">
        {error && <p role="alert" className="py-3 text-xs leading-relaxed text-red-600">{error}</p>}
        {status?.available && (
          <div className="py-4">
            {status.downloadSize !== null ? (
              <>
                <button type="button" disabled={busy}
                  className="rounded-lg bg-primary-600 px-4 py-2 text-xs font-semibold text-white hover:bg-primary-700 disabled:opacity-50"
                  onClick={() => void runtime.downloadAndOpen().catch(() => undefined)}>
                  {phase === 'downloading' ? '正在下载并校验…' : downloadedPath ? '再次下载并打开安装包' : '下载并打开安装包'}
                  {phase !== 'downloading' && `（${Math.ceil(status.downloadSize / 1_000_000)} MB）`}
                </button>
                <p className="mt-3 text-xs leading-relaxed text-gray-500">
                  {status.platform === 'macos'
                    ? '安装包打开后，退出 SnapLingo，将新版拖入“应用程序”替换旧版，再打开。设置和历史记录会保留。'
                    : '下载完成后将打开系统安装器，按提示覆盖安装。设置和历史记录会保留。'}
                </p>
              </>
            ) : (
              <p className="text-xs text-gray-500">此版本暂未提供可验证的适配安装包，请前往发布页面查看。</p>
            )}
            {downloadedPath && <p className="mt-2 break-all text-xs text-gray-500">安装包位置：{downloadedPath}</p>}
            {status.notes && (
              <details className="mt-4 text-xs text-gray-600">
                <summary className="cursor-pointer font-medium">{status.latestVersion} 更新说明</summary>
                <p className="mt-3 whitespace-pre-wrap break-words leading-relaxed">{status.notes}</p>
              </details>
            )}
          </div>
        )}
      </div>
    </>
  );
}
