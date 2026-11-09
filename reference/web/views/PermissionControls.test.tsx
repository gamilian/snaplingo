// @vitest-environment happy-dom

import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it, vi } from 'vitest';
import {
  createRequiredPermissionsRuntime,
  type RequiredPermissionsPort,
} from '../application/permissions/runtime';
import { PermissionControls, useRequiredPermissions } from './PermissionControls';

const missing = { screenRecording: false, accessibility: true };
const context = { platform: 'macos', appPath: '/Applications/SnapLingo.app', needsInstallation: false, canReset: true };
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe('PermissionControls in settings', () => {
  it('explains missing grants for the current app and possible old-version grants', async () => {
    const view = await renderControls();
    try {
      expect(view.container.textContent).toContain('当前运行的 SnapLingo 尚未获得屏幕录制权限');
      expect(view.container.textContent).toContain('旧版本');
      expect(view.container.textContent).toContain('无法判断');
      expect(view.container.textContent).not.toContain('当前运行的 SnapLingo 尚未获得辅助功能权限');
      expect(view.container.querySelector('[aria-label="授权辅助功能"]')).toBeNull();
    } finally {
      await view.unmount();
    }
  });

  it('does not show authorization prompts while status is unknown', async () => {
    const view = await renderControls({ status: vi.fn(() => new Promise<typeof missing>(() => undefined)) });
    try {
      expect(view.container.textContent).toContain('检测中');
      expect(view.container.textContent).not.toContain('去授权');
      expect(view.container.textContent).not.toContain('待授权');
      expect(view.container.textContent).not.toContain('重新授权');
    } finally {
      await view.unmount();
    }
  });

  it('shows a check failure instead of an authorization prompt', async () => {
    const view = await renderControls({ status: vi.fn().mockRejectedValue(new Error('status unavailable')) });
    try {
      expect(view.container.textContent).toContain('检测失败');
      expect(view.container.textContent).not.toContain('去授权');
      expect(view.container.textContent).not.toContain('待授权');
      expect(view.container.textContent).not.toContain('重新授权');
    } finally {
      await view.unmount();
    }
  });

  it('does not offer authorization or resets for permissions already granted', async () => {
    const view = await renderControls({ status: vi.fn(async () => ({ screenRecording: true, accessibility: true })) });
    try {
      expect(view.container.textContent).not.toContain('去授权');
      expect(view.container.textContent).not.toContain('重新授权');
      expect(view.container.textContent).not.toContain('旧版本');
      expect(view.port.reset).not.toHaveBeenCalled();
    } finally {
      await view.unmount();
    }
  });

  it('requires confirmation before resetting only the selected permission', async () => {
    const view = await renderControls();
    try {
      const details = view.container.querySelector('details')!;
      expect(details.open).toBe(false);
      expect(view.port.request).not.toHaveBeenCalled();
      details.open = true;
      await act(async () => view.button('重新授权屏幕录制').click());
      expect(view.port.reset).not.toHaveBeenCalled();
      await act(async () => view.button('确认重置屏幕录制').click());
      expect(view.port.reset).toHaveBeenCalledExactlyOnceWith('screenRecording');
      expect(view.port.restart).not.toHaveBeenCalled();
    } finally {
      await view.unmount();
    }
  });

  it('refreshes an existing grant without resetting or requesting it again', async () => {
    const status = vi.fn().mockResolvedValueOnce(missing)
      .mockResolvedValue({ screenRecording: true, accessibility: true });
    const view = await renderControls({ status });
    try {
      view.container.querySelector('details')!.open = true;
      await act(async () => view.button('重新检测').click());
      expect(status).toHaveBeenCalledTimes(2);
      expect(view.container.querySelector('[aria-label="授权屏幕录制"]')).toBeNull();
      expect(view.port.request).not.toHaveBeenCalled();
      expect(view.port.reset).not.toHaveBeenCalled();
      expect(view.port.restart).not.toHaveBeenCalled();
    } finally {
      await view.unmount();
    }
  });
});

async function renderControls(overrides: Partial<RequiredPermissionsPort> = {}) {
  const port = {
    status: vi.fn(async () => missing),
    context: vi.fn(async () => context),
    request: vi.fn(async () => missing),
    reset: vi.fn(async () => missing),
    restart: vi.fn(async () => undefined),
    ...overrides,
  };
  const runtime = createRequiredPermissionsRuntime(port);
  function Settings() {
    return <PermissionControls runtime={runtime} snapshot={useRequiredPermissions(runtime)} />;
  }
  const container = document.createElement('div');
  const root = createRoot(container);
  await act(async () => root.render(<Settings />));
  return {
    container,
    port,
    button: (label: string) => {
      const button = [...container.querySelectorAll('button')].find((item) => item.textContent === label);
      if (!button) throw new Error(`Button not found: ${label}`);
      return button;
    },
    unmount: async () => act(async () => root.unmount()),
  };
}
