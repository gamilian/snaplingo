// @vitest-environment happy-dom

import { act } from 'react-dom/test-utils';
import { createRoot } from 'react-dom/client';
import { describe, expect, it, vi } from 'vitest';
import type { RequiredPermissionsRuntime } from '../application/permissions/runtime';
import { RequiredPermissionsGate } from './RequiredPermissionsGate';

const granted = { screenRecording: true, accessibility: true };
const missing = { screenRecording: false, accessibility: false };

describe('RequiredPermissionsGate', () => {
  it('checks status without requesting permissions again on initial mount', async () => {
    const runtime: RequiredPermissionsRuntime = {
      status: vi.fn().mockResolvedValue(granted),
      request: vi.fn().mockResolvedValue(granted),
    };
    const view = await renderGate(runtime);

    expect(runtime.status).toHaveBeenCalledTimes(1);
    expect(runtime.request).not.toHaveBeenCalled();
    expect(view.container.textContent).toContain('ready');

    await view.unmount();
  });

  it('opens the next missing permission after the user explicitly continues', async () => {
    const runtime: RequiredPermissionsRuntime = {
      status: vi.fn().mockResolvedValue(missing),
      request: vi.fn().mockResolvedValue(granted),
    };
    const view = await renderGate(runtime);
    const retryButton = [...view.container.querySelectorAll('button')].find(
      (button) => button.textContent === '打开屏幕录制设置',
    );

    expect(view.container.textContent).toContain('ready');
    expect(view.container.querySelector('[role="dialog"]')).not.toBeNull();

    await act(async () => retryButton?.click());

    expect(runtime.request).toHaveBeenCalledTimes(1);
    expect(view.container.textContent).toContain('ready');

    await view.unmount();
  });

  it('offers Accessibility after Screen Recording is granted', async () => {
    const runtime: RequiredPermissionsRuntime = {
      status: vi.fn().mockResolvedValue({
        screenRecording: true,
        accessibility: false,
      }),
      request: vi.fn().mockResolvedValue(granted),
    };
    const view = await renderGate(runtime);

    expect(view.container.textContent).toContain('打开辅助功能设置');
    expect(runtime.request).not.toHaveBeenCalled();

    await view.unmount();
  });
});

async function renderGate(runtime: RequiredPermissionsRuntime) {
  const container = document.createElement('div');
  const root = createRoot(container);

  await act(async () => {
    root.render(
      <RequiredPermissionsGate runtime={runtime}>
        <div>ready</div>
      </RequiredPermissionsGate>,
    );
  });

  return {
    container,
    unmount: async () => act(async () => root.unmount()),
  };
}
