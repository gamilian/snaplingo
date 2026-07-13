import type { AnnotationColor } from '../../domain/annotationColor';

export function annotationColorToHex(color: AnnotationColor) {
  return `#${color
    .slice(0, 3)
    .map((channel) => channel.toString(16).padStart(2, '0'))
    .join('')}`.toUpperCase();
}

export function annotationColorFromHex(value: string): AnnotationColor | null {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(value);
  if (!match) return null;

  return [
    Number.parseInt(match[1], 16),
    Number.parseInt(match[2], 16),
    Number.parseInt(match[3], 16),
    255,
  ];
}

export function annotationColorToCss(color: AnnotationColor) {
  return `rgba(${color[0]}, ${color[1]}, ${color[2]}, ${color[3] / 255})`;
}
