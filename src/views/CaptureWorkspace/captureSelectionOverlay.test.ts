import { describe, expect, it } from 'vitest';
import {
  drawCaptureSelectionOverlayFrame,
  getCaptureSelectionOverlayFrame,
  getCaptureOutputPixelSize,
  type CaptureSelectionOverlayContext,
} from './captureSelectionOverlay';
import type { LogicalRect } from './types';

const bounds: LogicalRect = { x: -100, y: 50, width: 500, height: 300 };

function createRecordingContext(): CaptureSelectionOverlayContext & {
  calls: string[];
} {
  const calls: string[] = [];
  return {
    calls,
    set fillStyle(value: string) {
      calls.push(`fillStyle:${value}`);
    },
    set strokeStyle(value: string) {
      calls.push(`strokeStyle:${value}`);
    },
    set lineWidth(value: number) {
      calls.push(`lineWidth:${value}`);
    },
    set font(value: string) {
      calls.push(`font:${value}`);
    },
    set textBaseline(value: CanvasTextBaseline) {
      calls.push(`textBaseline:${value}`);
    },
    clearRect(x, y, width, height) {
      calls.push(`clearRect:${x},${y},${width},${height}`);
    },
    fillRect(x, y, width, height) {
      calls.push(`fillRect:${x},${y},${width},${height}`);
    },
    strokeRect(x, y, width, height) {
      calls.push(`strokeRect:${x},${y},${width},${height}`);
    },
    fillText(text, x, y) {
      calls.push(`fillText:${text},${x},${y}`);
    },
    measureText(text) {
      return { width: text.length * 6 };
    },
  };
}

describe('capture selection canvas overlay', () => {
  it('builds a viewport draft frame with the current selection label', () => {
    expect(
      getCaptureSelectionOverlayFrame({
        status: 'selecting',
        selectionBounds: bounds,
        selection: null,
        draftSelection: { x: -20, y: 90, width: 120.4, height: 60.6 },
        hoverSelection: null,
      }),
    ).toEqual({
      variant: 'draft',
      rect: { x: 80, y: 40, width: 120.4, height: 60.6 },
      label: '121 x 61 px',
    });
  });

  it('uses hover selection only while no draft selection is active', () => {
    expect(
      getCaptureSelectionOverlayFrame({
        status: 'selecting',
        selectionBounds: bounds,
        selection: null,
        draftSelection: null,
        hoverSelection: { x: 0, y: 80, width: 50, height: 40 },
      }),
    ).toEqual({
      variant: 'hover',
      rect: { x: 100, y: 30, width: 50, height: 40 },
      label: '50 x 40 px',
    });
  });

  it('builds a viewport preview frame from the committed selection', () => {
    expect(
      getCaptureSelectionOverlayFrame({
        status: 'preview',
        selectionBounds: bounds,
        selection: { x: 0, y: 80, width: 50.4, height: 40.6 },
        draftSelection: null,
        hoverSelection: null,
      }),
    ).toEqual({
      variant: 'preview',
      rect: { x: 100, y: 30, width: 50.4, height: 40.6 },
      label: '51 x 41 px',
    });
  });

  it('reports physical output pixels on a scaled monitor', () => {
    expect(
      getCaptureSelectionOverlayFrame({
        status: 'selecting',
        selectionBounds: bounds,
        selection: null,
        draftSelection: { x: 0, y: 80, width: 50.4, height: 40.6 },
        hoverSelection: null,
        monitors: [
          {
            id: 'scaled',
            logical_bounds: { x: 0, y: 50, width: 250, height: 150 },
            physical_bounds: { x: 0, y: 0, width: 1500, height: 900 },
            scale_factor: 1.5,
            image_base64: '',
          },
        ],
      }),
    ).toMatchObject({
      label: '76 x 61 px',
    });
  });

  it('uses the highest scale among intersected monitors for cross-monitor output', () => {
    expect(
      getCaptureSelectionOverlayFrame({
        status: 'preview',
        selectionBounds: bounds,
        selection: { x: 80, y: 80, width: 40, height: 30 },
        draftSelection: null,
        hoverSelection: null,
        monitors: [
          {
            id: 'left',
            logical_bounds: { x: -100, y: 50, width: 200, height: 150 },
            physical_bounds: { x: -200, y: 0, width: 400, height: 300 },
            scale_factor: 1,
            image_base64: '',
          },
          {
            id: 'right',
            logical_bounds: { x: 100, y: 50, width: 200, height: 150 },
            physical_bounds: { x: 200, y: 0, width: 600, height: 450 },
            scale_factor: 2,
            image_base64: '',
          },
          {
            id: 'unselected',
            logical_bounds: { x: 300, y: 50, width: 200, height: 150 },
            physical_bounds: { x: 800, y: 0, width: 1200, height: 900 },
            scale_factor: 3,
            image_base64: '',
          },
        ],
      }),
    ).toMatchObject({
      label: '80 x 60 px',
    });
  });

  it('uses the session-wide output scale even on a lower-density monitor', () => {
    expect(getCaptureOutputPixelSize(
      { x: -100, y: 50, width: 100, height: 80 },
      [{
        id: 'secondary',
        logical_bounds: { x: -100, y: 50, width: 100, height: 80 },
        physical_bounds: { x: -100, y: 50, width: 100, height: 80 },
        scale_factor: 1,
        image_base64: '',
      }],
      1.5,
    )).toEqual({ width: 150, height: 120 });
  });

  it('does not build a frame outside selecting or preview state', () => {
    expect(
      getCaptureSelectionOverlayFrame({
        status: 'loading',
        selectionBounds: bounds,
        selection: null,
        draftSelection: { x: 0, y: 80, width: 50, height: 40 },
        hoverSelection: null,
      }),
    ).toBeNull();
  });

  it('draws a clear canvas when there is no active frame', () => {
    const context = createRecordingContext();

    drawCaptureSelectionOverlayFrame(context, { width: 500, height: 300 }, null);

    expect(context.calls).toEqual(['clearRect:0,0,500,300']);
  });

  it('draws dim mask, selection border, and size label onto the canvas', () => {
    const context = createRecordingContext();

    drawCaptureSelectionOverlayFrame(
      context,
      { width: 500, height: 300 },
      {
        variant: 'draft',
        rect: { x: 80, y: 40, width: 120, height: 60 },
        label: '120 x 60 px',
      },
    );

    expect(context.calls).toEqual([
      'clearRect:0,0,500,300',
      'fillStyle:rgba(0, 0, 0, 0.18)',
      'fillRect:0,0,500,40',
      'fillRect:0,100,500,200',
      'fillRect:0,40,80,60',
      'fillRect:200,40,300,60',
      'fillStyle:rgba(255, 255, 255, 0.05)',
      'fillRect:80,40,120,60',
      'strokeStyle:rgba(255, 255, 255, 0.9)',
      'lineWidth:2',
      'strokeRect:80.5,40.5,119,59',
      'font:500 12px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
      'textBaseline:top',
      'fillStyle:rgba(0, 0, 0, 0.82)',
      'fillRect:80,14,82,20',
      'fillStyle:rgba(255, 255, 255, 0.95)',
      'fillText:120 x 60 px,88,18',
    ]);
  });

  it('uses the configured selection mask, border width, and border color', () => {
    const context = createRecordingContext();

    drawCaptureSelectionOverlayFrame(
      context,
      { width: 500, height: 300 },
      {
        variant: 'draft',
        rect: { x: 80, y: 40, width: 120, height: 60 },
        label: null,
      },
      {
        borderWidth: 4,
        borderColor: [91, 127, 255, 242],
        maskColor: [32, 36, 44, 72],
      },
    );

    expect(context.calls).toContain(
      `fillStyle:rgba(32, 36, 44, ${72 / 255})`,
    );
    expect(context.calls).toContain(
      `strokeStyle:rgba(91, 127, 255, ${242 / 255})`,
    );
    expect(context.calls).toContain('lineWidth:4');
  });

  it('draws the preview size label above the selection top left', () => {
    const context = createRecordingContext();

    drawCaptureSelectionOverlayFrame(
      context,
      { width: 500, height: 300 },
      {
        variant: 'preview',
        rect: { x: 100, y: 30, width: 50, height: 40 },
        label: '50 x 40 px',
      },
    );

    expect(context.calls).toContain('fillRect:100,4,76,20');
    expect(context.calls).toContain('fillText:50 x 40 px,108,8');
  });

  it('keeps the size label inside the selection when the top edge is too close to the screen', () => {
    const context = createRecordingContext();

    drawCaptureSelectionOverlayFrame(
      context,
      { width: 500, height: 300 },
      {
        variant: 'preview',
        rect: { x: 100, y: 12, width: 50, height: 40 },
        label: '50 x 40 px',
      },
    );

    expect(context.calls).toContain('fillRect:100,12,76,20');
    expect(context.calls).toContain('fillText:50 x 40 px,108,16');
  });
});
