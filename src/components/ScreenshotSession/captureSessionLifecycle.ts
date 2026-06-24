import { cancelCaptureSession } from '../../tauri/captureSession';

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

export async function hideInactiveCaptureWindow(window: CaptureInactiveWindow) {
  await window.hide();
}
