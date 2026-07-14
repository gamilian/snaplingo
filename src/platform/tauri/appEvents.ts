import { listen } from '@tauri-apps/api/event';
import type { CaptureWorkspaceEventsPort } from '../../application/capture-workspace/ports';
import type { ResultWindowEventsPort } from '../../application/result-window/ports';
import {
  CAPTURE_MODES,
  type CaptureLaunch,
  type CaptureMode,
} from '../../domain/capture';

const RESULT_PAYLOAD_READY_EVENT = 'capture-result-payload-ready';
const CAPTURE_CANCEL_EVENT = 'capture-cancel-requested';
const CAPTURE_COPY_EVENT = 'capture-copy-requested';
const CAPTURE_SAVE_EVENT = 'capture-save-requested';
const CAPTURE_UNDO_EVENT = 'capture-undo-requested';
const CAPTURE_REDO_EVENT = 'capture-redo-requested';
const HOTKEY_TRIGGERED_EVENT = 'hotkey-triggered';
const SETTINGS_CHANGED_EVENT = 'settings-changed';
const HOTKEYS_CHANGED_EVENT = 'hotkeys-changed';
const PROVIDERS_CHANGED_EVENT = 'providers-changed';
const HISTORY_CHANGED_EVENT = 'history-changed';
const FAVORITES_CHANGED_EVENT = 'favorites-changed';
const SCREENSHOT_FAVORITES_CHANGED_EVENT = 'screenshot-favorites-changed';

function isCaptureMode(value: unknown): value is CaptureMode {
  return (
    typeof value === 'string' && CAPTURE_MODES.includes(value as CaptureMode)
  );
}

function parseCaptureLaunch(payload: unknown): CaptureLaunch | null {
  if (isCaptureMode(payload)) {
    return { mode: payload };
  }

  if (!payload || typeof payload !== 'object') {
    return null;
  }

  const candidate = payload as { mode?: unknown; sessionId?: unknown };
  if (!isCaptureMode(candidate.mode)) {
    return null;
  }

  const hasSessionId = Object.prototype.hasOwnProperty.call(
    candidate,
    'sessionId',
  );
  if (!hasSessionId) {
    return { mode: candidate.mode };
  }
  if (typeof candidate.sessionId !== 'string') {
    return null;
  }

  return {
    mode: candidate.mode,
    sessionId: candidate.sessionId,
  };
}

async function subscribeToSignal(
  eventName: string,
  handler: () => void | Promise<void>,
) {
  const unlisten = await listen<void>(eventName, () => {
    void handler();
  });

  return () => unlisten();
}

function parseResultPayloadReady(payload: unknown) {
  if (!payload || typeof payload !== 'object') return null;

  const requestId = (payload as { requestId?: unknown }).requestId;
  return typeof requestId === 'string' ? requestId : null;
}

export const resultWindowEvents: ResultWindowEventsPort = {
  async subscribeResultPayloadReady(handler) {
    const unlisten = await listen<unknown>(
      RESULT_PAYLOAD_READY_EVENT,
      (event) => {
        const requestId = parseResultPayloadReady(event.payload);
        if (!requestId) return;

        void handler(requestId);
      },
    );

    return () => unlisten();
  },
};

export const captureWorkspaceEvents: CaptureWorkspaceEventsPort = {
  subscribeCaptureCancel: (handler) =>
    subscribeToSignal(CAPTURE_CANCEL_EVENT, handler),
  subscribeCaptureCopy: (handler) =>
    subscribeToSignal(CAPTURE_COPY_EVENT, handler),
  subscribeCaptureSave: (handler) =>
    subscribeToSignal(CAPTURE_SAVE_EVENT, handler),
  subscribeCaptureUndo: (handler) =>
    subscribeToSignal(CAPTURE_UNDO_EVENT, handler),
  subscribeCaptureRedo: (handler) =>
    subscribeToSignal(CAPTURE_REDO_EVENT, handler),
  async subscribeHotkeyTriggered(handler) {
    const unlisten = await listen<unknown>(HOTKEY_TRIGGERED_EVENT, (event) => {
      const launch = parseCaptureLaunch(event.payload);
      if (!launch) return;

      void handler(launch);
    });

    return () => unlisten();
  },
};

export const persistentStateEvents = {
  subscribeSettingsChanged: (handler: () => void | Promise<void>) =>
    subscribeToSignal(SETTINGS_CHANGED_EVENT, handler),
  subscribeHotkeysChanged: (handler: () => void | Promise<void>) =>
    subscribeToSignal(HOTKEYS_CHANGED_EVENT, handler),
  subscribeProvidersChanged: (handler: () => void | Promise<void>) =>
    subscribeToSignal(PROVIDERS_CHANGED_EVENT, handler),
  subscribeHistoryChanged: (handler: () => void | Promise<void>) =>
    subscribeToSignal(HISTORY_CHANGED_EVENT, handler),
  subscribeFavoritesChanged: (handler: () => void | Promise<void>) =>
    subscribeToSignal(FAVORITES_CHANGED_EVENT, handler),
  subscribeScreenshotFavoritesChanged: (handler: () => void | Promise<void>) =>
    subscribeToSignal(SCREENSHOT_FAVORITES_CHANGED_EVENT, handler),
};
