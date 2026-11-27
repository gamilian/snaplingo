import type { CaptureSelectionOverlayFrame } from './captureSelectionOverlay';

export async function prepareCaptureSurfaceForReveal({
  windowPrepared,
  frame,
  paintSelectionOverlayFrame,
  waitForPaint = waitForCaptureSurfacePaint,
}: {
  windowPrepared?: Promise<void>;
  frame: CaptureSelectionOverlayFrame | null;
  paintSelectionOverlayFrame: (
    frame: CaptureSelectionOverlayFrame | null,
  ) => void;
  waitForPaint?: typeof waitForCaptureSurfacePaint;
}) {
  // Selection input only needs geometry and the overlay, not decoded pixels.
  await windowPrepared;
  paintSelectionOverlayFrame(frame);
  await waitForPaint();
}

export async function waitForCaptureSurfacePaint(
  requestAnimationFrame: typeof globalThis.requestAnimationFrame =
    globalThis.requestAnimationFrame,
  timeoutMs = 48,
) {
  const paint = new Promise<void>((resolve) => {
    requestAnimationFrame(() => resolve());
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
