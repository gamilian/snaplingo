// @vitest-environment happy-dom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it, vi } from 'vitest';
import { createAppUpdatesRuntime, type AppUpdateStatus } from '../../../application/updates/runtime';
import type { SettingsRuntime } from '../../../application/settings/runtime';
import { SettingsRuntimeProvider } from '../runtimeContext';
import { SoftwareUpdateSettings } from './SoftwareUpdateSettings';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const release: AppUpdateStatus = { currentVersion: '0.1.4', latestVersion: '0.1.5', available: true,
  downloadSize: 27_000_000, notes: '修复授权流程', publishedAt: null, platform: 'macos' };

async function render(status = release) {
  const port = { check: vi.fn(async () => status), downloadAndOpen: vi.fn(async () => '/cache/verified.dmg'), openReleasePage: vi.fn(async () => undefined) };
  const updates = createAppUpdatesRuntime(port);
  const container = document.createElement('div');
  const root = createRoot(container);
  await act(async () => root.render(
    <SettingsRuntimeProvider runtime={{ updates } as SettingsRuntime}><SoftwareUpdateSettings /></SettingsRuntimeProvider>,
  ));
  return { port, container, unmount: async () => act(async () => root.unmount()) };
}

describe('software update settings', () => {
  it('finds a release automatically and downloads only after clicking the installer button', async () => {
    const view = await render();
    expect(view.container.textContent).toContain('发现新版本 0.1.5');
    expect(view.port.check).toHaveBeenCalledOnce();
    expect(view.port.downloadAndOpen).not.toHaveBeenCalled();
    const button = [...view.container.querySelectorAll('button')].find((item) => item.textContent?.includes('下载并打开安装包'));
    expect(button?.textContent).toContain('27 MB');
    await act(async () => button?.click());
    expect(view.port.downloadAndOpen).toHaveBeenCalledOnce();
    expect(view.container.textContent).toContain('安装包已下载并打开');
    await view.unmount();
  });

  it('distinguishes a missing installer from being up to date and offers the release page', async () => {
    const view = await render({ ...release, downloadSize: null });
    expect(view.container.textContent).toContain('暂未提供可验证的适配安装包');
    expect(view.container.textContent).not.toContain('当前已是最新版本');
    const button = [...view.container.querySelectorAll('button')].find((item) => item.textContent === '发布页面');
    await act(async () => button?.click());
    expect(view.port.openReleasePage).toHaveBeenCalledOnce();
    await view.unmount();
  });
});
