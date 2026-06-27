import type { CaptureLaunch, CaptureMode } from './types';

export const CAPTURE_WINDOW_LABEL = 'capture';

export function isCaptureMode(value: unknown): value is CaptureMode {
  return (
    value === 'screenshot' ||
    value === 'screenshot-copy' ||
    value === 'screenshot-ocr' ||
    value === 'silent-screenshot-ocr' ||
    value === 'screenshot-translate'
  );
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

export function parseCaptureLaunchPayload(payload: unknown): CaptureLaunch | null {
  if (isCaptureMode(payload)) {
    return { mode: payload };
  }

  if (!payload || typeof payload !== 'object') {
    return null;
  }

  const candidate = payload as { mode?: unknown; sessionId?: unknown };
  if (!isCaptureMode(candidate.mode)) {
    return null;
  }

  return {
    mode: candidate.mode,
    sessionId:
      typeof candidate.sessionId === 'string' ? candidate.sessionId : undefined,
  };
}
