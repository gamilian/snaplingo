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
const MAGNIFIER_SIZE = { width: 120, height: 96 };
const MAGNIFIER_ZOOM = 4;

interface CaptureMagnifierOverlayProps {
  imageBase64: string;
  viewportCursor: Point;
  imageCursor: Point;
  viewportBounds: LogicalRect;
  imageSize: { width: number; height: number };
  selection: LogicalRect | null;
  color: ColorSample | null;
  colorFormat: ColorSampleFormat;
}

export function CaptureMagnifierOverlay({
  imageBase64,
  viewportCursor,
  imageCursor,
  viewportBounds,
  imageSize,
  selection,
  color,
  colorFormat,
}: CaptureMagnifierOverlayProps) {
  const position = getMagnifierPosition(
    viewportCursor,
    viewportBounds,
    MAGNIFIER_SIZE,
    MAGNIFIER_GAP,
  );
  const imageStyle = getMagnifierImageStyle(
    imageBase64,
    imageCursor,
    imageSize,
    MAGNIFIER_SIZE,
    MAGNIFIER_ZOOM,
  );
  const sizeText = selection
    ? `${Math.round(selection.width)} x ${Math.round(selection.height)} px`
    : '';
  const colorText = color ? colorSampleToClipboardText(color, colorFormat) : '';

  return (
    <div
      className="pointer-events-none absolute overflow-hidden rounded border border-white/70 bg-neutral-950 text-[10px] text-white shadow-2xl ring-1 ring-black/50"
      style={{
        left: `${position.x}px`,
        top: `${position.y}px`,
        width: `${MAGNIFIER_SIZE.width}px`,
      }}
    >
      <div
        className="relative border-b border-white/20"
        style={{
          ...imageStyle,
          width: `${MAGNIFIER_SIZE.width}px`,
          height: `${MAGNIFIER_SIZE.height}px`,
          backgroundRepeat: 'no-repeat',
        }}
      >
        <div className="absolute left-1/2 top-0 h-full w-px bg-red-400/90" />
        <div className="absolute left-0 top-1/2 h-px w-full bg-red-400/90" />
      </div>
      <div className="flex items-center justify-between px-1.5 py-1 font-mono leading-none">
        <span>
          {Math.round(imageCursor.x)}, {Math.round(imageCursor.y)}
        </span>
        <span className="flex items-center gap-1">
          {color && (
            <>
              <span
                className="h-2.5 w-2.5 border border-white/60"
                style={{ backgroundColor: color.hex }}
              />
              <span>{colorText}</span>
            </>
          )}
          {sizeText && <span>{sizeText}</span>}
        </span>
      </div>
    </div>
  );
}
