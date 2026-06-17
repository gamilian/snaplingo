import type { AnnotationCommand, LogicalRect, Point } from './types';

export type CaptureInvokeArgs = Record<string, unknown>;
export type CaptureInvoke = <T>(
  command: string,
  args?: CaptureInvokeArgs,
) => Promise<T>;
export type CaptureImagePrinter = (
  imageBase64: string,
) => Promise<void> | void;

interface CaptureShortcutEvent {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey?: boolean;
  shiftKey?: boolean;
}

interface CapturePointerEvent {
  detail?: number;
  button: number;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

export type SelectionHistoryStep = 'previous' | 'next';
export type UndoRedoAction = 'undo' | 'redo';
export type CancelCapturePointerAction =
  | 'dismiss-layer'
  | 'reset-selection'
  | 'cancel-session';

interface CancelCapturePointerState {
  status: 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
  hasSelection: boolean;
  hasDismissibleLayer: boolean;
}

export function isSaveCaptureShortcut(event: CaptureShortcutEvent) {
  return (
    event.key.toLowerCase() === 's' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isQuickSaveCaptureShortcut(event: CaptureShortcutEvent) {
  return (
    event.key.toLowerCase() === 's' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !!event.shiftKey
  );
}

export function isRestoreLastSelectionShortcut(event: CaptureShortcutEvent) {
  return (
    event.key.toLowerCase() === 'r' &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function getSelectionHistoryStepFromShortcut(
  event: CaptureShortcutEvent,
): SelectionHistoryStep | null {
  if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) {
    return null;
  }

  if (event.key === ',') return 'previous';
  if (event.key === '.') return 'next';

  return null;
}

export function isClearAnnotationsShortcut(event: CaptureShortcutEvent) {
  return (
    event.key.toLowerCase() === 'z' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !!event.shiftKey
  );
}

export function getUndoRedoActionFromShortcut(
  event: CaptureShortcutEvent,
): UndoRedoAction | null {
  if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) {
    return null;
  }

  if (event.key.toLowerCase() === 'z') return 'undo';
  if (event.key.toLowerCase() === 'y') return 'redo';

  return null;
}

export function isCopyCaptureKeyboardShortcut(event: CaptureShortcutEvent) {
  const isPlainEnter =
    event.key === 'Enter' &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey;

  return (
    isPlainEnter ||
    (event.key.toLowerCase() === 'c' &&
      (event.metaKey || event.ctrlKey) &&
      !event.altKey &&
      !event.shiftKey)
  );
}

export function isConfirmHoverSelectionShortcut(event: CaptureShortcutEvent) {
  return (
    event.key === 'Enter' &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isToggleToolbarShortcut(event: CaptureShortcutEvent) {
  return (
    event.key === ' ' &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isMoveDraftSelectionShortcut(event: CaptureShortcutEvent) {
  return (
    event.key === ' ' &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey
  );
}

export function isPinCaptureShortcut(event: CaptureShortcutEvent) {
  return (
    event.key.toLowerCase() === 't' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isPrintCaptureShortcut(event: CaptureShortcutEvent) {
  return (
    event.key.toLowerCase() === 'p' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isMagnifierShortcut(event: CaptureShortcutEvent) {
  return (
    event.key === 'Alt' &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.shiftKey
  );
}

export function getCursorNudgeDeltaFromShortcut(
  event: CaptureShortcutEvent,
): Point | null {
  if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return null;

  const deltaByKey: Record<string, Point> = {
    w: { x: 0, y: -1 },
    a: { x: -1, y: 0 },
    s: { x: 0, y: 1 },
    d: { x: 1, y: 0 },
  };

  return deltaByKey[event.key.toLowerCase()] ?? null;
}

export function getCandidateCycleDirectionFromShortcut(
  event: CaptureShortcutEvent,
): 1 | -1 | null {
  if (event.key !== 'Tab' || event.metaKey || event.ctrlKey || event.altKey) {
    return null;
  }

  return event.shiftKey ? -1 : 1;
}

export function isSelectAllCaptureShortcut(event: CaptureShortcutEvent) {
  return (
    event.key.toLowerCase() === 'a' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isCopyCaptureDoubleClick(event: CapturePointerEvent) {
  return (
    (event.detail ?? 0) >= 2 &&
    event.button === 0 &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isPinCapturePointer(event: CapturePointerEvent) {
  return (
    event.button === 1 &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isCancelCapturePointer(event: CapturePointerEvent) {
  return (
    event.button === 2 &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function getCancelCapturePointerAction(
  state: CancelCapturePointerState,
): CancelCapturePointerAction {
  if (state.hasDismissibleLayer) return 'dismiss-layer';
  if (state.status === 'preview' && state.hasSelection) return 'reset-selection';
  return 'cancel-session';
}

export async function saveCaptureSelection(
  invoke: CaptureInvoke,
  sessionId: string,
  rect: LogicalRect,
  annotations: AnnotationCommand[] = [],
) {
  const path = await invoke<string>('default_capture_save_path');

  await invoke('output_capture', {
    sessionId,
    rect,
    annotations,
    action: {
      type: 'save',
      path,
    },
  });
}

export async function quickSaveCaptureSelection(
  invoke: CaptureInvoke,
  sessionId: string,
  rect: LogicalRect,
  annotations: AnnotationCommand[] = [],
  directory?: string,
) {
  const path = await invoke<string>('quick_capture_save_path', { directory });

  await invoke('output_capture', {
    sessionId,
    rect,
    annotations,
    action: {
      type: 'save',
      path,
    },
  });
}

export async function copyCaptureSelection(
  invoke: CaptureInvoke,
  sessionId: string,
  rect: LogicalRect,
  annotations: AnnotationCommand[] = [],
) {
  await invoke('output_capture', {
    sessionId,
    rect,
    annotations,
    action: { type: 'copy' },
  });
}

export async function printCaptureSelection(
  invoke: CaptureInvoke,
  sessionId: string,
  rect: LogicalRect,
  annotations: AnnotationCommand[],
  printImage: CaptureImagePrinter,
) {
  const imageBase64 = await invoke<string>('render_capture_output', {
    sessionId,
    rect,
    annotations,
  });

  await printImage(imageBase64);
}
