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
