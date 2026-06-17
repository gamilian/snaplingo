import type {
  AnnotationCommand,
  ArrowKey,
  CaptureSessionView,
  LogicalRect,
  Point,
} from './types';

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
export type SelectionArrowActionMode = 'move' | 'expand' | 'shrink';
export interface SelectionArrowAction {
  mode: SelectionArrowActionMode;
  direction: ArrowKey;
}
interface SelectionArrowActionOptions {
  editing?: boolean;
}
interface ConfirmHoverSelectionOptions {
  drafting?: boolean;
}
export type UndoRedoAction = 'undo' | 'redo';
export type CancelCapturePointerAction =
  | 'finish-edit'
  | 'finish-annotation'
  | 'dismiss-layer'
  | 'reset-selection'
  | 'cancel-session';

interface CancelCapturePointerState {
  status: 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
  hasSelection: boolean;
  hasTextDraft?: boolean;
  hasAnnotationGesture?: boolean;
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

export function isRefreshCaptureShortcut(event: CaptureShortcutEvent) {
  return (
    event.key === 'F5' &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isToggleCapturedCursorShortcut(event: CaptureShortcutEvent) {
  if (event.metaKey || event.ctrlKey || event.altKey) return false;
  return (
    (event.key === '`' && !event.shiftKey) ||
    (event.key === '!' && !!event.shiftKey)
  );
}

export function canToggleCapturedCursor(session: CaptureSessionView | null) {
  return Boolean(session?.captured_cursor);
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
    (event.key === 'Backspace' || event.key === 'Delete') &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !!event.shiftKey
  );
}

export function getUndoRedoActionFromShortcut(
  event: CaptureShortcutEvent,
): UndoRedoAction | null {
  if (!(event.metaKey || event.ctrlKey) || event.altKey) {
    return null;
  }

  if (event.key.toLowerCase() === 'z') return event.shiftKey ? 'redo' : 'undo';
  if (event.key.toLowerCase() === 'y' && !event.shiftKey) return 'redo';

  return null;
}

export function isDeleteSelectedAnnotationShortcut(
  event: CaptureShortcutEvent,
) {
  return (
    (event.key === 'Backspace' || event.key === 'Delete') &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isUndoAnnotationGesturePointShortcut(
  event: CaptureShortcutEvent,
) {
  return (
    (event.key === 'Backspace' || event.key === 'Delete') &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function getSelectionArrowActionFromShortcut(
  event: CaptureShortcutEvent,
  options: SelectionArrowActionOptions = {},
): SelectionArrowAction | null {
  if (options.editing) return null;

  if (
    event.key !== 'ArrowUp' &&
    event.key !== 'ArrowRight' &&
    event.key !== 'ArrowDown' &&
    event.key !== 'ArrowLeft'
  ) {
    return null;
  }

  if (event.altKey) return null;

  const hasPrimaryModifier = event.metaKey || event.ctrlKey;
  if (hasPrimaryModifier && !event.shiftKey) {
    return { mode: 'expand', direction: event.key };
  }
  if (event.shiftKey && !hasPrimaryModifier) {
    return { mode: 'shrink', direction: event.key };
  }
  if (!hasPrimaryModifier && !event.shiftKey) {
    return { mode: 'move', direction: event.key };
  }

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

export function isConfirmHoverSelectionShortcut(
  event: CaptureShortcutEvent,
  options: ConfirmHoverSelectionOptions = {},
) {
  if (options.drafting) return false;

  return (
    (event.key === 'Enter' || event.key === ' ') &&
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
  return isUnmodifiedPrimaryDoubleClick(event);
}

export function isFinishAnnotationGestureDoubleClick(event: CapturePointerEvent) {
  return isUnmodifiedPrimaryDoubleClick(event);
}

function isUnmodifiedPrimaryDoubleClick(event: CapturePointerEvent) {
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
  if (state.hasTextDraft) return 'finish-edit';
  if (state.hasAnnotationGesture) return 'finish-annotation';
  if (state.hasDismissibleLayer) return 'dismiss-layer';
  if (state.status === 'preview' && state.hasSelection) return 'reset-selection';
  return 'cancel-session';
}

export async function saveCaptureSelection(
  invoke: CaptureInvoke,
  sessionId: string,
  rect: LogicalRect,
  annotations: AnnotationCommand[] = [],
  includeCursor = false,
) {
  const path = await invoke<string>('default_capture_save_path');

  await invoke('output_capture', {
    sessionId,
    rect,
    annotations,
    ...(includeCursor ? { includeCursor } : {}),
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
  includeCursor = false,
) {
  const path = await invoke<string>('quick_capture_save_path', { directory });

  await invoke('output_capture', {
    sessionId,
    rect,
    annotations,
    ...(includeCursor ? { includeCursor } : {}),
    action: {
      type: 'save',
      path,
    },
  });
}

export async function refreshCaptureSession(
  invoke: CaptureInvoke,
  previousSessionId: string,
) {
  const session = await invoke<CaptureSessionView>('create_capture_session');
  await invoke('cancel_capture_session', { sessionId: previousSessionId });
  return session;
}

export async function copyCaptureSelection(
  invoke: CaptureInvoke,
  sessionId: string,
  rect: LogicalRect,
  annotations: AnnotationCommand[] = [],
  includeCursor = false,
) {
  await invoke('output_capture', {
    sessionId,
    rect,
    annotations,
    ...(includeCursor ? { includeCursor } : {}),
    action: { type: 'copy' },
  });
}

export async function printCaptureSelection(
  invoke: CaptureInvoke,
  sessionId: string,
  rect: LogicalRect,
  annotations: AnnotationCommand[],
  printImage: CaptureImagePrinter,
  includeCursor = false,
) {
  const imageBase64 = await invoke<string>('render_capture_output', {
    sessionId,
    rect,
    annotations,
    ...(includeCursor ? { includeCursor } : {}),
  });

  await printImage(imageBase64);
}
