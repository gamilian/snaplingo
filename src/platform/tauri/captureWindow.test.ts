import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

import { captureWindow } from './captureWindow';

describe('Tauri capture window adapter', () => {
  beforeEach(() => {
    invoke.mockReset().mockResolvedValue(undefined);
  });

  it('reveals the capture window through the current native workflow', async () => {
    await captureWindow.reveal();

    expect(invoke).toHaveBeenCalledWith('reveal_capture_window');
  });

  it('prepares the capture window before session-aware surface reveal', async () => {
    await captureWindow.prepareForReveal();

    expect(invoke).toHaveBeenCalledWith('prepare_capture_window_for_reveal');
  });

  it('hides the capture window through the current native workflow', async () => {
    await captureWindow.hide();

    expect(invoke).toHaveBeenCalledWith('hide_capture_window');
  });
});
