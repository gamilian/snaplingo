export type CaptureLifecycleInvokeArgs = Record<string, unknown>;
export type CaptureLifecycleInvoke = (
  command: string,
  args?: CaptureLifecycleInvokeArgs,
) => Promise<unknown>;
export type CaptureInactiveHandler = () => void | Promise<void>;

interface CloseInactiveCaptureSessionOptions {
  onInactive?: CaptureInactiveHandler;
  resetSessionState: () => void;
}

interface FinishCaptureSessionOptions extends CloseInactiveCaptureSessionOptions {
  invoke: CaptureLifecycleInvoke;
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
  invoke,
  sessionId,
  onInactive,
  resetSessionState,
}: FinishCaptureSessionOptions) {
  await invoke('cancel_capture_session', { sessionId });
  await closeInactiveCaptureSession({ onInactive, resetSessionState });
}
