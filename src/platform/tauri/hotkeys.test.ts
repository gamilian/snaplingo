import { describe, expect, it, vi } from 'vitest';

const { invoke, listen } = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen }));

describe('Tauri hotkeys command adapter', () => {
  it('scopes recording commands and delivers native key recordings', async () => {
    const { beginHotkeyRecording, endHotkeyRecording, subscribeRecordedHotkey } =
      await import('./hotkeys');
    const handler = vi.fn();
    const unsubscribe = vi.fn();
    listen.mockResolvedValueOnce(unsubscribe);
    expect(await subscribeRecordedHotkey(handler)).toBe(unsubscribe);
    expect(listen).toHaveBeenCalledWith('hotkey-recorded', expect.any(Function));
    listen.mock.calls[0][1]({ payload: { recordingId: 'session', hotkey: 'F1' } });
    expect(handler).toHaveBeenCalledWith({ recordingId: 'session', hotkey: 'F1' });
    await beginHotkeyRecording('session');
    await endHotkeyRecording('session');
    expect(invoke).toHaveBeenCalledWith('begin_hotkey_recording', { recordingId: 'session' });
    expect(invoke).toHaveBeenCalledWith('end_hotkey_recording', { recordingId: 'session' });
  });
  it('loads the hotkey snapshot', async () => {
    const { getHotkeySnapshot } = await import('./hotkeys');
    invoke.mockResolvedValueOnce({ screenshot: {}, translation: {}, ocr: {} });
    await getHotkeySnapshot();
    expect(invoke).toHaveBeenCalledWith('get_hotkey_snapshot');
  });

  it('updates a hotkey through the unified command', async () => {
    const { updateHotkey } = await import('./hotkeys');
    invoke.mockResolvedValueOnce({ snapshot: {}, accelerator: 'Alt+KeyD' });
    await updateHotkey({ category: 'translation', action: 'selection-translate', hotkey: '⌥D' });
    expect(invoke).toHaveBeenCalledWith('update_hotkey', {
      category: 'translation', action: 'selection-translate', hotkey: '⌥D',
    });
  });
});
