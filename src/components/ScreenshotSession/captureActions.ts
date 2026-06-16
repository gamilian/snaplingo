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

export function isSaveCaptureShortcut(event: CaptureShortcutEvent) {
  return event.key.toLowerCase() === 's' && (event.metaKey || event.ctrlKey);
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
