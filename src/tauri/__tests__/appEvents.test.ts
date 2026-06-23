import { listen } from '@tauri-apps/api/event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

describe('appEvents', () => {
  beforeEach(() => {
    vi.mocked(listen).mockReset();
  });

  it('subscribes all main-window events behind one interface', async () => {
    const handlers = new Map<string, (event: { payload: unknown }) => void>();
    const unlisteners = new Map<string, () => void>();

    vi.mocked(listen).mockImplementation(async (eventName, callback) => {
      const name = String(eventName);
      const unlisten: () => void = vi.fn();
      handlers.set(name, callback as (event: { payload: unknown }) => void);
      unlisteners.set(name, unlisten);
      return unlisten;
    });

    const { subscribeMainWindowEvents } = await import('../appEvents');
    const events = {
      onInputTranslation: vi.fn(),
      onInputOcr: vi.fn(),
      onShowOcrWindow: vi.fn(),
      onStartFileOcr: vi.fn(),
      onShowTranslationWindow: vi.fn(),
      onScreenshotCaptured: vi.fn(),
      onScreenshotError: vi.fn(),
    };

    const dispose = await subscribeMainWindowEvents(events);

    expect([...handlers.keys()].sort()).toEqual([
      'input-ocr',
      'input-translation',
      'screenshot-captured',
      'screenshot-error',
      'show-ocr-window',
      'show-translation-window',
      'start-file-ocr',
    ]);

    handlers.get('input-translation')?.({
      payload: { text: 'hello', autoTranslate: true },
    });
    handlers.get('start-file-ocr')?.({ payload: null });

    expect(events.onInputTranslation).toHaveBeenCalledWith({
      text: 'hello',
      autoTranslate: true,
    });
    expect(events.onStartFileOcr).toHaveBeenCalled();

    dispose();
    for (const unlisten of unlisteners.values()) {
      expect(unlisten).toHaveBeenCalled();
    }
  });

  it('ignores invalid input translation payloads', async () => {
    let inputTranslationHandler:
      | ((event: { payload: unknown }) => void)
      | undefined;

    vi.mocked(listen).mockImplementation(async (eventName, callback) => {
      if (eventName === 'input-translation') {
        inputTranslationHandler = callback as typeof inputTranslationHandler;
      }
      return vi.fn() as () => void;
    });

    const { subscribeMainWindowEvents } = await import('../appEvents');
    const callback = vi.fn();
    await subscribeMainWindowEvents({ onInputTranslation: callback });

    inputTranslationHandler?.({ payload: { autoTranslate: true } });

    expect(callback).not.toHaveBeenCalled();
  });
});
