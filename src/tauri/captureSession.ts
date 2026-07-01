import { invoke } from '@tauri-apps/api/core';
import type {
  AnnotationCommand,
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  OcrResult,
  Point,
} from '../components/ScreenshotSession/types';

export type CaptureOutputAction =
  | { type: 'copy' }
  | { type: 'save'; path: string }
  | { type: 'pin' };

export interface RenderCaptureOutputInput {
  sessionId: string;
  rect: LogicalRect;
  annotations: AnnotationCommand[];
  includeCursor?: boolean;
}

export interface OutputCaptureInput extends RenderCaptureOutputInput {
  action: CaptureOutputAction;
}

export interface CaptureResultWindowPayload {
  mode: 'translation' | 'ocr';
  text: string;
  autoTranslate: boolean;
  startFileOcr: boolean;
}

function captureOutputArgs(input: RenderCaptureOutputInput) {
  return {
    sessionId: input.sessionId,
    rect: input.rect,
    annotations: input.annotations,
    ...(input.includeCursor ? { includeCursor: true } : {}),
  };
}

export async function openCaptureWindow(mode: CaptureMode | string) {
  return invoke<void>('open_capture_window', { mode });
}

export async function createCaptureSession() {
  return invoke<CaptureSessionView>('create_capture_session');
}

export async function getCaptureSession(sessionId: string) {
  return invoke<CaptureSessionView>('get_capture_session', { sessionId });
}

export async function hydrateCaptureSessionSnapshots(sessionId: string) {
  return invoke<CaptureSessionView>('hydrate_capture_session_snapshots', { sessionId });
}

export function logCaptureFrontendPerf(input: {
  event: string;
  mode: CaptureMode | string;
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

export async function revealCaptureWindow() {
  return invoke<void>('reveal_capture_window');
}

export async function prepareCaptureWindowForReveal() {
  return invoke<void>('prepare_capture_window_for_reveal');
}

export async function hideCaptureWindow() {
  return invoke<void>('hide_capture_window');
}

export async function renderCaptureOutput(input: RenderCaptureOutputInput) {
  return invoke<string>('render_capture_output', captureOutputArgs(input));
}

export async function defaultCaptureSavePath() {
  return invoke<string>('default_capture_save_path');
}

export async function quickCaptureSavePath(directory?: string) {
  return invoke<string>('quick_capture_save_path', { directory });
}

export async function outputCapture(input: OutputCaptureInput) {
  return invoke<void>('output_capture', {
    ...captureOutputArgs(input),
    action: input.action,
  });
}

export async function runCaptureOcr(sessionId: string, rect: LogicalRect) {
  return invoke<OcrResult>('run_capture_ocr', { sessionId, rect });
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

export async function openCaptureOcrResultWindow(text: string) {
  return invoke<void>('open_capture_ocr_result_window', { text });
}

export async function openCaptureTranslationResultWindow(text: string) {
  return invoke<void>('open_capture_translation_result_window', { text });
}

export async function takeCaptureResultWindowPayload() {
  return invoke<CaptureResultWindowPayload | null>(
    'take_capture_result_window_payload',
  );
}

export async function copyTextToClipboard(text: string) {
  return invoke<void>('copy_text_to_clipboard', { text });
}

export async function triggerScreenshot() {
  return invoke<void>('trigger_screenshot');
}

export async function captureFullScreen() {
  return invoke<string>('capture_full_screen');
}
