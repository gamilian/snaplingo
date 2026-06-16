import type { AnnotationCommand, LogicalRect } from './types';

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

export function isToggleToolbarShortcut(event: CaptureShortcutEvent) {
  return (
    event.key === 'Tab' &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isPinCaptureShortcut(event: CaptureShortcutEvent) {
  return (
    event.key === 'F3' &&
    !event.metaKey &&
    !event.ctrlKey &&
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
