export type AnnotationColor = [number, number, number, number];

export const ANNOTATION_COLORS: AnnotationColor[] = [
  [255, 77, 79, 255],
  [40, 167, 69, 255],
  [24, 144, 255, 255],
  [250, 219, 20, 255],
  [255, 255, 255, 255],
  [0, 0, 0, 255],
];

export function annotationColorsEqual(
  first: AnnotationColor,
  second: AnnotationColor,
) {
  return first.every((channel, index) => channel === second[index]);
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
