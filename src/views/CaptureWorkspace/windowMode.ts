import { CAPTURE_MODES, type CaptureLaunch, type CaptureMode } from '../../domain/capture';

export const CAPTURE_WINDOW_LABEL = 'capture';

export function isCaptureMode(value: unknown): value is CaptureMode {
  return typeof value === 'string' && CAPTURE_MODES.includes(value as CaptureMode);
}

export function readCaptureLaunch(search: string): CaptureLaunch | null {
  const params = new URLSearchParams(search);
  if (params.get('window') !== CAPTURE_WINDOW_LABEL) return null;

  const mode = params.get('mode');
  if (!isCaptureMode(mode)) return null;

  return {
    mode,
    sessionId: params.get('sessionId') || undefined,
  };
}
