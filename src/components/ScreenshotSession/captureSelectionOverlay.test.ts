import { describe, expect, it } from 'vitest';
import {
  drawCaptureSelectionOverlayFrame,
  getCaptureSelectionOverlayFrame,
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
      label: '120 x 61',
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
      label: '50 x 40',
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
      label: '50 x 41',
    });
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
        label: '120 x 60',
      },
    );

    expect(context.calls).toEqual([
      'clearRect:0,0,500,300',
      'fillStyle:rgba(0, 0, 0, 0.14)',
      'fillRect:0,0,500,40',
      'fillRect:0,100,500,200',
      'fillRect:0,40,80,60',
      'fillRect:200,40,300,60',
      'fillStyle:rgba(255, 255, 255, 0.05)',
      'fillRect:80,40,120,60',
      'strokeStyle:rgba(255, 255, 255, 0.9)',
      'lineWidth:2',
      'strokeRect:80.5,40.5,119,59',
      'font:12px sans-serif',
      'textBaseline:top',
      'fillStyle:rgba(0, 0, 0, 0.82)',
      'fillRect:80,12,64,20',
      'fillStyle:rgba(255, 255, 255, 0.95)',
      'fillText:120 x 60,88,16',
    ]);
  });
});
