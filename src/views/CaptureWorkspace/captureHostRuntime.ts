import type { CaptureSelectionOverlayFrame } from './captureSelectionOverlay';

export async function prepareCaptureSurfaceForReveal({
  images = [],
  windowPrepared,
  frame,
  paintSelectionOverlayFrame,
  waitForPaint = waitForCaptureSurfacePaint,
}: {
  images?: readonly Pick<HTMLImageElement, 'decode'>[];
  windowPrepared?: Promise<void>;
  frame: CaptureSelectionOverlayFrame | null;
  paintSelectionOverlayFrame: (
    frame: CaptureSelectionOverlayFrame | null,
  ) => void;
  waitForPaint?: typeof waitForCaptureSurfacePaint;
}) {
  // Decode while native geometry is prepared, but paint only after both finish.
  await Promise.all([windowPrepared, ...images.map((image) => image.decode())]);
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
