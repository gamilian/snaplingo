import type { AnnotationCommand, LogicalRect, Point } from './types';

export type CaptureInvokeArgs = Record<string, unknown>;
export type CaptureInvoke = <T>(
  command: string,
  args?: CaptureInvokeArgs,
) => Promise<T>;

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

export function isSaveCaptureShortcut(event: CaptureShortcutEvent) {
  return event.key.toLowerCase() === 's' && (event.metaKey || event.ctrlKey);
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
    (event.key.toLowerCase() === 'c' && (event.metaKey || event.ctrlKey))
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
