import { describe, expect, it } from 'vitest';
import {
  getMagnifierImageStyle,
  getMagnifierPosition,
} from './magnifier';
import type { LogicalRect, Point } from './types';

const bounds: LogicalRect = { x: 0, y: 0, width: 300, height: 200 };

describe('capture magnifier', () => {
  it('positions near the cursor and flips away from capture edges', () => {
    const size = { width: 120, height: 96 };

    expect(getMagnifierPosition({ x: 40, y: 30 }, bounds, size, 12)).toEqual({
      x: 52,
      y: 42,
    });
    expect(getMagnifierPosition({ x: 260, y: 170 }, bounds, size, 12)).toEqual({
      x: 128,
      y: 62,
    });
  });

  it('centers the frozen image background on the cursor point', () => {
    const cursor: Point = { x: 30, y: 20 };

    expect(
      getMagnifierImageStyle(
        'frozen-image',
        cursor,
        { width: 300, height: 200 },
        { width: 120, height: 96 },
        4,
      ),
    ).toEqual({
      backgroundImage: 'url(data:image/png;base64,frozen-image)',
      backgroundSize: '1200px 800px',
      backgroundPosition: '-60px -32px',
    });
  });
});
