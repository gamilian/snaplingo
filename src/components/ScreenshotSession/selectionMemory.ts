import type { LogicalRect } from './types';

const LAST_CAPTURE_SELECTION_KEY = 'snaplingo:last-capture-selection';
const CAPTURE_SELECTION_HISTORY_KEY = 'snaplingo:capture-selection-history';
const DEFAULT_CAPTURE_SELECTION_HISTORY_LIMIT = 20;

export type SelectionHistoryStep = 'previous' | 'next';

interface SelectionStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function isLogicalRect(value: unknown): value is LogicalRect {
  if (!value || typeof value !== 'object') return false;

  const rect = value as Record<string, unknown>;
  return (
    typeof rect.x === 'number' &&
    typeof rect.y === 'number' &&
    typeof rect.width === 'number' &&
    typeof rect.height === 'number'
  );
}

function sameRect(a: LogicalRect, b: LogicalRect) {
  return (
    a.x === b.x &&
    a.y === b.y &&
    a.width === b.width &&
    a.height === b.height
  );
}

function parseLogicalRect(value: string | null) {
  if (!value) return null;

  const rect = JSON.parse(value);
  return isLogicalRect(rect) ? rect : null;
}

function parseLogicalRectArray(value: string | null) {
  if (!value) return null;

  const rects = JSON.parse(value);
  return Array.isArray(rects) ? rects.filter(isLogicalRect) : null;
}

export function saveLastCaptureSelection(
  storage: SelectionStorage,
  rect: LogicalRect,
  maxEntries = DEFAULT_CAPTURE_SELECTION_HISTORY_LIMIT,
) {
  storage.setItem(LAST_CAPTURE_SELECTION_KEY, JSON.stringify(rect));

  const history = loadCaptureSelectionHistory(storage);
  const nextHistory = [
    rect,
    ...history.filter((entry) => !sameRect(entry, rect)),
  ].slice(0, maxEntries);

  storage.setItem(CAPTURE_SELECTION_HISTORY_KEY, JSON.stringify(nextHistory));
}

export function loadLastCaptureSelection(storage: SelectionStorage) {
  try {
    return parseLogicalRect(storage.getItem(LAST_CAPTURE_SELECTION_KEY));
  } catch {
    return null;
  }
}

export function loadCaptureSelectionHistory(storage: SelectionStorage) {
  try {
    const history = parseLogicalRectArray(
      storage.getItem(CAPTURE_SELECTION_HISTORY_KEY),
    );
    if (history) return history;

    const lastSelection = loadLastCaptureSelection(storage);
    return lastSelection ? [lastSelection] : [];
  } catch {
    return [];
  }
}

export function getSelectionHistoryEntry(
  history: LogicalRect[],
  current: LogicalRect | null,
  step: SelectionHistoryStep,
) {
  if (history.length === 0) return null;
  if (!current) return history[0];

  const currentIndex = history.findIndex((entry) => sameRect(entry, current));
  if (currentIndex === -1) return history[0];

  const offset = step === 'previous' ? 1 : -1;
  const nextIndex = (currentIndex + offset + history.length) % history.length;
  return history[nextIndex];
}
