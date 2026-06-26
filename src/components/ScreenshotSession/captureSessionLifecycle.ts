import {
  cancelCaptureSession,
  hideCaptureWindow,
} from '../../tauri/captureSession';

export type CaptureInactiveHandler = () => void | Promise<void>;
export interface CaptureInactiveWindow {
  hide: () => Promise<void>;
}

export interface CaptureLifecycleClient {
  cancelCaptureSession: (sessionId: string) => Promise<void>;
}

const tauriCaptureLifecycleClient: CaptureLifecycleClient = {
  cancelCaptureSession,
};

interface CloseInactiveCaptureSessionOptions {
  onInactive?: CaptureInactiveHandler;
  resetSessionState: () => void;
}

interface FinishCaptureSessionOptions
  extends CloseInactiveCaptureSessionOptions {
  client?: CaptureLifecycleClient;
  sessionId: string;
}

export async function closeInactiveCaptureSession({
  onInactive,
  resetSessionState,
}: CloseInactiveCaptureSessionOptions) {
  if (onInactive) {
    await onInactive();
    return;
  }

  resetSessionState();
}

export async function finishCaptureSession({
  client = tauriCaptureLifecycleClient,
  sessionId,
  onInactive,
  resetSessionState,
}: FinishCaptureSessionOptions) {
  await closeInactiveCaptureSession({ onInactive, resetSessionState });
  await client.cancelCaptureSession(sessionId);
}

export interface CaptureWindowHideClient {
  hideCaptureWindow: () => Promise<void>;
}

const tauriCaptureWindowHideClient: CaptureWindowHideClient = {
  hideCaptureWindow,
};

export async function hideInactiveCaptureWindow(
  _window: CaptureInactiveWindow,
  client: CaptureWindowHideClient = tauriCaptureWindowHideClient,
) {
  await client.hideCaptureWindow();
}
