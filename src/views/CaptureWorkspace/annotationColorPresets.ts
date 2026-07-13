import type { AnnotationColor } from './annotationStyle';

export function annotationColorsEqual(
  first: AnnotationColor,
  second: AnnotationColor,
) {
  return first.every((channel, index) => channel === second[index]);
}

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

export function addAnnotationColorPreset(
  presets: readonly AnnotationColor[],
  color: AnnotationColor,
) {
  if (presets.some((preset) => annotationColorsEqual(preset, color))) {
    return [...presets];
  }

  return [...presets, color];
}

export function replaceAnnotationColorPreset(
  presets: readonly AnnotationColor[],
  index: number,
  color: AnnotationColor,
) {
  if (
    index < 0 ||
    index >= presets.length ||
    presets.some(
      (preset, presetIndex) =>
        presetIndex !== index && annotationColorsEqual(preset, color),
    )
  ) {
    return [...presets];
  }

  return presets.map((preset, presetIndex) =>
    presetIndex === index ? color : preset,
  );
}

export function removeAnnotationColorPreset(
  presets: readonly AnnotationColor[],
  index: number,
) {
  if (index < 0 || index >= presets.length) return [...presets];
  return presets.filter((_, presetIndex) => presetIndex !== index);
}
