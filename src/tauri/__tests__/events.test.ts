import { describe, expect, it, vi } from 'vitest';

const listen = vi.fn();

vi.mock('@tauri-apps/api/event', () => ({ listen }));

describe('event adapter', () => {
  it('subscribes through the Tauri event seam', async () => {
    listen.mockResolvedValueOnce(() => undefined);
    const handler = vi.fn();
    const { listenTauriEvent } = await import('../events');

    await listenTauriEvent('capture-result-payload-ready', handler);

    expect(listen).toHaveBeenCalledWith('capture-result-payload-ready', handler);
  });
});
