import { describe, expect, it } from 'vitest';

import type { AnnotationColor } from './annotationStyle';
import {
  addAnnotationColorPreset,
  annotationColorFromHex,
  annotationColorToHex,
  removeAnnotationColorPreset,
  replaceAnnotationColorPreset,
} from './annotationColorPresets';

describe('annotation color presets', () => {
  it('converts opaque annotation colors to and from hex values', () => {
    expect(annotationColorToHex([255, 77, 79, 255])).toBe('#FF4D4F');
    expect(annotationColorFromHex('#1890ff')).toEqual([24, 144, 255, 255]);
    expect(annotationColorFromHex('#xyzxyz')).toBeNull();
  });

  it('adds a new color without duplicating an existing preset', () => {
    const presets: AnnotationColor[] = [
      [255, 77, 79, 255],
      [24, 144, 255, 255],
    ];

    expect(addAnnotationColorPreset(presets, [40, 167, 69, 255])).toEqual([
      [255, 77, 79, 255],
      [24, 144, 255, 255],
      [40, 167, 69, 255],
    ]);
    expect(addAnnotationColorPreset(presets, [24, 144, 255, 255])).toEqual(
      presets,
    );
  });

  it('replaces a preset in place and rejects duplicate replacements', () => {
    const presets: AnnotationColor[] = [
      [255, 77, 79, 255],
      [24, 144, 255, 255],
    ];

    expect(
      replaceAnnotationColorPreset(presets, 0, [250, 219, 20, 255]),
    ).toEqual([
      [250, 219, 20, 255],
      [24, 144, 255, 255],
    ]);
    expect(
      replaceAnnotationColorPreset(presets, 0, [24, 144, 255, 255]),
    ).toEqual(presets);
  });

  it('removes a preset without changing the current annotation color', () => {
    const presets: AnnotationColor[] = [
      [255, 77, 79, 255],
      [24, 144, 255, 255],
    ];

    expect(removeAnnotationColorPreset(presets, 0)).toEqual([
      [24, 144, 255, 255],
    ]);
  });
});
