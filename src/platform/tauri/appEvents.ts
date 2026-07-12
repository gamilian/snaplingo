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
const HOTKEY_TRIGGERED_EVENT = 'hotkey-triggered';

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
  async subscribeHotkeyTriggered(handler) {
    const unlisten = await listen<unknown>(HOTKEY_TRIGGERED_EVENT, (event) => {
      const launch = parseCaptureLaunch(event.payload);
      if (!launch) return;

      void handler(launch);
    });

    return () => unlisten();
  },
};
