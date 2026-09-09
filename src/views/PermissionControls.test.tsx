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
