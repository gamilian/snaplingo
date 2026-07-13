import type {
  AnnotationCommand,
  ArrowKey,
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  Point,
} from './types';
import type {
  OutputCaptureInput,
  RenderCaptureOutputInput,
} from '../../application/capture-workspace/ports';

export type CaptureImagePrinter = (
  imageBase64: string,
) => Promise<void> | void;

export interface CaptureActionClient {
  defaultCaptureSavePath: () => Promise<string | null>;
  quickCaptureSavePath: (directory?: string) => Promise<string>;
  outputCapture: (input: OutputCaptureInput) => Promise<void>;
  createCaptureSession: () => Promise<CaptureSessionView>;
  cancelCaptureSession: (sessionId: string) => Promise<void>;
  renderCaptureOutput: (input: RenderCaptureOutputInput) => Promise<string>;
}

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
interface HoverSelectionShortcutOptions {
  drafting?: boolean;
  mode?: CaptureMode;
}
export type HoverSelectionCompletionAction =
  | 'copy'
  | 'save'
  | 'quick-save'
  | 'pin'
  | 'print'
  | 'ocr'
  | 'silent-ocr'
  | 'ocr-translate';
interface RestoreLastSelectionOptions {
  status: 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
  editing?: boolean;
}
export type UndoRedoAction = 'undo' | 'redo';
export type CaptureKeyboardToolbarAction = 'toggle';
export type CaptureSelectionFlow =
  | 'preview'
  | 'copy'
  | 'ocr'
  | 'silent-ocr'
  | 'ocr-translate';
export type SaveCapturePointerAction = 'save' | 'quick-save';
export type CaptureCompletionAction =
  | 'copy'
  | 'save'
  | 'quick-save'
  | 'pin'
  | 'ocr'
  | 'silent-ocr'
  | 'ocr-translate'
  | 'print'
  | 'cancel';
export type PreviewCaptureCompletionAction = Extract<
  CaptureCompletionAction,
  'copy' | 'save' | 'quick-save' | 'pin' | 'print' | 'ocr'
>;
type PreviewCaptureShortcutCompletionAction = Exclude<
  PreviewCaptureCompletionAction,
  'ocr'
>;
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
interface CancelCaptureBlurState {
  status: 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
  isRenderingOutput?: boolean;
}

export function shouldRecordSuccessfulCaptureSelection(
  action: CaptureCompletionAction,
) {
  return (
    action === 'copy' ||
    action === 'save' ||
    action === 'quick-save' ||
    action === 'pin'
  );
}

export function getCaptureSelectionFlowForMode(
  mode: CaptureMode,
): CaptureSelectionFlow {
  if (mode === 'screenshot-copy') return 'copy';
  if (mode === 'screenshot-ocr') return 'ocr';
  if (mode === 'silent-screenshot-ocr') return 'silent-ocr';
  if (mode === 'screenshot-translate') return 'ocr-translate';
  return 'preview';
}

function getPrimaryHoverSelectionCompletionActionForMode(
  mode: CaptureMode = 'screenshot',
): HoverSelectionCompletionAction {
  const flow = getCaptureSelectionFlowForMode(mode);
  if (flow === 'ocr' || flow === 'silent-ocr' || flow === 'ocr-translate') {
    return flow;
  }
  return 'copy';
}

export function isSaveCaptureShortcut(event: CaptureShortcutEvent) {
  return (
    event.key.toLowerCase() === 's' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function getSaveCapturePointerAction(
  event: CapturePointerEvent,
): SaveCapturePointerAction {
  if (
    event.button === 0 &&
    event.shiftKey &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey
  ) {
    return 'quick-save';
  }

  return 'save';
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

export function shouldRestoreLastSelectionFromShortcut(
  event: CaptureShortcutEvent,
  options: RestoreLastSelectionOptions,
) {
  if (options.editing) return false;
  if (options.status !== 'selecting' && options.status !== 'preview') return false;

  return isRestoreLastSelectionShortcut(event);
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
    event.key.toLowerCase() === 'z' &&
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

  if (event.key.toLowerCase() === 'z') return event.shiftKey ? null : 'undo';
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
  return (
    event.key.toLowerCase() === 'c' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isPlainCaptureCompletionShortcut(event: CaptureShortcutEvent) {
  return (
    event.key === 'Enter' &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function shouldCopyHoverSelectionFromShortcut(
  event: CaptureShortcutEvent,
  options: HoverSelectionShortcutOptions = {},
) {
  return getHoverSelectionCompletionActionFromShortcut(event, options) === 'copy';
}

export function getHoverSelectionCompletionActionFromShortcut(
  event: CaptureShortcutEvent,
  options: HoverSelectionShortcutOptions = {},
): HoverSelectionCompletionAction | null {
  if (options.drafting) return null;

  if (isPlainCaptureCompletionShortcut(event)) {
    return getPrimaryHoverSelectionCompletionActionForMode(options.mode);
  }
  if (isCopyCaptureKeyboardShortcut(event)) return 'copy';
  if (isQuickSaveCaptureShortcut(event)) return 'quick-save';
  if (isSaveCaptureShortcut(event)) return 'save';
  if (isPinCaptureShortcut(event)) return 'pin';
  if (isPrintCaptureShortcut(event)) return 'print';

  return null;
}

export function getPreviewCaptureCompletionActionFromShortcut(
  event: CaptureShortcutEvent,
): PreviewCaptureShortcutCompletionAction | null {
  if (isCopyCaptureKeyboardShortcut(event)) return 'copy';
  if (isQuickSaveCaptureShortcut(event)) return 'quick-save';
  if (isSaveCaptureShortcut(event)) return 'save';
  if (isPinCaptureShortcut(event)) return 'pin';
  if (isPrintCaptureShortcut(event)) return 'print';

  return null;
}

export function getCaptureKeyboardToolbarAction(
  event: CaptureShortcutEvent,
  _toolbarVisible = false,
): CaptureKeyboardToolbarAction | null {
  if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) {
    return null;
  }

  if (event.key === ' ') return 'toggle';

  return null;
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

export function getHoverSelectionCompletionActionFromPointer(
  event: CapturePointerEvent,
  options: HoverSelectionShortcutOptions = {},
): HoverSelectionCompletionAction | null {
  if (options.drafting) return null;
  if (!isUnmodifiedPrimaryClick(event)) return null;

  return getPrimaryHoverSelectionCompletionActionForMode(options.mode);
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

function isUnmodifiedPrimaryClick(event: CapturePointerEvent) {
  return (
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
  if (
    (state.status === 'selecting' || state.status === 'preview') &&
    state.hasSelection
  ) {
    return 'reset-selection';
  }

  return 'cancel-session';
}

export function shouldCancelCaptureOnBlur(state: CancelCaptureBlurState) {
  if (state.isRenderingOutput) return false;

  return state.status === 'selecting' || state.status === 'preview';
}

export async function saveCaptureSelection(
  sessionId: string,
  rect: LogicalRect,
  annotations: AnnotationCommand[] = [],
  includeCursor = false,
  client: CaptureActionClient,
) {
  const path = await client.defaultCaptureSavePath();
  if (!path) return;

  await client.outputCapture({
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
  sessionId: string,
  rect: LogicalRect,
  annotations: AnnotationCommand[] = [],
  directory: string | undefined,
  includeCursor = false,
  client: CaptureActionClient,
) {
  const path = await client.quickCaptureSavePath(directory);

  await client.outputCapture({
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
  previousSessionId: string,
  client: CaptureActionClient,
) {
  const session = await client.createCaptureSession();
  await client.cancelCaptureSession(previousSessionId);
  return session;
}

export async function copyCaptureSelection(
  sessionId: string,
  rect: LogicalRect,
  annotations: AnnotationCommand[] = [],
  includeCursor = false,
  client: CaptureActionClient,
) {
  await client.outputCapture({
    sessionId,
    rect,
    annotations,
    ...(includeCursor ? { includeCursor } : {}),
    action: { type: 'copy' },
  });
}

export async function printCaptureSelection(
  sessionId: string,
  rect: LogicalRect,
  annotations: AnnotationCommand[],
  printImage: CaptureImagePrinter,
  includeCursor = false,
  client: CaptureActionClient,
) {
  const imageBase64 = await client.renderCaptureOutput({
    sessionId,
    rect,
    annotations,
    ...(includeCursor ? { includeCursor } : {}),
  });

  await printImage(imageBase64);
}
