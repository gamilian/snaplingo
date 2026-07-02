import type { CaptureResultWindowPayload } from '../../tauri/captureSession';

export function shouldApplyTranslationPayloadText(
  payload: CaptureResultWindowPayload,
) {
  return payload.mode === 'translation' && (payload.autoTranslate || payload.text.length > 0);
}

export function shouldClearTranslationResultsForPayload(
  payload: CaptureResultWindowPayload,
) {
  return payload.mode === 'translation' && payload.autoTranslate;
}

export function shouldApplyOcrPayloadText(payload: CaptureResultWindowPayload) {
  return payload.mode === 'ocr' && payload.ocrIntent === 'display-text';
}

export function shouldClearOcrResultsForPayload(payload: CaptureResultWindowPayload) {
  return (
    payload.mode === 'ocr' &&
    (payload.ocrIntent === 'display-text' || payload.ocrIntent === 'file')
  );
}

export function shouldStartFileOcrForPayload(payload: CaptureResultWindowPayload) {
  return payload.mode === 'ocr' && payload.ocrIntent === 'file';
}
