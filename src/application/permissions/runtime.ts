export interface RequiredPermissionsStatus {
  screenRecording: boolean;
  accessibility: boolean;
}

export type SystemPermission = 'screenRecording' | 'accessibility';

export interface RequiredPermissionsContext {
  platform: string;
  appPath: string | null;
  needsInstallation: boolean;
  canReset: boolean;
}

export interface RequiredPermissionsPort {
  status(): Promise<RequiredPermissionsStatus>;
  context(): Promise<RequiredPermissionsContext>;
  request(permission: SystemPermission): Promise<RequiredPermissionsStatus>;
  reset(permission: SystemPermission): Promise<RequiredPermissionsStatus>;
  restart(): Promise<void>;
}

export interface RequiredPermissionsSnapshot {
  status: RequiredPermissionsStatus | null;
  error: string | null;
}

export interface PermissionsPollingScheduler {
  schedule(callback: () => void, delayMs: number): unknown;
  cancel(handle: unknown): void;
}

export interface RequiredPermissionsRuntime {
  subscribe(
    listener: (snapshot: RequiredPermissionsSnapshot) => void,
  ): () => void;
  context(): Promise<RequiredPermissionsContext>;
  request(permission: SystemPermission): Promise<RequiredPermissionsStatus>;
  reset(permission: SystemPermission): Promise<RequiredPermissionsStatus>;
  restart(): Promise<void>;
  refresh(): Promise<RequiredPermissionsStatus>;
}

const browserScheduler: PermissionsPollingScheduler = {
  schedule: (callback, delayMs) => globalThis.setTimeout(callback, delayMs),
  cancel: (handle) => globalThis.clearTimeout(handle as number),
};

export function createRequiredPermissionsRuntime(
  port: RequiredPermissionsPort,
  scheduler: PermissionsPollingScheduler = browserScheduler,
): RequiredPermissionsRuntime {
  const listeners = new Set<
    (snapshot: RequiredPermissionsSnapshot) => void
  >();
  let snapshot: RequiredPermissionsSnapshot = { status: null, error: null };
  let pollHandle: unknown;
  let operationVersion = 0;
  let pendingAction: Promise<RequiredPermissionsStatus> | null = null;
  let pendingRefresh: Promise<RequiredPermissionsStatus> | null = null;

  function publish(next: RequiredPermissionsSnapshot) {
    snapshot = next;
    listeners.forEach((listener) => listener(snapshot));
  }

  function cancelPoll() {
    if (pollHandle === undefined) return;
    scheduler.cancel(pollHandle);
    pollHandle = undefined;
  }

  function schedulePoll(delayMs: number) {
    cancelPoll();
    if (listeners.size === 0) return;
    pollHandle = scheduler.schedule(() => {
      pollHandle = undefined;
      void poll();
    }, delayMs);
  }

  async function poll() {
    const version = operationVersion;
    try {
      const status = await port.status();
      if (version !== operationVersion || listeners.size === 0) return;
      publish({ status, error: null });
      if (status.screenRecording && status.accessibility) cancelPoll();
      else schedulePoll(750);
    } catch (cause) {
      if (version !== operationVersion || listeners.size === 0) return;
      publish({ status: snapshot.status, error: errorMessage(cause) });
      schedulePoll(1500);
    }
  }

  async function runOperation(operation: () => Promise<RequiredPermissionsStatus>) {
    operationVersion += 1;
    cancelPoll();
    const version = operationVersion;
    try {
      const status = await operation();
      if (version !== operationVersion || listeners.size === 0) return status;
      publish({ status, error: null });
      if (status.screenRecording && status.accessibility) cancelPoll();
      else schedulePoll(750);
      return status;
    } catch (cause) {
      if (version !== operationVersion || listeners.size === 0) throw cause;
      publish({ status: snapshot.status, error: errorMessage(cause) });
      schedulePoll(1500);
      throw cause;
    }
  }

  function refresh() {
    if (pendingAction) return pendingAction;
    if (pendingRefresh) return pendingRefresh;
    pendingRefresh = runOperation(() => port.status()).finally(() => {
      pendingRefresh = null;
    });
    return pendingRefresh;
  }

  function mutate(operation: () => Promise<RequiredPermissionsStatus>) {
    if (pendingAction) return pendingAction;
    pendingAction = runOperation(operation).finally(() => {
      pendingAction = null;
    });
    return pendingAction;
  }

  return {
    subscribe(listener) {
      listeners.add(listener);
      listener(snapshot);
      if (listeners.size === 1) void poll();
      return () => {
        listeners.delete(listener);
        if (listeners.size === 0) {
          operationVersion += 1;
          cancelPoll();
        }
      };
    },
    context: () => port.context(),
    request: (permission) => mutate(() => port.request(permission)),
    reset: (permission) => mutate(() => port.reset(permission)),
    restart: () => port.restart(),
    refresh,
  };
}

export function areRequiredPermissionsGranted(
  status: RequiredPermissionsStatus,
) {
  return status.screenRecording;
}

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}
