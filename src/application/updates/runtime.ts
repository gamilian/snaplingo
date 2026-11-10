export interface AppUpdateStatus {
  currentVersion: string;
  latestVersion: string | null;
  available: boolean;
  downloadSize: number | null;
  notes: string;
  publishedAt: string | null;
  platform: string;
}

export interface AppUpdatesPort {
  check(): Promise<AppUpdateStatus>;
  downloadAndOpen(): Promise<string>;
  openReleasePage(): Promise<void>;
}

export interface AppUpdatesSnapshot {
  status: AppUpdateStatus | null;
  phase: 'idle' | 'checking' | 'downloading';
  error: string | null;
  downloadedPath: string | null;
}

export interface AppUpdatesRuntime {
  subscribe(listener: (snapshot: AppUpdatesSnapshot) => void): () => void;
  ensureChecked(): Promise<void>;
  check(): Promise<void>;
  downloadAndOpen(): Promise<void>;
  openReleasePage(): Promise<void>;
}

export function createAppUpdatesRuntime(port: AppUpdatesPort): AppUpdatesRuntime {
  let snapshot: AppUpdatesSnapshot = { status: null, phase: 'idle', error: null, downloadedPath: null };
  const listeners = new Set<(snapshot: AppUpdatesSnapshot) => void>();
  let pending: Promise<void> | null = null;
  let attemptedCheck = false;

  function publish(patch: Partial<AppUpdatesSnapshot>) {
    snapshot = { ...snapshot, ...patch };
    listeners.forEach((listener) => listener(snapshot));
  }

  function run(phase: AppUpdatesSnapshot['phase'], action: () => Promise<Partial<AppUpdatesSnapshot>>) {
    if (pending) return pending;
    publish({ phase, error: null, ...(phase === 'downloading' ? { downloadedPath: null } : {}) });
    pending = Promise.resolve().then(action).then(
      (patch) => { publish({ ...patch, phase: 'idle' }); },
      (cause) => {
        publish({ phase: 'idle', error: cause instanceof Error ? cause.message : String(cause) });
        throw cause;
      },
    ).finally(() => { pending = null; });
    return pending;
  }

  function check() {
    attemptedCheck = true;
    return run('checking', async () => ({ status: await port.check(), downloadedPath: null }));
  }

  return {
    subscribe(listener) {
      listeners.add(listener);
      listener(snapshot);
      return () => { listeners.delete(listener); };
    },
    ensureChecked: () => attemptedCheck ? Promise.resolve() : check(),
    check,
    downloadAndOpen: () => run('downloading', async () => {
      if (!snapshot.status?.available || snapshot.status.downloadSize === null) {
        throw new Error('请先检查更新，确认有适配当前系统的新版安装包。');
      }
      return { downloadedPath: await port.downloadAndOpen() };
    }),
    openReleasePage: () => run('idle', async () => { await port.openReleasePage(); return {}; }),
  };
}
