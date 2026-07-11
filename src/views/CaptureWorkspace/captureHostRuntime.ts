import { shouldRecordSuccessfulCaptureCompletion } from './captureInteractionModel';
import type { CaptureCompletionAction, SelectionHistoryStep } from './captureActions';
import type { CaptureSelectionOverlayFrame } from './captureSelectionOverlay';
import {
  getSelectionHistoryEntry,
  loadCaptureSelectionHistory,
  loadLastCaptureSelection,
  saveLastCaptureSelection,
} from './selectionMemory';
import { restoreSelectionWithinBounds } from './selection';
import { waitForCaptureSurfacePaint } from './captureWindowVisibility';
import type { CaptureSessionView, LogicalRect, Point } from './types';

export type CaptureSelectionStorage = Parameters<
  typeof saveLastCaptureSelection
>[0];

export interface LoadedCaptureHostSession {
  session: CaptureSessionView;
  cursorPoint: Point | null;
  hoverSelection: LogicalRect | null;
}

export function recordSuccessfulCaptureSelection(
  storage: CaptureSelectionStorage,
  action: CaptureCompletionAction,
  rect: LogicalRect,
) {
  if (!shouldRecordSuccessfulCaptureCompletion(action)) return;
  saveLastCaptureSelection(storage, rect);
}

export function restoreLastSuccessfulCaptureSelection({
  completeSelection,
  minSelectionSize,
  selectionBounds,
  storage,
}: {
  storage: CaptureSelectionStorage;
  selectionBounds: LogicalRect;
  minSelectionSize: number;
  completeSelection: (rect: LogicalRect) => void;
}) {
  const savedSelection = loadLastCaptureSelection(storage);
  if (!savedSelection) return;

  const restoredSelection = restoreSelectionWithinBounds(
    savedSelection,
    selectionBounds,
    minSelectionSize,
  );
  if (restoredSelection) completeSelection(restoredSelection);
}

export async function prepareCaptureSurfaceForReveal({
  frame,
  paintSelectionOverlayFrame,
  waitForPaint = waitForCaptureSurfacePaint,
}: {
  frame: CaptureSelectionOverlayFrame | null;
  paintSelectionOverlayFrame: (
    frame: CaptureSelectionOverlayFrame | null,
  ) => void;
  waitForPaint?: typeof waitForCaptureSurfacePaint;
}) {
  paintSelectionOverlayFrame(frame);
  await waitForPaint();
}

export function restoreCaptureSelectionFromHistory({
  completeSelection,
  currentSelection,
  minSelectionSize,
  selectionBounds,
  step,
  storage,
}: {
  storage: CaptureSelectionStorage;
  currentSelection: LogicalRect | null;
  step: SelectionHistoryStep | null;
  selectionBounds: LogicalRect;
  minSelectionSize: number;
  completeSelection: (rect: LogicalRect) => void;
}) {
  if (!step) return;

  const historySelection = getSelectionHistoryEntry(
    loadCaptureSelectionHistory(storage),
    currentSelection,
    step,
  );
  if (!historySelection) return;

  const restoredSelection = restoreSelectionWithinBounds(
    historySelection,
    selectionBounds,
    minSelectionSize,
  );
  if (restoredSelection) completeSelection(restoredSelection);
}
