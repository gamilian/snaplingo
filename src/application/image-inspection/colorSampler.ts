interface Point {
  x: number;
  y: number;
}

interface Size {
  width: number;
  height: number;
}

export interface ColorSample {
  hex: string;
  red: number;
  green: number;
  blue: number;
}

export type ColorSampleFormat = 'hex' | 'rgb';

export function colorSamplesEqual(
  first: ColorSample | null,
  second: ColorSample | null,
) {
  return (
    first === second ||
    (first !== null &&
      second !== null &&
      first.red === second.red &&
      first.green === second.green &&
      first.blue === second.blue)
  );
}

interface ColorSampleCopyShortcutEvent {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

export function getImageSamplePoint(
  cursor: Point,
  logicalSize: Size,
  imageSize: Size,
): Point {
  const xRatio = logicalSize.width > 0 ? cursor.x / logicalSize.width : 0;
  const yRatio = logicalSize.height > 0 ? cursor.y / logicalSize.height : 0;

  return {
    x: clamp(Math.floor(xRatio * imageSize.width), 0, imageSize.width - 1),
    y: clamp(Math.floor(yRatio * imageSize.height), 0, imageSize.height - 1),
  };
}

function channelToHex(value: number) {
  return clamp(Math.round(value), 0, 255)
    .toString(16)
    .padStart(2, '0')
    .toUpperCase();
}

export function rgbaToHex(red: number, green: number, blue: number) {
  return `#${channelToHex(red)}${channelToHex(green)}${channelToHex(blue)}`;
}

export function colorSampleToClipboardText(
  sample: ColorSample,
  format: ColorSampleFormat = 'hex',
) {
  if (format === 'rgb') {
    return `rgb(${sample.red}, ${sample.green}, ${sample.blue})`;
  }

  return sample.hex;
}

export function isColorSampleCopyShortcut(event: ColorSampleCopyShortcutEvent) {
  return (
    event.key.toLowerCase() === 'c' &&
    !event.metaKey &&
    !event.ctrlKey
  );
}

export function isColorSampleFormatToggleShortcut(
  event: ColorSampleCopyShortcutEvent,
) {
  return (
    event.key === 'Shift' &&
    !event.metaKey &&
    !event.ctrlKey
  );
}

export function sampleCanvasColor(
  canvas: HTMLCanvasElement,
  cursor: Point,
  logicalSize: Size,
): ColorSample | null {
  const context = canvas.getContext('2d');
  if (!context || canvas.width <= 0 || canvas.height <= 0) return null;

  const point = getImageSamplePoint(cursor, logicalSize, {
    width: canvas.width,
    height: canvas.height,
  });
  const [red, green, blue] = context.getImageData(point.x, point.y, 1, 1).data;

  return {
    hex: rgbaToHex(red, green, blue),
    red,
    green,
    blue,
  };
}

export function sampleImageColor(
  image: HTMLImageElement,
  sampleCanvas: HTMLCanvasElement,
  cursor: Point,
  logicalSize: Size,
): ColorSample | null {
  if (image.naturalWidth <= 0 || image.naturalHeight <= 0) return null;
  if (sampleCanvas.width !== 1) sampleCanvas.width = 1;
  if (sampleCanvas.height !== 1) sampleCanvas.height = 1;
  const context = sampleCanvas.getContext('2d');
  if (!context) return null;

  const point = getImageSamplePoint(cursor, logicalSize, {
    width: image.naturalWidth,
    height: image.naturalHeight,
  });
  context.clearRect(0, 0, 1, 1);
  context.imageSmoothingEnabled = false;
  context.drawImage(image, point.x, point.y, 1, 1, 0, 0, 1, 1);
  const [red, green, blue] = context.getImageData(0, 0, 1, 1).data;

  return {
    hex: rgbaToHex(red, green, blue),
    red,
    green,
    blue,
  };
}
