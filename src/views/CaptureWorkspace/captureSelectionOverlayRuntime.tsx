import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  type Ref,
  type RefObject,
} from 'react';
import { getCaptureSelectionOverlayCanvasClassName } from './capturePresentation';
import {
  drawCaptureSelectionOverlayFrame,
  getCaptureSelectionOverlayCursor,
  getCaptureSelectionOverlayFrame,
  type CaptureSelectionOverlayFrame,
} from './captureSelectionOverlay';
import type { LogicalRect, Point } from './types';

type CaptureSelectionOverlayStatus =
  | 'idle'
  | 'loading'
  | 'selecting'
  | 'preview'
  | 'error';

interface CaptureSelectionOverlayPixelRatioSource {
  devicePixelRatio?: number;
}

interface CaptureSelectionOverlaySize {
  width: number;
  height: number;
}

interface UseCaptureSelectionOverlayOptions {
  status: CaptureSelectionOverlayStatus;
  selectionBounds: LogicalRect | null;
  selection: LogicalRect | null;
  viewportBounds: LogicalRect | null;
  cursorPointRef: RefObject<Point | null>;
  draftSelectionRef: RefObject<LogicalRect | null>;
  hoverSelectionRef: RefObject<LogicalRect | null>;
  showSelectionSize?: boolean;
}

interface CaptureSelectionOverlayCanvasProps {
  canvasRef: Ref<HTMLCanvasElement>;
  cssSize: CaptureSelectionOverlaySize | null;
  pixelRatio: number;
}

export function getSelectionOverlayPixelRatio(
  source: CaptureSelectionOverlayPixelRatioSource | undefined =
    typeof window === 'undefined' ? undefined : window,
) {
  return Math.max(1, source?.devicePixelRatio || 1);
}

export function getSelectionOverlayCanvasSize(
  viewportBounds: LogicalRect | null,
): CaptureSelectionOverlaySize | null {
  if (!viewportBounds) return null;

  return {
    width: Math.max(0, Math.round(viewportBounds.width)),
    height: Math.max(0, Math.round(viewportBounds.height)),
  };
}

export function CaptureSelectionOverlayCanvas({
  canvasRef,
  cssSize,
  pixelRatio,
}: CaptureSelectionOverlayCanvasProps) {
  if (!cssSize) return null;

  return (
    <canvas
      ref={canvasRef}
      width={Math.round(cssSize.width * pixelRatio)}
      height={Math.round(cssSize.height * pixelRatio)}
      className={getCaptureSelectionOverlayCanvasClassName()}
      style={{
        width: `${cssSize.width}px`,
        height: `${cssSize.height}px`,
      }}
      aria-hidden="true"
    />
  );
}

export function useCaptureSelectionOverlay({
  cursorPointRef,
  draftSelectionRef,
  hoverSelectionRef,
  showSelectionSize = true,
  selection,
  selectionBounds,
  status,
  viewportBounds,
}: UseCaptureSelectionOverlayOptions) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const animationFrameRef = useRef<number | null>(null);
  const frameRef = useRef<CaptureSelectionOverlayFrame | null>(null);
  const cssSize = useMemo(
    () => getSelectionOverlayCanvasSize(viewportBounds),
    [viewportBounds],
  );
  const pixelRatio = getSelectionOverlayPixelRatio();

  const paintFrame = useCallback(
    (frame: CaptureSelectionOverlayFrame | null) => {
      const canvas = canvasRef.current;
      const context = canvas?.getContext('2d');
      if (!canvas || !context || !cssSize) return;

      context.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
      drawCaptureSelectionOverlayFrame(
        context,
        cssSize,
        frame,
        getCaptureSelectionOverlayCursor({
          status,
          selectionBounds,
          cursorPoint: cursorPointRef.current,
        }),
      );
    },
    [cssSize, cursorPointRef, pixelRatio, selectionBounds, status],
  );

  const schedulePaint = useCallback(
    (
      draftSelection: LogicalRect | null = draftSelectionRef.current,
      hoverSelection: LogicalRect | null = hoverSelectionRef.current,
      activeSelection: LogicalRect | null = selection,
    ) => {
      frameRef.current = getCaptureSelectionOverlayFrame({
        status,
        selectionBounds,
        selection: activeSelection,
        draftSelection,
        hoverSelection,
        showSelectionSize,
      });

      if (animationFrameRef.current !== null) return;

      animationFrameRef.current = window.requestAnimationFrame(() => {
        animationFrameRef.current = null;
        paintFrame(frameRef.current);
      });
    },
    [
      draftSelectionRef,
      hoverSelectionRef,
      paintFrame,
      selection,
      selectionBounds,
      showSelectionSize,
      status,
    ],
  );

  const reset = useCallback(() => {
    frameRef.current = null;
    paintFrame(null);
  }, [paintFrame]);

  const getCurrentFrame = useCallback(() => frameRef.current, []);

  useEffect(() => {
    schedulePaint();
  }, [schedulePaint, selection, viewportBounds]);

  useEffect(() => {
    return () => {
      if (animationFrameRef.current !== null) {
        window.cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, []);

  return {
    canvasRef,
    cssSize,
    pixelRatio,
    paintFrame,
    schedulePaint,
    reset,
    getCurrentFrame,
  };
}
