import {
  getImageSamplePoint,
  rgbaToHex,
  type ColorSample,
} from '../../application/image-inspection/colorSampler';

interface Point {
  x: number;
  y: number;
}
interface Size {
  width: number;
  height: number;
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
