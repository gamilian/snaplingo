import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import type {
  CaptureMode,
  CaptureCandidateView,
  CaptureSessionView,
  LogicalRect,
  MonitorSnapshotView,
  OcrResult,
  Point,
} from '../../domain/capture';
import type {
  CaptureWorkspaceCommandsPort,
  CaptureSavePathOptions,
  OutputCaptureInput,
  RenderCaptureOutputInput,
} from '../../application/capture-workspace/ports';
import type { CaptureResultWindowPayload } from '../../application/result-window/ports';

function captureOutputArgs(input: RenderCaptureOutputInput) {
  return {
    sessionId: input.sessionId,
    rect: input.rect,
    annotations: input.annotations,
    ...(input.includeCursor ? { includeCursor: true } : {}),
  };
}

export async function openCaptureWindow(mode: CaptureMode) {
  return invoke<void>('open_capture_window', { mode });
}

export async function createCaptureSession() {
  return invoke<CaptureSessionView>('create_capture_session');
}

export async function getCaptureSession(sessionId: string) {
  return invoke<CaptureSessionView>('get_capture_session', { sessionId });
}

export async function hydrateCaptureSessionSnapshots(sessionId: string) {
  return invoke<CaptureSessionView>('hydrate_capture_session_snapshots', {
    sessionId,
  });
}

export async function hydrateCaptureMonitorSnapshot(
  sessionId: string,
  monitorId: string,
) {
  return invoke<MonitorSnapshotView>('hydrate_capture_monitor_snapshot', {
    sessionId,
    monitorId,
  });
}

export function logCaptureFrontendPerf(input: {
  event: string;
  mode: CaptureMode;
  sessionId?: string | null;
  elapsedMs: number;
}) {
  return invoke<void>('log_capture_frontend_perf', {
    event: input.event,
    mode: input.mode,
    sessionId: input.sessionId ?? null,
    elapsedMs: input.elapsedMs,
  });
}

export async function currentCaptureCursorPosition(sessionId: string) {
  return invoke<Point | null>('current_capture_cursor_position', { sessionId });
}

export async function currentCaptureControlCandidate(
  sessionId: string,
  point: Point,
) {
  return invoke<CaptureCandidateView | null>('current_capture_control_candidate', {
    sessionId,
    point,
  });
}

export async function moveCaptureCursor(delta: Point) {
  return invoke<void>('move_capture_cursor', {
    deltaX: delta.x,
    deltaY: delta.y,
  });
}

export async function cancelCaptureSession(sessionId: string) {
  return invoke<void>('cancel_capture_session', { sessionId });
}

export async function restoreCaptureSnapshotWindowsForSession(
  sessionId: string,
) {
  return invoke<void>('restore_capture_snapshot_windows_for_session', {
    sessionId,
  });
}

export async function renderCaptureOutput(input: RenderCaptureOutputInput) {
  return invoke<string>('render_capture_output', captureOutputArgs(input));
}

function captureSavePathArgs(options?: CaptureSavePathOptions) {
  return {
    directory: options?.directory,
    format: options?.format,
    namingRule: options?.namingRule,
    customFileName: options?.customFileName,
  };
}

export async function defaultCaptureSavePath(options?: CaptureSavePathOptions) {
  return invoke<string>('default_capture_save_path', captureSavePathArgs(options));
}

export async function selectCaptureSavePath(options?: CaptureSavePathOptions) {
  const format = options?.format ?? 'png';
  const defaultPath = await defaultCaptureSavePath(options);
  return save({
    defaultPath,
    filters: [
      {
        name: format === 'jpg' ? 'JPEG image' : format === 'webp' ? 'WebP image' : 'PNG image',
        extensions: [format],
      },
    ],
  });
}

export async function quickCaptureSavePath(options?: CaptureSavePathOptions) {
  return invoke<string>('quick_capture_save_path', captureSavePathArgs(options));
}

export async function outputCapture(input: OutputCaptureInput) {
  if (input.action.type === 'favorite') {
    return invoke<void>('favorite_capture_selection', captureOutputArgs(input));
  }
  return invoke<void>('output_capture', {
    ...captureOutputArgs(input),
    action: input.action,
  });
}

export async function runCaptureOcr(sessionId: string, rect: LogicalRect, language?: string) {
  return invoke<OcrResult>('run_capture_ocr', {
    sessionId,
    rect,
    ...(language && language !== 'auto' ? { language } : {}),
  });
}

export async function openResultWindow(text: string) {
  return invoke<void>('open_result_window', { text });
}

export async function openOcrResultWindow(text: string) {
  return invoke<void>('open_ocr_result_window', { text });
}

export async function openTranslationResultWindow(text: string) {
  return invoke<void>('open_translation_result_window', { text });
}

export async function openCaptureOcrResultWindow(
  text: string,
  imageBase64?: string,
  confidence?: number,
) {
  return invoke<void>('open_capture_ocr_result_window', {
    text,
    ...(imageBase64 ? { imageBase64 } : {}),
    ...(confidence !== undefined ? { confidence } : {}),
  });
}

export async function openCaptureTranslationResultWindow(text: string) {
  return invoke<void>('open_capture_translation_result_window', { text });
}

export async function currentCaptureResultWindowRequestId() {
  return invoke<string | null>('current_capture_result_window_request_id');
}

export async function takeCaptureResultWindowPayload(requestId: string) {
  return invoke<CaptureResultWindowPayload | null>(
    'take_capture_result_window_payload',
    { requestId },
  );
}

export async function copyTextToClipboard(text: string) {
  return invoke<void>('copy_text_to_clipboard', { text });
}

export async function triggerScreenshot() {
  return invoke<void>('trigger_screenshot');
}

export const captureWorkspaceCommands: CaptureWorkspaceCommandsPort = {
  createCaptureSession,
  getCaptureSession,
  hydrateCaptureSessionSnapshots,
  hydrateCaptureMonitorSnapshot,
  logCaptureFrontendPerf,
  currentCaptureCursorPosition,
  currentCaptureControlCandidate,
  moveCaptureCursor,
  cancelCaptureSession,
  restoreCaptureSnapshotWindowsForSession,
  renderCaptureOutput,
  defaultCaptureSavePath: selectCaptureSavePath,
  quickCaptureSavePath,
  outputCapture,
  runCaptureOcr,
  openCaptureOcrResultWindow,
  openCaptureTranslationResultWindow,
  copyTextToClipboard,
};
