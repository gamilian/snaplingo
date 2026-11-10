// @vitest-environment happy-dom

import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it, vi } from 'vitest';
import {
  createRequiredPermissionsRuntime,
  type RequiredPermissionsRuntime,
} from '../application/permissions/runtime';
import { RequiredPermissionsGate } from './RequiredPermissionsGate';

const granted = { screenRecording: true, accessibility: true };
const missing = { screenRecording: false, accessibility: false };
const context = { platform: 'macos', appPath: '/Applications/SnapLingo.app', needsInstallation: false, canReset: true };
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe('RequiredPermissionsGate', () => {
  it('does not treat a failed permission check as a missing grant', async () => {
    const runtime = createRequiredPermissionsRuntime({
      status: vi.fn().mockRejectedValue(new Error('status unavailable')),
      request: vi.fn(), context: vi.fn(async () => context), reset: vi.fn(), restart: vi.fn(),
    });
    const view = await renderGate(runtime);
    try {
      expect(view.container.querySelector('[role="dialog"]')).toBeNull();
      expect(view.container.textContent).toContain('ready');
    } finally {
      await view.unmount();
    }
  });

  it('waits for a fresh check when reopened after permission was granted elsewhere', async () => {
    let finishCheck: (status: typeof granted) => void = () => undefined;
    const status = vi.fn().mockResolvedValueOnce(missing)
      .mockImplementation(() => new Promise<typeof granted>((resolve) => { finishCheck = resolve; }));
    const request = vi.fn();
    const runtime = createRequiredPermissionsRuntime({
      status, request, context: vi.fn(async () => context), reset: vi.fn(), restart: vi.fn(),
    });
    const previousView = await renderGate(runtime);
    await previousView.unmount();
    const view = await renderGate(runtime);
    try {
      expect(view.container.querySelector('[role="dialog"]')).toBeNull();
      await act(async () => finishCheck(granted));
      expect(view.container.querySelector('[role="dialog"]')).toBeNull();
      expect(request).not.toHaveBeenCalled();
    } finally {
      await view.unmount();
    }
  });

  it('does not use an earlier missing status after a refresh fails', async () => {
    const runtime = createRequiredPermissionsRuntime({
      status: vi.fn().mockResolvedValueOnce(missing).mockRejectedValue(new Error('status unavailable')),
      request: vi.fn(), context: vi.fn(async () => context), reset: vi.fn(), restart: vi.fn(),
    });
    const view = await renderGate(runtime);
    try {
      await act(async () => window.dispatchEvent(new Event('focus')));
      expect(view.container.querySelector('[role="dialog"]')).toBeNull();
    } finally {
      await view.unmount();
    }
  });

  it('explains that the current app lacks permission and how old grants can differ', async () => {
    const view = await renderGate(createRuntime(missing));
    try {
      expect(view.container.textContent).toContain('当前运行的 SnapLingo 尚未获得屏幕录制权限');
      expect(view.container.textContent).toContain('旧版本');
      expect(view.container.textContent).toContain('无法判断');
    } finally {
      await view.unmount();
    }
  });

  it('detects an existing grant on return without another request or relaunch', async () => {
    const status = vi.fn()
      .mockResolvedValueOnce(missing)
      .mockResolvedValue(granted);
    const request = vi.fn();
    const runtime = createRequiredPermissionsRuntime({
      status, request, context: vi.fn(async () => context), reset: vi.fn(), restart: vi.fn(),
    });
    const view = await renderGate(runtime);

    try {
      await act(async () => window.dispatchEvent(new Event('focus')));

      expect(status).toHaveBeenCalledTimes(2);
      expect(request).not.toHaveBeenCalled();
      expect(view.container.querySelector('[role="dialog"]')).toBeNull();
      expect(view.container.textContent).toContain('ready');
    } finally {
      await view.unmount();
    }
  });

  it('checks status without requesting permissions again on initial mount', async () => {
    const runtime = createRuntime(granted);
    const view = await renderGate(runtime);

    expect(runtime.subscribe).toHaveBeenCalledTimes(1);
    expect(runtime.request).not.toHaveBeenCalled();
    expect(view.container.textContent).toContain('ready');
    expect(view.container.querySelector('[role="dialog"]')).toBeNull();

    await view.unmount();
  });

  it('rechecks before authorizing and skips the request when the grant is already effective', async () => {
    const request = vi.fn();
    const runtime = createRequiredPermissionsRuntime({
      status: vi.fn().mockResolvedValueOnce(missing).mockResolvedValue(granted),
      request, context: vi.fn(async () => context), reset: vi.fn(), restart: vi.fn(),
    });
    const view = await renderGate(runtime);
    try {
      const button = [...view.container.querySelectorAll('button')].find((item) => item.textContent === '去授权');
      expect(button).toBeDefined();
      await act(async () => button?.click());
      expect(request).not.toHaveBeenCalled();
      expect(view.container.querySelector('[role="dialog"]')).toBeNull();
    } finally {
      await view.unmount();
    }
  });

  it('opens the selected permission after the user explicitly continues', async () => {
    const runtime = createRuntime(missing, granted);
    const view = await renderGate(runtime);
    const retryButton = [...view.container.querySelectorAll('button')].find(
      (button) => button.textContent === '去授权',
    );

    expect(view.container.textContent).toContain('ready');
    expect(view.container.querySelector('[role="dialog"]')).not.toBeNull();
    expect(view.container.textContent).not.toContain('辅助功能');
    expect(view.container.querySelectorAll('button')).toHaveLength(2);

    await act(async () => retryButton?.click());

    expect(runtime.request).toHaveBeenCalledExactlyOnceWith('screenRecording');
    expect(view.container.textContent).toContain('ready');
    expect(view.container.querySelector('[role="dialog"]')).toBeNull();

    await view.unmount();
  });

  it('does not block screenshot features when only optional Accessibility is missing', async () => {
    const runtime = createRuntime({
      screenRecording: true,
      accessibility: false,
    });
    const view = await renderGate(runtime);

    expect(view.container.textContent).toContain('ready');
    expect(view.container.querySelector('[role="dialog"]')).toBeNull();
    expect(runtime.request).not.toHaveBeenCalled();

    await view.unmount();
  });

  it('lets users continue without granting unrelated permissions', async () => {
    const runtime = createRuntime(missing);
    const view = await renderGate(runtime);
    const skip = [...view.container.querySelectorAll('button')].find(
      (button) => button.textContent === '稍后',
    );
    expect(skip).toBeDefined();
    await act(async () => skip?.click());
    expect(view.container.querySelector('[role="dialog"]')).toBeNull();
    expect(view.container.textContent).toContain('ready');
    expect(runtime.request).not.toHaveBeenCalled();
    await view.unmount();
  });

  it('refreshes permission status when the window regains focus', async () => {
    const runtime = createRuntime(granted);
    const view = await renderGate(runtime);

    await act(async () => {
      window.dispatchEvent(new Event('focus'));
    });

    expect(runtime.refresh).toHaveBeenCalledOnce();
    await view.unmount();
  });

  it('offers recovery in settings only after an unsuccessful grant attempt', async () => {
    const runtime = createRuntime(missing);
    const onOpenSettings = vi.fn();
    const view = await renderGate(runtime, onOpenSettings);
    const button = (label: string) => [...view.container.querySelectorAll('button')].find((item) => item.textContent === label);
    expect(button('授权遇到问题？')).toBeUndefined();
    await act(async () => button('去授权')?.click());
    expect(button('授权遇到问题？')).toBeDefined();
    expect(view.container.textContent).not.toContain(context.appPath);
    await act(async () => button('授权遇到问题？')?.click());
    expect(onOpenSettings).toHaveBeenCalledOnce();
    expect(view.container.querySelector('[role="dialog"]')).toBeNull();
    expect(runtime.reset).not.toHaveBeenCalled();
    expect(runtime.restart).not.toHaveBeenCalled();
    await view.unmount();
  });

  it('does not request or repair permissions for a temporary disk-image copy', async () => {
    const runtime = createRuntime(missing);
    vi.mocked(runtime.context).mockResolvedValue({ ...context, needsInstallation: true, canReset: false });
    const view = await renderGate(runtime);
    expect(view.container.textContent).toContain('先安装 SnapLingo');
    expect(view.container.textContent).toContain('应用程序');
    expect(view.container.querySelectorAll('button')).toHaveLength(1);
    expect(view.container.textContent).not.toContain('去授权');
    expect(runtime.request).not.toHaveBeenCalled();
    expect(runtime.reset).not.toHaveBeenCalled();
    await view.unmount();
  });
});

function createRuntime(
  initialStatus: typeof granted,
  requestedStatus: typeof granted = initialStatus,
) {
  let listener:
    | Parameters<RequiredPermissionsRuntime['subscribe']>[0]
    | undefined;
  const runtime: RequiredPermissionsRuntime = {
    subscribe: vi.fn((nextListener) => {
      listener = nextListener;
      nextListener({ status: initialStatus, error: null });
      return vi.fn();
    }),
    context: vi.fn(async () => context),
    reset: vi.fn(async () => initialStatus),
    restart: vi.fn(async () => undefined),
    request: vi.fn(async () => {
      listener?.({ status: requestedStatus, error: null });
      return requestedStatus;
    }),
    refresh: vi.fn(async () => initialStatus),
  };
  return runtime;
}

async function renderGate(runtime: RequiredPermissionsRuntime, onOpenSettings = vi.fn()) {
  const container = document.createElement('div');
  const root = createRoot(container);

  await act(async () => {
    root.render(
      <RequiredPermissionsGate runtime={runtime} onOpenSettings={onOpenSettings}>
        <div>ready</div>
      </RequiredPermissionsGate>,
    );
  });

  return {
    container,
    unmount: async () => act(async () => root.unmount()),
  };
}
