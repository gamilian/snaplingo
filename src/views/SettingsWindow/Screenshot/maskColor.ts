import type { AnnotationColorPreset } from '../../../application/settings/ports';

export function maskColorHex(color: AnnotationColorPreset) {
  return `#${color
    .slice(0, 3)
    .map((channel) => channel.toString(16).padStart(2, '0'))
    .join('')}`;
}

export function maskColorWithHex(
  color: AnnotationColorPreset,
  hex: string,
): AnnotationColorPreset {
  return [
    Number.parseInt(hex.slice(1, 3), 16),
    Number.parseInt(hex.slice(3, 5), 16),
    Number.parseInt(hex.slice(5, 7), 16),
    color[3],
  ];
}

export function maskColorOpacity(color: AnnotationColorPreset) {
  return Math.round((color[3] / 255) * 100);
}

export function maskColorWithOpacity(
  color: AnnotationColorPreset,
  opacity: number,
): AnnotationColorPreset {
  return [color[0], color[1], color[2], Math.round((opacity / 100) * 255)];
}
