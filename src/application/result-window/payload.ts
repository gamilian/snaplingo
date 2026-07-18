import type { CaptureResultWindowPayload } from './ports';
import {
  applyOcrTextPreferences,
  normalizeOcrText,
} from '../../utils/ocrTextProcessing';

interface OcrTextPreferences {
  preserveFormatting: boolean;
  removeChineseSpaces: boolean;
}

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

export function ocrPayloadDisplayText(
  payload: CaptureResultWindowPayload,
  preferences?: OcrTextPreferences,
) {
  return preferences
    ? applyOcrTextPreferences(payload.text, preferences)
    : normalizeOcrText(payload.text);
}

export function translationPayloadSourceText(
  payload: CaptureResultWindowPayload,
  preferences?: OcrTextPreferences,
) {
  if (!payload.autoTranslate) return payload.text;
  if (payload.origin === 'screenshot' && preferences) {
    return applyOcrTextPreferences(payload.text, preferences);
  }
  return normalizeOcrText(payload.text);
}
