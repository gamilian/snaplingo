import { beforeEach, describe, expect, it, vi } from 'vitest';

const { emit, invoke } = vi.hoisted(() => ({
  emit: vi.fn(),
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ emit }));

import { captureWindow } from './captureWindow';

describe('Tauri capture window adapter', () => {
  beforeEach(() => {
    invoke.mockReset().mockResolvedValue(undefined);
    emit.mockReset().mockResolvedValue(undefined);
  });

  it('signals native readiness after capture event subscription is established', async () => {
    await captureWindow.signalReady();

    expect(emit).toHaveBeenCalledWith('capture-window-ready');
  });

  it('reveals the capture window through the current native workflow', async () => {
    await captureWindow.reveal('frozen-session');

    expect(invoke).toHaveBeenCalledWith('reveal_capture_window', {
      sessionId: 'frozen-session',
    });
  });

  it('prepares the capture window before session-aware surface reveal', async () => {
    await captureWindow.prepareForReveal('refreshed-session');

    expect(invoke).toHaveBeenCalledWith('prepare_capture_window_for_reveal', {
      sessionId: 'refreshed-session',
    });
  });

  it('hides the capture window through the current native workflow', async () => {
    await captureWindow.hide();

    expect(invoke).toHaveBeenCalledWith('hide_capture_window');
  });
});
