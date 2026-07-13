import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CaptureLaunch } from '../../domain/capture';

const { listen } = vi.hoisted(() => ({
  listen: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({ listen }));

import {
  captureWorkspaceEvents,
  persistentStateEvents,
  resultWindowEvents,
} from './appEvents';

type Listener = (event: { payload: unknown }) => void;

describe('Tauri app event adapter', () => {
  const listeners = new Map<string, Listener>();
  const cleanup = vi.fn();

  beforeEach(() => {
    listeners.clear();
    cleanup.mockReset();
    listen.mockReset();
    listen.mockImplementation(
      async (eventName: string, listener: Listener) => {
        listeners.set(eventName, listener);
        return cleanup;
      },
    );
  });

  it('subscribes to result payload readiness with a safe request ID', async () => {
    const handler = vi.fn();

    const unsubscribe = await resultWindowEvents.subscribeResultPayloadReady(
      handler,
    );
    listeners.get('capture-result-payload-ready')?.({
      payload: { requestId: '9007199254740992' },
    });
    unsubscribe();

    expect(handler).toHaveBeenCalledWith('9007199254740992');
    expect(cleanup).toHaveBeenCalledOnce();
  });

  it.each([undefined, null, {}, { requestId: 42 }])(
    'ignores malformed result payload readiness event %j',
    async (payload) => {
      const handler = vi.fn();

      await resultWindowEvents.subscribeResultPayloadReady(handler);
      listeners.get('capture-result-payload-ready')?.({ payload });

      expect(handler).not.toHaveBeenCalled();
    },
  );

  it.each([
    ['capture cancel', 'capture-cancel-requested', 'subscribeCaptureCancel'],
    ['capture copy', 'capture-copy-requested', 'subscribeCaptureCopy'],
    ['capture save', 'capture-save-requested', 'subscribeCaptureSave'],
    ['capture undo', 'capture-undo-requested', 'subscribeCaptureUndo'],
    ['capture redo', 'capture-redo-requested', 'subscribeCaptureRedo'],
  ] as const)('subscribes to %s requests with portable callbacks', async (
    _workflow,
    eventName,
    method,
  ) => {
    const handler = vi.fn();

    const unsubscribe = await captureWorkspaceEvents[method](handler);
    listeners.get(eventName)?.({ payload: undefined });
    unsubscribe();

    expect(handler).toHaveBeenCalledWith();
    expect(cleanup).toHaveBeenCalledOnce();
  });

  it.each([
    ['settings', 'settings-changed', 'subscribeSettingsChanged'],
    ['hotkeys', 'hotkeys-changed', 'subscribeHotkeysChanged'],
    ['providers', 'providers-changed', 'subscribeProvidersChanged'],
    ['history', 'history-changed', 'subscribeHistoryChanged'],
  ] as const)('subscribes to %s state changes', async (
    _state,
    eventName,
    method,
  ) => {
    const handler = vi.fn();

    const unsubscribe = await persistentStateEvents[method](handler);
    listeners.get(eventName)?.({ payload: undefined });
    unsubscribe();

    expect(handler).toHaveBeenCalledWith();
    expect(cleanup).toHaveBeenCalledOnce();
  });

  it('normalizes a valid string hotkey payload into a capture launch', async () => {
    const handler = vi.fn((_launch: CaptureLaunch): void => undefined);

    const unsubscribe = await captureWorkspaceEvents.subscribeHotkeyTriggered(
      handler,
    );
    listeners.get('hotkey-triggered')?.({ payload: 'screenshot-copy' });
    unsubscribe();

    expect(handler).toHaveBeenCalledWith({ mode: 'screenshot-copy' });
    expect(cleanup).toHaveBeenCalledOnce();
  });

  it('accepts a valid mode-only object hotkey payload', async () => {
    const handler = vi.fn((_launch: CaptureLaunch): void => undefined);

    await captureWorkspaceEvents.subscribeHotkeyTriggered(handler);
    listeners.get('hotkey-triggered')?.({
      payload: { mode: 'silent-screenshot-ocr' },
    });

    expect(handler).toHaveBeenCalledWith({ mode: 'silent-screenshot-ocr' });
  });

  it('accepts a valid string session ID in an object hotkey payload', async () => {
    const handler = vi.fn((_launch: CaptureLaunch): void => undefined);

    await captureWorkspaceEvents.subscribeHotkeyTriggered(handler);
    listeners.get('hotkey-triggered')?.({
      payload: { mode: 'screenshot-ocr', sessionId: 'capture-1' },
    });

    expect(handler).toHaveBeenCalledWith({
      mode: 'screenshot-ocr',
      sessionId: 'capture-1',
    });
  });

  it.each([42, null, { id: 'capture-1' }])(
    'ignores an object hotkey payload with malformed session ID %j',
    async (sessionId) => {
      const handler = vi.fn((_launch: CaptureLaunch): void => undefined);

      await captureWorkspaceEvents.subscribeHotkeyTriggered(handler);
      listeners.get('hotkey-triggered')?.({
        payload: { mode: 'screenshot-ocr', sessionId },
      });

      expect(handler).not.toHaveBeenCalled();
    },
  );

  it('ignores a null hotkey payload', async () => {
    const handler = vi.fn((_launch: CaptureLaunch): void => undefined);

    await captureWorkspaceEvents.subscribeHotkeyTriggered(handler);
    listeners.get('hotkey-triggered')?.({ payload: null });

    expect(handler).not.toHaveBeenCalled();
  });

  it('ignores invalid hotkey payloads', async () => {
    const handler = vi.fn((_launch: CaptureLaunch): void => undefined);

    await captureWorkspaceEvents.subscribeHotkeyTriggered(handler);
    listeners.get('hotkey-triggered')?.({
      payload: { mode: 'translation', sessionId: 'capture-1' },
    });

    expect(handler).not.toHaveBeenCalled();
  });
});
