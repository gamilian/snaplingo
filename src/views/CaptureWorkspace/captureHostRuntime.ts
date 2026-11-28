import type { CaptureSelectionOverlayFrame } from './captureSelectionOverlay';

export async function prepareCaptureSurfaceForReveal({
  windowPrepared,
  frame,
  paintSelectionOverlayFrame,
  flushStyles,
}: {
  windowPrepared?: Promise<void>;
  frame: CaptureSelectionOverlayFrame | null;
  paintSelectionOverlayFrame: (
    frame: CaptureSelectionOverlayFrame | null,
  ) => void;
  flushStyles: () => void;
}) {
  // Visual layers stay transparent until native reveal. Waiting for a paint
  // while the WebView is hidden only stalls input until its timer fallback.
  await windowPrepared;
  paintSelectionOverlayFrame(frame);
  // Commit opacity=0 before reveal so the entrance transition has a start state.
  flushStyles();
}
