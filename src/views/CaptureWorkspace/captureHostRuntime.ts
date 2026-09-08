import type { CaptureSelectionOverlayFrame } from './captureSelectionOverlay';

export async function prepareCaptureSurfaceForReveal({
  images = [],
  frame,
  paintSelectionOverlayFrame,
  waitForPaint = waitForCaptureSurfacePaint,
}: {
  images?: readonly Pick<HTMLImageElement, 'decode'>[];
  frame: CaptureSelectionOverlayFrame | null;
  paintSelectionOverlayFrame: (
    frame: CaptureSelectionOverlayFrame | null,
  ) => void;
  waitForPaint?: typeof waitForCaptureSurfacePaint;
}) {
  await Promise.all(images.map((image) => image.decode()));
  paintSelectionOverlayFrame(frame);
  await waitForPaint();
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
