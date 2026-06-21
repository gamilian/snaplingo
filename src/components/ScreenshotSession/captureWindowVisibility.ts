type CaptureWindowVisibilityStatus =
  | 'idle'
  | 'loading'
  | 'selecting'
  | 'preview'
  | 'error';

interface CaptureWindowRevealState {
  status: CaptureWindowVisibilityStatus;
  hasSession: boolean;
  hasCaptureImagesReady: boolean;
  hasRevealed: boolean;
}

export interface CaptureWindowHandle {
  show: () => Promise<void>;
  setFocus: () => Promise<void>;
}

export type CaptureWindowRevealInvoke = (
  command: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;

interface RevealCaptureWindowForSessionOptions {
  window: CaptureWindowHandle;
  invoke: CaptureWindowRevealInvoke;
  sessionId: string;
}

export function getCaptureWindowRevealPermissions() {
  return ['core:window:allow-show', 'core:window:allow-set-focus'] as const;
}

export function shouldRevealCaptureWindow({
  status,
  hasSession,
  hasCaptureImagesReady,
  hasRevealed,
}: CaptureWindowRevealState) {
  if (hasRevealed) return false;
  if (status === 'error') return true;

  return (
    hasSession &&
    hasCaptureImagesReady &&
    (status === 'selecting' || status === 'preview')
  );
}

export async function revealCaptureWindow(window: CaptureWindowHandle) {
  await window.show();
  await window.setFocus();
}

export async function waitForCaptureSurfacePaint(
  requestAnimationFrame: typeof globalThis.requestAnimationFrame =
    globalThis.requestAnimationFrame,
) {
  await new Promise<void>((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });
}

export async function revealCaptureWindowForSession({
  window,
  invoke,
  sessionId,
}: RevealCaptureWindowForSessionOptions) {
  await revealCaptureWindow(window);
  await invoke('restore_capture_snapshot_windows_for_session', { sessionId });
}
