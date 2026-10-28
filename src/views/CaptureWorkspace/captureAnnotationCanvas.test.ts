// @vitest-environment happy-dom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { CaptureAnnotationCanvas, drawCaptureAnnotation, drawFrozenCaptureSelection, getCaptureAnnotationCanvasBackingSize } from './captureAnnotationCanvas';
import type { AnnotationCommand, MonitorSnapshotView } from './types';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('capture annotation canvas', () => {
  it('keeps local DPR sharpness while guaranteeing the shared output pixel size', () => {
    const selection = { x: 0, y: 0, width: 100, height: 80 };

    expect(
      getCaptureAnnotationCanvasBackingSize(selection, 3, { width: 150, height: 120 }),
    ).toEqual({ pixelRatio: 3, width: 300, height: 240 });
    expect(
      getCaptureAnnotationCanvasBackingSize(selection, 1, { width: 150, height: 120 }),
    ).toEqual({ pixelRatio: 1.5, width: 150, height: 120 });
  });

  it('composes frozen monitors with negative origins and different pixel densities', () => {
    const context = { clearRect: vi.fn(), drawImage: vi.fn() } as unknown as CanvasRenderingContext2D;
    const left = document.createElement('img');
    const right = document.createElement('img');
    const cursor = document.createElement('img');
    const monitors: MonitorSnapshotView[] = [
      { id: 'left', logical_bounds: { x: -400, y: -100, width: 400, height: 300 }, physical_bounds: { x: -400, y: -100, width: 400, height: 300 }, scale_factor: 1, image_base64: '' },
      { id: 'right', logical_bounds: { x: 0, y: 0, width: 500, height: 300 }, physical_bounds: { x: 0, y: 0, width: 1000, height: 600 }, scale_factor: 2, image_base64: '' },
    ];
    drawFrozenCaptureSelection(context, { x: -50, y: -25, width: 150, height: 100 }, monitors,
      new Map([['left', left], ['right', right]]), {
        logical_position: { x: 10, y: 20 }, hotspot: { x: 2, y: 3 },
        image_width: 32, image_height: 48, scale_factor: 2, image_base64: '',
      }, cursor);
    expect(context.drawImage).toHaveBeenNthCalledWith(1, left, -350, -75, 400, 300);
    expect(context.drawImage).toHaveBeenNthCalledWith(2, right, 50, 25, 500, 300);
    expect(context.drawImage).toHaveBeenNthCalledWith(3, cursor, 58, 42, 16, 24);
  });

  it('uses Windows system fonts for capture text annotations', () => {
    vi.stubGlobal('navigator', { platform: 'Win32' });
    const context = {
      save: vi.fn(), restore: vi.fn(), fillText: vi.fn(),
    } as unknown as CanvasRenderingContext2D;

    drawCaptureAnnotation(context, {
      type: 'text', position: { x: 1, y: 2 }, text: 'OCR',
      color: [0, 0, 0, 255], font_size: 18,
    });

    expect(context.font).toBe(
      '18px "Segoe UI Variable Text", "Segoe UI", system-ui, "Microsoft YaHei", sans-serif',
    );
  });

  it('paints annotations at commit without repainting frozen pixels or waiting another frame', async () => {
    const sourceContext = { setTransform: vi.fn(), clearRect: vi.fn(), drawImage: vi.fn() };
    const annotationContext = {
      setTransform: vi.fn(), clearRect: vi.fn(), save: vi.fn(), restore: vi.fn(),
      beginPath: vi.fn(), rect: vi.fn(), stroke: vi.fn(),
    };
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(function (this: HTMLCanvasElement) {
      return (this.hasAttribute('data-capture-preview') ? sourceContext : annotationContext) as never;
    });
    const requestFrame = vi.spyOn(window, 'requestAnimationFrame');
    const desktop = document.createElement('div');
    const image = document.createElement('img');
    image.dataset.captureMonitor = 'monitor';
    desktop.append(image);
    const rect = { x: 0, y: 0, width: 100, height: 80 };
    const props = {
      desktopRef: { current: desktop },
      monitors: [{ id: 'monitor', logical_bounds: rect, physical_bounds: rect, scale_factor: 1, image_base64: '' }],
      outputPixelSize: { width: 100, height: 80 },
      selection: rect, selectionViewportRect: rect, draftAnnotation: null,
    };
    const container = document.createElement('div');
    const root = createRoot(container);
    try {
      await act(async () => root.render(createElement(CaptureAnnotationCanvas, { ...props, annotations: [] })));
      await act(async () => root.render(createElement(CaptureAnnotationCanvas, {
        ...props,
        annotations: [{ type: 'rectangle', rect: { x: 10, y: 10, width: 30, height: 20 }, stroke_width: 2, color: [255, 0, 0, 255], filled: false }],
      })));
      expect(annotationContext.stroke).toHaveBeenCalledOnce();
      expect(sourceContext.drawImage).toHaveBeenCalledExactlyOnceWith(image, 0, 0, 100, 80);
      expect(requestFrame).not.toHaveBeenCalled();
    } finally {
      await act(async () => root.unmount());
    }
  });

  it('samples mosaic pixels from the immutable capture source', () => {
    const displayCanvas = { dataset: { role: 'display' } } as unknown as HTMLCanvasElement;
    const sourceCanvas = { dataset: { role: 'source' }, width: 200, height: 80 } as unknown as HTMLCanvasElement;
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
      2,
      2,
      6,
      6,
      0,
      0,
      3,
      3,
    );
  });

  it('rasterizes only newly added draft mosaic segments while the pointer moves', async () => {
    const sourceContext = {
      setTransform: vi.fn(), clearRect: vi.fn(), drawImage: vi.fn(),
      getTransform: vi.fn(() => ({ a: 1 })),
    };
    const annotationContext = {
      setTransform: vi.fn(), clearRect: vi.fn(), save: vi.fn(), restore: vi.fn(),
      beginPath: vi.fn(), rect: vi.fn(), stroke: vi.fn(), moveTo: vi.fn(),
      arc: vi.fn(), clip: vi.fn(), drawImage: vi.fn(),
      getTransform: vi.fn(() => ({ a: 1 })), imageSmoothingEnabled: true,
    };
    const draftContext = {
      canvas: { dataset: { role: 'draft' } },
      setTransform: vi.fn(), clearRect: vi.fn(), save: vi.fn(), restore: vi.fn(),
      beginPath: vi.fn(), moveTo: vi.fn(), arc: vi.fn(), clip: vi.fn(),
      drawImage: vi.fn(), getTransform: vi.fn(() => ({ a: 1 })),
      imageSmoothingEnabled: true,
    };
    const mosaicBufferContext = {
      clearRect: vi.fn(), drawImage: vi.fn(), imageSmoothingEnabled: true,
    };
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(function (this: HTMLCanvasElement) {
      if (this.hasAttribute('data-capture-preview')) return sourceContext as never;
      if (this.hasAttribute('data-capture-draft')) return draftContext as never;
      if (this.hasAttribute('data-capture-mosaic-buffer') || this.width < 200) return mosaicBufferContext as never;
      return annotationContext as never;
    });

    const desktop = document.createElement('div');
    const image = document.createElement('img');
    image.dataset.captureMonitor = 'monitor';
    desktop.append(image);
    const selection = { x: 0, y: 0, width: 200, height: 80 };
    const mosaic = (points: Array<{ x: number; y: number }>): AnnotationCommand => ({
      type: 'mosaic', points, stroke_width: 4, block_size: 2,
    });
    const props = {
      desktopRef: { current: desktop },
      monitors: [{ id: 'monitor', logical_bounds: selection, physical_bounds: selection, scale_factor: 1, image_base64: '' }],
      outputPixelSize: { width: 200, height: 80 },
      selection, selectionViewportRect: selection, annotations: [],
    };
    const container = document.createElement('div');
    const root = createRoot(container);
    try {
      await act(async () => root.render(createElement(CaptureAnnotationCanvas, {
        ...props, draftAnnotation: mosaic([{ x: 10, y: 10 }]),
      })));
      await act(async () => root.render(createElement(CaptureAnnotationCanvas, {
        ...props, draftAnnotation: mosaic([{ x: 10, y: 10 }, { x: 20, y: 10 }]),
      })));
      await act(async () => root.render(createElement(CaptureAnnotationCanvas, {
        ...props, draftAnnotation: mosaic([{ x: 10, y: 10 }, { x: 20, y: 10 }, { x: 30, y: 10 }]),
      })));

      expect(mosaicBufferContext.drawImage).toHaveBeenCalledTimes(3);
      expect(mosaicBufferContext.drawImage.mock.calls.map((call) => call[3])).toEqual([4, 14, 14]);
    } finally {
      await act(async () => root.unmount());
    }
  });
});
