import { useLayoutEffect, useRef } from 'react';
import {
  colorSampleToClipboardText,
  type ColorSample,
  type ColorSampleFormat,
} from './colorSampler';
import {
  getMagnifierCanvasBlit,
  getMagnifierPosition,
  normalizeMagnifierZoom,
} from './magnifier';
import type { LogicalRect, Point } from './types';

const MAGNIFIER_GAP = 14;
const MAGNIFIER_PANEL_SIZE = { width: 228, height: 226 };
const MAGNIFIER_LENS_SIZE = { width: 228, height: 132 };

interface CaptureMagnifierOverlayProps {
  image: HTMLImageElement;
  viewportCursor: Point;
  screenCursor: Point;
  imageCursor: Point;
  viewportBounds: LogicalRect;
  imageSize: { width: number; height: number };
  color: ColorSample | null;
  colorFormat: ColorSampleFormat;
  zoom: number;
}

export function CaptureMagnifierOverlay({
  image,
  viewportCursor,
  screenCursor,
  imageCursor,
  viewportBounds,
  imageSize,
  color,
  colorFormat,
  zoom,
}: CaptureMagnifierOverlayProps) {
  const position = getMagnifierPosition(
    viewportCursor,
    viewportBounds,
    MAGNIFIER_PANEL_SIZE,
    MAGNIFIER_GAP,
  );
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useLayoutEffect(() => {
    const canvas = canvasRef.current;
    const context = canvas?.getContext('2d');
    if (!canvas || !context) return;

    context.clearRect(0, 0, canvas.width, canvas.height);
    context.fillStyle = 'rgba(15, 23, 42, 0.62)';
    context.fillRect(0, 0, canvas.width, canvas.height);
    if (!image) return;

    const blit = getMagnifierCanvasBlit(
      imageCursor,
      imageSize,
      { width: image.naturalWidth, height: image.naturalHeight },
      MAGNIFIER_LENS_SIZE,
      normalizeMagnifierZoom(zoom),
    );
    if (!blit) return;

    context.imageSmoothingEnabled = false;
    context.drawImage(
      image,
      blit.source.x,
      blit.source.y,
      blit.source.width,
      blit.source.height,
      blit.destination.x,
      blit.destination.y,
      blit.destination.width,
      blit.destination.height,
    );
  }, [
    image,
    imageCursor.x,
    imageCursor.y,
    imageSize.height,
    imageSize.width,
    zoom,
  ]);

  const colorText = color
    ? colorFormat === 'rgb'
      ? `${color.red}, ${color.green}, ${color.blue}`
      : colorSampleToClipboardText(color, colorFormat)
    : '—';

  return (
    <div
      aria-label="像素放大镜"
      className="pointer-events-none absolute left-0 top-0 z-[70] will-change-transform overflow-hidden rounded-[6px] border border-white/30 bg-transparent text-white shadow-xl ring-1 ring-black/20"
      style={{
        transform: `translate3d(${position.x}px, ${position.y}px, 0)`,
        width: `${MAGNIFIER_PANEL_SIZE.width}px`,
      }}
    >
      <div className="relative border-b border-white/15">
        <canvas
          ref={canvasRef}
          width={MAGNIFIER_LENS_SIZE.width}
          height={MAGNIFIER_LENS_SIZE.height}
          className="block [image-rendering:pixelated]"
        />
        <div className="absolute left-1/2 top-0 h-full w-[2px] -translate-x-1/2 bg-blue-400/[0.65]" />
        <div className="absolute left-0 top-1/2 h-[2px] w-full -translate-y-1/2 bg-blue-400/[0.65]" />
        <div className="absolute left-1/2 top-1/2 h-3 w-3 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.85)]" />
      </div>
      <div className="bg-slate-900/[0.58] px-4 py-2.5 font-mono text-[12px] leading-5">
        <div className="text-center font-semibold tracking-wide">
          ({Math.round(screenCursor.x)}, {Math.round(screenCursor.y)})
        </div>
        <div className="mt-1 flex items-center justify-center gap-2 font-semibold">
          <span
            className="h-4 w-4 border border-white/70 shadow-[2px_2px_0_rgba(255,255,255,0.45)]"
            style={{ backgroundColor: color?.hex ?? 'transparent' }}
          />
          <span>{colorText}</span>
        </div>
        <div className="mt-1.5 border-t border-white/15 pt-1.5 text-center text-[11px] leading-[17px] text-white/90">
          <div>按 C 复制颜色值</div>
          <div>按 Shift 切换 RGB / HEX</div>
        </div>
      </div>
    </div>
  );
}
