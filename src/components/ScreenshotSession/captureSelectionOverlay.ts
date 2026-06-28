import type { LogicalRect } from './types';
import { virtualRectToViewportRect } from './virtualDesktop';

type CaptureSelectionOverlayStatus = 'idle' | 'loading' | 'selecting' | 'preview' | 'error';

export type CaptureSelectionOverlayVariant = 'draft' | 'hover' | 'preview';

export interface CaptureSelectionOverlayFrame {
  variant: CaptureSelectionOverlayVariant;
  rect: LogicalRect;
  label: string;
}

export interface CaptureSelectionOverlayFrameInput {
  status: CaptureSelectionOverlayStatus;
  selectionBounds: LogicalRect | null;
  selection: LogicalRect | null;
  draftSelection: LogicalRect | null;
  hoverSelection: LogicalRect | null;
}

export interface CaptureSelectionOverlayContext {
  fillStyle: string | CanvasGradient | CanvasPattern;
  strokeStyle: string | CanvasGradient | CanvasPattern;
  lineWidth: number;
  font: string;
  textBaseline: CanvasTextBaseline;
  clearRect(x: number, y: number, width: number, height: number): void;
  fillRect(x: number, y: number, width: number, height: number): void;
  strokeRect(x: number, y: number, width: number, height: number): void;
  fillText(text: string, x: number, y: number): void;
  measureText(text: string): { width: number };
}

interface Size {
  width: number;
  height: number;
}

const LABEL_HEIGHT = 20;
const LABEL_GAP = 8;
const LABEL_PADDING_X = 8;

export function getCaptureSelectionOverlayFrame({
  status,
  selectionBounds,
  selection,
  draftSelection,
  hoverSelection,
}: CaptureSelectionOverlayFrameInput): CaptureSelectionOverlayFrame | null {
  if (!selectionBounds) return null;

  if (status === 'preview' && selection) {
    return buildCaptureSelectionOverlayFrame(
      'preview',
      selection,
      selectionBounds,
    );
  }

  if (status !== 'selecting') return null;

  const activeSelection = draftSelection ?? (selection ? null : hoverSelection);
  if (!activeSelection) return null;

  return buildCaptureSelectionOverlayFrame(
    draftSelection ? 'draft' : 'hover',
    activeSelection,
    selectionBounds,
  );
}

function buildCaptureSelectionOverlayFrame(
  variant: CaptureSelectionOverlayVariant,
  selection: LogicalRect,
  selectionBounds: LogicalRect,
): CaptureSelectionOverlayFrame {
  return {
    variant,
    rect: virtualRectToViewportRect(selection, selectionBounds),
    label: `${Math.round(selection.width)} x ${Math.round(selection.height)}`,
  };
}

export function drawCaptureSelectionOverlayFrame(
  context: CaptureSelectionOverlayContext,
  size: Size,
  frame: CaptureSelectionOverlayFrame | null,
) {
  context.clearRect(0, 0, size.width, size.height);
  if (!frame) return;

  drawDimMask(context, size, frame.rect);
  drawSelectionRect(context, frame);
  drawSizeLabel(context, frame);
}

function drawDimMask(
  context: CaptureSelectionOverlayContext,
  size: Size,
  rect: LogicalRect,
) {
  const rectRight = rect.x + rect.width;
  const rectBottom = rect.y + rect.height;

  context.fillStyle = 'rgba(0, 0, 0, 0.18)';
  context.fillRect(0, 0, size.width, rect.y);
  context.fillRect(0, rectBottom, size.width, size.height - rectBottom);
  context.fillRect(0, rect.y, rect.x, rect.height);
  context.fillRect(rectRight, rect.y, size.width - rectRight, rect.height);
}

function drawSelectionRect(
  context: CaptureSelectionOverlayContext,
  frame: CaptureSelectionOverlayFrame,
) {
  const { rect } = frame;

  context.fillStyle = 'rgba(255, 255, 255, 0.05)';
  if (frame.variant !== 'preview') {
    context.fillRect(rect.x, rect.y, rect.width, rect.height);
  }
  context.strokeStyle = getSelectionStrokeStyle(frame.variant);
  context.lineWidth = getSelectionLineWidth(frame.variant);
  context.strokeRect(
    rect.x + 0.5,
    rect.y + 0.5,
    Math.max(0, rect.width - 1),
    Math.max(0, rect.height - 1),
  );
}

function getSelectionStrokeStyle(variant: CaptureSelectionOverlayVariant) {
  if (variant === 'preview') return 'rgba(91, 127, 255, 0.95)';
  if (variant === 'draft') return 'rgba(255, 255, 255, 0.9)';
  return 'rgba(255, 255, 255, 0.78)';
}

function getSelectionLineWidth(variant: CaptureSelectionOverlayVariant) {
  if (variant === 'preview') return 3;
  if (variant === 'draft') return 2;
  return 1;
}

function drawSizeLabel(
  context: CaptureSelectionOverlayContext,
  frame: CaptureSelectionOverlayFrame,
) {
  const { rect } = frame;

  context.font = '12px sans-serif';
  context.textBaseline = 'top';

  const labelWidth = Math.ceil(context.measureText(frame.label).width) + LABEL_PADDING_X * 2;
  const labelX =
    frame.variant === 'preview'
      ? rect.x + rect.width / 2 - labelWidth / 2
      : rect.x;
  const labelY =
    frame.variant === 'preview'
      ? rect.y + rect.height / 2 - LABEL_HEIGHT / 2
      : Math.max(0, rect.y - LABEL_HEIGHT - LABEL_GAP);

  context.fillStyle = 'rgba(0, 0, 0, 0.82)';
  context.fillRect(labelX, labelY, labelWidth, LABEL_HEIGHT);
  context.fillStyle = 'rgba(255, 255, 255, 0.95)';
  context.fillText(frame.label, labelX + LABEL_PADDING_X, labelY + 4);
}
