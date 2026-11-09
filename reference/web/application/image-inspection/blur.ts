function boxBlur(
  source: Uint8ClampedArray,
  target: Uint8ClampedArray,
  width: number,
  height: number,
  radius: number,
  vertical: boolean,
) {
  const length = vertical ? height : width;
  const lines = vertical ? width : height;
  const stride = vertical ? width * 4 : 4;
  const size = radius * 2 + 1;
  for (let line = 0; line < lines; line += 1) {
    const base = vertical ? line * 4 : line * width * 4;
    for (let channel = 0; channel < 4; channel += 1) {
      let sum = source[base + channel] * (radius + 1);
      for (let i = 1; i <= radius; i += 1) {
        sum += source[base + Math.min(i, length - 1) * stride + channel];
      }
      for (let i = 0; i < length; i += 1) {
        target[base + i * stride + channel] = sum / size;
        sum +=
          source[
            base + Math.min(i + radius + 1, length - 1) * stride + channel
          ] - source[base + Math.max(i - radius, 0) * stride + channel];
      }
    }
  }
}

// Three box passes approximate Gaussian blur without Canvas filter support.
export function blurRgbaPixels(
  pixels: Uint8ClampedArray,
  width: number,
  height: number,
  sigma: number,
) {
  sigma = Math.max(0.1, Math.min(256, sigma));
  let lower = Math.floor(Math.sqrt(4 * sigma * sigma + 1));
  if (lower % 2 === 0) lower -= 1;
  const lowerPasses = Math.round(
    (12 * sigma * sigma - 3 * lower * lower - 12 * lower - 9) /
      (-4 * lower - 4),
  );
  let source = new Uint8ClampedArray(pixels);
  let target = new Uint8ClampedArray(pixels.length);
  for (let pass = 0; pass < 3; pass += 1) {
    const radius = (lower + (pass < lowerPasses ? 0 : 2) - 1) / 2;
    boxBlur(source, target, width, height, radius, false);
    [source, target] = [target, source];
    boxBlur(source, target, width, height, radius, true);
    [source, target] = [target, source];
  }
  return source;
}
