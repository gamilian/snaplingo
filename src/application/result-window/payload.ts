import type { CaptureResultWindowPayload } from './ports';
import { normalizeOcrText } from '../../utils/ocrTextProcessing';

export function shouldApplyTranslationPayloadText(
  payload: CaptureResultWindowPayload,
) {
  return (
    payload.mode === 'translation' && (payload.autoTranslate || payload.text.length > 0)
  );
}

export function shouldClearTranslationResultsForPayload(
  payload: CaptureResultWindowPayload,
) {
  return payload.mode === 'translation' && payload.autoTranslate;
}

export function shouldApplyOcrPayloadText(payload: CaptureResultWindowPayload) {
  return payload.mode === 'ocr' && payload.ocrIntent === 'display-text';
}

export function shouldClearOcrResultsForPayload(
  payload: CaptureResultWindowPayload,
) {
  return (
    payload.mode === 'ocr' &&
    (payload.ocrIntent === 'display-text' || payload.ocrIntent === 'file')
  );
}

export function shouldStartFileOcrForPayload(payload: CaptureResultWindowPayload) {
  return payload.mode === 'ocr' && payload.ocrIntent === 'file';
}

export function ocrPayloadDisplayText(payload: CaptureResultWindowPayload) {
  return normalizeOcrText(payload.text);
}

export function translationPayloadSourceText(payload: CaptureResultWindowPayload) {
  return payload.autoTranslate ? normalizeOcrText(payload.text) : payload.text;
}
