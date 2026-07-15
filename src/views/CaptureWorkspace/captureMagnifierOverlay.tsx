import {
  colorSampleToClipboardText,
  type ColorSample,
  type ColorSampleFormat,
} from './colorSampler';
import {
  getMagnifierImageStyle,
  getMagnifierPosition,
} from './magnifier';
import type { LogicalRect, Point } from './types';

const MAGNIFIER_GAP = 14;
const MAGNIFIER_PANEL_SIZE = { width: 220, height: 226 };
const MAGNIFIER_LENS_SIZE = { width: 220, height: 132 };
const MAGNIFIER_ZOOM = 6;

interface CaptureMagnifierOverlayProps {
  imageBase64: string;
  viewportCursor: Point;
  screenCursor: Point;
  imageCursor: Point;
  viewportBounds: LogicalRect;
  imageSize: { width: number; height: number };
  color: ColorSample | null;
  colorFormat: ColorSampleFormat;
}

export function CaptureMagnifierOverlay({
  imageBase64,
  viewportCursor,
  screenCursor,
  imageCursor,
  viewportBounds,
  imageSize,
  color,
  colorFormat,
}: CaptureMagnifierOverlayProps) {
  const position = getMagnifierPosition(
    viewportCursor,
    viewportBounds,
    MAGNIFIER_PANEL_SIZE,
    MAGNIFIER_GAP,
  );
  const imageStyle = getMagnifierImageStyle(
    imageBase64,
    imageCursor,
    imageSize,
    MAGNIFIER_LENS_SIZE,
    MAGNIFIER_ZOOM,
  );
  const colorText = color
    ? colorFormat === 'rgb'
      ? `${color.red}, ${color.green}, ${color.blue}`
      : colorSampleToClipboardText(color, colorFormat)
    : '—';

  return (
    <div
      aria-label="像素放大镜"
      className="pointer-events-none absolute overflow-hidden rounded-[5px] border border-white/70 bg-neutral-950 text-white shadow-2xl ring-1 ring-black/60"
      style={{
        left: `${position.x}px`,
        top: `${position.y}px`,
        width: `${MAGNIFIER_PANEL_SIZE.width}px`,
      }}
    >
      <div
        className="relative border-b border-white/20"
        style={{
          ...imageStyle,
          width: `${MAGNIFIER_LENS_SIZE.width}px`,
          height: `${MAGNIFIER_LENS_SIZE.height}px`,
          backgroundRepeat: 'no-repeat',
        }}
      >
        <div className="absolute left-1/2 top-0 h-full w-[2px] -translate-x-1/2 bg-blue-500/75" />
        <div className="absolute left-0 top-1/2 h-[2px] w-full -translate-y-1/2 bg-blue-500/75" />
        <div className="absolute left-1/2 top-1/2 h-3 w-3 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.85)]" />
      </div>
      <div className="bg-neutral-950 px-4 py-2.5 font-mono text-[12px] leading-5">
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
