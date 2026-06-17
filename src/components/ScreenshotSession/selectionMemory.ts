import type { LogicalRect } from './types';

const LAST_CAPTURE_SELECTION_KEY = 'snaplingo:last-capture-selection';

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

export function saveLastCaptureSelection(
  storage: SelectionStorage,
  rect: LogicalRect,
) {
  storage.setItem(LAST_CAPTURE_SELECTION_KEY, JSON.stringify(rect));
}

export function loadLastCaptureSelection(storage: SelectionStorage) {
  try {
    const value = storage.getItem(LAST_CAPTURE_SELECTION_KEY);
    if (!value) return null;

    const rect = JSON.parse(value);
    return isLogicalRect(rect) ? rect : null;
  } catch {
    return null;
  }
}
