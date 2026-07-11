
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

export interface CaptureWindowRevealClient {
  prepareCaptureWindowForReveal?: () => Promise<void>;
  revealCaptureWindow: () => Promise<void>;
  restoreCaptureSnapshotWindowsForSession?: (sessionId: string) => Promise<void>;
}

interface RevealCaptureWindowForSessionOptions {
  client: CaptureWindowRevealClient;
  sessionId: string;
  prepareSurface?: () => void | Promise<void>;
}

export function getCaptureWindowRevealPermissions() {
  return [] as const;
}

export function shouldRevealCaptureWindow({
  status,
  hasSession,
  hasCaptureImagesReady: _hasCaptureImagesReady,
  hasRevealed,
}: CaptureWindowRevealState) {
  if (hasRevealed) return false;
  if (status === 'error') return true;

  return hasSession && (status === 'selecting' || status === 'preview');
}

export async function revealCaptureWindow(
  client: CaptureWindowRevealClient,
) {
  await client.revealCaptureWindow();
}

export async function waitForCaptureSurfacePaint(
  requestAnimationFrame: typeof globalThis.requestAnimationFrame =
    globalThis.requestAnimationFrame,
  timeoutMs = 48,
) {
  const paint = new Promise<void>((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });

  if (timeoutMs <= 0) {
    await paint;
    return;
  }

  await Promise.race([
    paint,
    new Promise<void>((resolve) => {
      globalThis.setTimeout(resolve, timeoutMs);
    }),
  ]);
}

export async function revealCaptureWindowForSession({
  client,
  sessionId: _sessionId,
  prepareSurface,
}: RevealCaptureWindowForSessionOptions) {
  await client.prepareCaptureWindowForReveal?.();
  await prepareSurface?.();
  await revealCaptureWindow(client);
}
