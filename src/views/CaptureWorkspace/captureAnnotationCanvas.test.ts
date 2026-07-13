import { describe, expect, it, vi } from 'vitest';

import { drawCaptureAnnotation } from './captureAnnotationCanvas';
import type { AnnotationCommand } from './types';

describe('capture annotation canvas', () => {
  it('samples mosaic pixels from the immutable capture source', () => {
    const displayCanvas = { dataset: { role: 'display' } } as unknown as HTMLCanvasElement;
    const sourceCanvas = { dataset: { role: 'source' } } as unknown as HTMLCanvasElement;
    const mosaicBufferContext = {
      clearRect: vi.fn(),
      drawImage: vi.fn(),
      imageSmoothingEnabled: true,
    } as unknown as CanvasRenderingContext2D;
    const mosaicBuffer = {
      dataset: { role: 'mosaic' },
      getContext: vi.fn(() => mosaicBufferContext),
    } as unknown as HTMLCanvasElement;
    const displayContext = {
      canvas: displayCanvas,
      save: vi.fn(),
      restore: vi.fn(),
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      arc: vi.fn(),
      clip: vi.fn(),
      drawImage: vi.fn(),
      getTransform: vi.fn(() => ({ a: 1 })),
      imageSmoothingEnabled: true,
    } as unknown as CanvasRenderingContext2D;
    const mosaic: AnnotationCommand = {
      type: 'mosaic',
      points: [{ x: 5, y: 5 }],
      stroke_width: 4,
      block_size: 2,
    };

    drawCaptureAnnotation(displayContext, mosaic, {
      mosaic: mosaicBuffer,
      source: sourceCanvas,
    });

    expect(mosaicBufferContext.drawImage).toHaveBeenCalledWith(
      sourceCanvas,
      3,
      3,
      4,
      4,
      0,
      0,
      2,
      2,
    );
  });
});
