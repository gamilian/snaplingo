import { describe, expect, it, vi } from 'vitest';
import { createAppUpdatesRuntime, type AppUpdatesPort, type AppUpdateStatus } from './runtime';

const available: AppUpdateStatus = {
  currentVersion: '0.1.4', latestVersion: '0.1.5', available: true, downloadSize: 25_000_000,
  notes: '更新说明', publishedAt: null, platform: 'macos',
};

function createPort(overrides: Partial<AppUpdatesPort> = {}): AppUpdatesPort {
  return { check: vi.fn(async () => available), downloadAndOpen: vi.fn(async () => '/cache/update.dmg'),
    openReleasePage: vi.fn(async () => undefined), ...overrides };
}

describe('software updates runtime', () => {
  it('checks automatically once across settings remounts and never downloads implicitly', async () => {
    const port = createPort();
    const runtime = createAppUpdatesRuntime(port);
    await runtime.ensureChecked();
    await runtime.ensureChecked();
    expect(port.check).toHaveBeenCalledOnce();
    expect(port.downloadAndOpen).not.toHaveBeenCalled();
    await runtime.check();
    expect(port.check).toHaveBeenCalledTimes(2);
  });

  it('reports network errors and allows an explicit retry', async () => {
    const port = createPort({ check: vi.fn().mockRejectedValueOnce(new Error('network unavailable')).mockResolvedValue(available) });
    const runtime = createAppUpdatesRuntime(port);
    const listener = vi.fn();
    runtime.subscribe(listener);
    await expect(runtime.ensureChecked()).rejects.toThrow('network unavailable');
    expect(listener).toHaveBeenLastCalledWith(expect.objectContaining({ phase: 'idle', error: 'network unavailable' }));
    await runtime.check();
    expect(listener).toHaveBeenLastCalledWith(expect.objectContaining({ status: available, error: null }));
  });

  it('prevents duplicate downloads and checks while a verified installer is being fetched', async () => {
    let complete: (path: string) => void = () => undefined;
    const port = createPort({ downloadAndOpen: vi.fn(() => new Promise<string>((resolve) => { complete = resolve; })) });
    const runtime = createAppUpdatesRuntime(port);
    const listener = vi.fn();
    runtime.subscribe(listener);
    await runtime.check();
    const download = runtime.downloadAndOpen();
    const duplicate = runtime.downloadAndOpen();
    const check = runtime.check();
    await Promise.resolve();
    expect(port.downloadAndOpen).toHaveBeenCalledOnce();
    expect(port.check).toHaveBeenCalledOnce();
    complete('/cache/update.dmg');
    await Promise.all([download, duplicate, check]);
    expect(listener).toHaveBeenLastCalledWith(expect.objectContaining({ phase: 'idle', downloadedPath: '/cache/update.dmg' }));
  });

  it('does not download when up to date or when the matching installer is missing', async () => {
    for (const status of [{ ...available, available: false }, { ...available, downloadSize: null }]) {
      const port = createPort({ check: vi.fn(async () => status) });
      const runtime = createAppUpdatesRuntime(port);
      await runtime.check();
      await expect(runtime.downloadAndOpen()).rejects.toThrow('请先检查更新');
      expect(port.downloadAndOpen).not.toHaveBeenCalled();
    }
  });

  it('keeps a failed checksum visible and lets users download again', async () => {
    const port = createPort({ downloadAndOpen: vi.fn().mockRejectedValueOnce(new Error('安装包校验失败')).mockResolvedValue('/cache/verified.dmg') });
    const runtime = createAppUpdatesRuntime(port);
    const listener = vi.fn();
    runtime.subscribe(listener);
    await runtime.check();
    await expect(runtime.downloadAndOpen()).rejects.toThrow('安装包校验失败');
    expect(listener).toHaveBeenLastCalledWith(expect.objectContaining({ error: '安装包校验失败', downloadedPath: null, phase: 'idle' }));
    await runtime.downloadAndOpen();
    expect(listener).toHaveBeenLastCalledWith(expect.objectContaining({ error: null, downloadedPath: '/cache/verified.dmg' }));
  });
});
