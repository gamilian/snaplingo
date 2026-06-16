import type { LogicalRect } from './types';

export type CaptureInvokeArgs = Record<string, unknown>;
export type CaptureInvoke = <T>(
  command: string,
  args?: CaptureInvokeArgs,
) => Promise<T>;

export async function saveCaptureSelection(
  invoke: CaptureInvoke,
  sessionId: string,
  rect: LogicalRect,
) {
  const path = await invoke<string>('default_capture_save_path');

  await invoke('output_capture', {
    sessionId,
    rect,
    action: {
      type: 'save',
      path,
    },
  });
}
