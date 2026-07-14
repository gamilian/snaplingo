import type { LogicalRect, Point } from './types';
import { virtualPointToViewportPoint, virtualRectToViewportRect } from './virtualDesktop';

type CaptureSelectionOverlayStatus = 'idle' | 'loading' | 'selecting' | 'preview' | 'error';

export type CaptureSelectionOverlayVariant = 'draft' | 'hover' | 'preview';

export interface CaptureSelectionOverlayFrame {
  variant: CaptureSelectionOverlayVariant;
  rect: LogicalRect;
  label: string | null;
}

export interface CaptureSelectionOverlayFrameInput {
  status: CaptureSelectionOverlayStatus;
  selectionBounds: LogicalRect | null;
  selection: LogicalRect | null;
  draftSelection: LogicalRect | null;
  hoverSelection: LogicalRect | null;
  showSelectionSize?: boolean;
}

export interface CaptureSelectionOverlayCursorInput {
  status: CaptureSelectionOverlayStatus;
  selectionBounds: LogicalRect | null;
  cursorPoint: Point | null;
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
const LABEL_PADDING_X = 8;
const LABEL_SELECTION_GAP = 6;
const CROSSHAIR_ARM_LENGTH = 10;
const CROSSHAIR_GAP = 3;

export function getCaptureSelectionOverlayFrame({
  status,
  selectionBounds,
  selection,
  draftSelection,
  hoverSelection,
  showSelectionSize = true,
}: CaptureSelectionOverlayFrameInput): CaptureSelectionOverlayFrame | null {
  if (!selectionBounds) return null;

  if (status === 'preview' && selection) {
    return buildCaptureSelectionOverlayFrame(
      'preview',
      selection,
      selectionBounds,
      showSelectionSize,
    );
  }

  if (status !== 'selecting') return null;

  const activeSelection = draftSelection ?? (selection ? null : hoverSelection);
  if (!activeSelection) return null;

  return buildCaptureSelectionOverlayFrame(
    draftSelection ? 'draft' : 'hover',
    activeSelection,
    selectionBounds,
    showSelectionSize,
  );
}

function buildCaptureSelectionOverlayFrame(
  variant: CaptureSelectionOverlayVariant,
  selection: LogicalRect,
  selectionBounds: LogicalRect,
  showSelectionSize: boolean,
): CaptureSelectionOverlayFrame {
  return {
    variant,
    rect: virtualRectToViewportRect(selection, selectionBounds),
    label: showSelectionSize
      ? `${Math.round(selection.width)} x ${Math.round(selection.height)} px`
      : null,
  };
}

export function getCaptureSelectionOverlayCursor({
  status,
  selectionBounds,
  cursorPoint,
}: CaptureSelectionOverlayCursorInput): Point | null {
  if (status !== 'selecting' || !selectionBounds || !cursorPoint) return null;

  return virtualPointToViewportPoint(cursorPoint, selectionBounds);
}

export function drawCaptureSelectionOverlayFrame(
  context: CaptureSelectionOverlayContext,
  size: Size,
  frame: CaptureSelectionOverlayFrame | null,
  cursor: Point | null = null,
) {
  context.clearRect(0, 0, size.width, size.height);

  if (frame) {
    drawDimMask(context, size, frame.rect);
    drawSelectionRect(context, frame);
    if (frame.label) drawSizeLabel(context, size, frame);
  }

  if (cursor) {
    drawCrosshairCursor(context, size, cursor);
  }
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
  size: Size,
  frame: CaptureSelectionOverlayFrame,
) {
  const { rect } = frame;
  const label = frame.label;
  if (!label) return;

  context.font = '500 12px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
  context.textBaseline = 'top';

  const labelWidth = Math.ceil(context.measureText(label).width) + LABEL_PADDING_X * 2;
  const labelX = snapCanvasTextPosition(
    clamp(rect.x, 0, Math.max(0, size.width - labelWidth)),
  );
  const preferredLabelY = rect.y - LABEL_HEIGHT - LABEL_SELECTION_GAP;
  const fallbackLabelY = rect.y;
  const labelY = snapCanvasTextPosition(
    clamp(
      preferredLabelY >= 0 ? preferredLabelY : fallbackLabelY,
      0,
      Math.max(0, size.height - LABEL_HEIGHT),
    ),
  );

  context.fillStyle = 'rgba(0, 0, 0, 0.82)';
  context.fillRect(labelX, labelY, labelWidth, LABEL_HEIGHT);
  context.fillStyle = 'rgba(255, 255, 255, 0.95)';
  context.fillText(
    label,
    snapCanvasTextPosition(labelX + LABEL_PADDING_X),
    snapCanvasTextPosition(labelY + 4),
  );
}

function drawCrosshairCursor(
  context: CaptureSelectionOverlayContext,
  size: Size,
  cursor: Point,
) {
  const x = clamp(Math.round(cursor.x), 0, size.width);
  const y = clamp(Math.round(cursor.y), 0, size.height);

  context.fillStyle = 'rgba(0, 0, 0, 0.82)';
  drawCrosshairLayer(context, x, y, 3);
  context.fillStyle = 'rgba(255, 255, 255, 0.96)';
  drawCrosshairLayer(context, x, y, 1);
}

function drawCrosshairLayer(
  context: CaptureSelectionOverlayContext,
  x: number,
  y: number,
  thickness: number,
) {
  const halfThickness = thickness / 2;
  const left = x - CROSSHAIR_ARM_LENGTH;
  const right = x + CROSSHAIR_GAP;
  const top = y - CROSSHAIR_ARM_LENGTH;
  const bottom = y + CROSSHAIR_GAP;
  const armLength = CROSSHAIR_ARM_LENGTH - CROSSHAIR_GAP;

  context.fillRect(left, y - halfThickness, armLength, thickness);
  context.fillRect(right, y - halfThickness, armLength, thickness);
  context.fillRect(x - halfThickness, top, thickness, armLength);
  context.fillRect(x - halfThickness, bottom, thickness, armLength);
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function snapCanvasTextPosition(value: number) {
  return Math.round(value);
}
