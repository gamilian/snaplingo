import { restoreCaptureSnapshotWindowsForSession } from '../../tauri/captureSession';

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

export interface CaptureWindowRevealClient {
  restoreCaptureSnapshotWindowsForSession: (
    sessionId: string,
  ) => Promise<void>;
}

const tauriCaptureWindowRevealClient: CaptureWindowRevealClient = {
  restoreCaptureSnapshotWindowsForSession,
};

interface RevealCaptureWindowForSessionOptions {
  window: CaptureWindowHandle;
  client?: CaptureWindowRevealClient;
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
  client = tauriCaptureWindowRevealClient,
  sessionId,
}: RevealCaptureWindowForSessionOptions) {
  await revealCaptureWindow(window);
  await client.restoreCaptureSnapshotWindowsForSession(sessionId);
}
