import { describe, expect, it } from 'vitest';
import {
  ocrPayloadDisplayText,
  shouldApplyOcrPayloadText,
  shouldClearOcrResultsForPayload,
  shouldApplyTranslationPayloadText,
  shouldClearTranslationResultsForPayload,
  shouldStartFileOcrForPayload,
  translationPayloadSourceText,
} from './payload';

describe('result payload application', () => {
  it('does not clear existing translation content for empty manual opens', () => {
    expect(
      shouldApplyTranslationPayloadText({
        mode: 'translation',
        text: '',
        autoTranslate: false,
      }),
    ).toBe(false);
  });

  it('applies new translation payloads that carry text or request auto translation', () => {
    expect(
      shouldApplyTranslationPayloadText({
        mode: 'translation',
        text: 'hello',
        autoTranslate: false,
      }),
    ).toBe(true);
    expect(
      shouldApplyTranslationPayloadText({
        mode: 'translation',
        text: 'hello',
        autoTranslate: true,
      }),
    ).toBe(true);
  });

  it('clears previous results for new auto translation payloads only', () => {
    expect(
      shouldClearTranslationResultsForPayload({
        mode: 'translation',
        text: 'new selected text',
        autoTranslate: true,
      }),
    ).toBe(true);
    expect(
      shouldClearTranslationResultsForPayload({
        mode: 'translation',
        text: '',
        autoTranslate: false,
      }),
    ).toBe(false);
  });

  it('applies OCR text only for display-text intent', () => {
    expect(
      shouldApplyOcrPayloadText({
        mode: 'ocr',
        text: 'recognized text',
        autoTranslate: false,
        ocrIntent: 'display-text',
      }),
    ).toBe(true);
  });

  it('normalizes OCR display text before putting it into the result window', () => {
    expect(
      ocrPayloadDisplayText({
        mode: 'ocr',
        text: 'Visit https://example.\ncom',
        autoTranslate: false,
        ocrIntent: 'display-text',
      }),
    ).toBe('Visit https://example.com');
  });

  it('preserves screenshot OCR line breaks when configured', () => {
    expect(
      ocrPayloadDisplayText(
        {
          mode: 'ocr',
          origin: 'ocr',
          text: 'first line\nsecond line',
          autoTranslate: false,
          ocrIntent: 'display-text',
        },
        { preserveFormatting: true, removeChineseSpaces: true },
      ),
    ).toBe('first line\nsecond line');
  });

  it('preserves screenshot translation line breaks when configured', () => {
    expect(
      translationPayloadSourceText(
        {
          mode: 'translation',
          origin: 'screenshot',
          text: 'first line\nsecond line',
          autoTranslate: true,
        },
        { preserveFormatting: true, removeChineseSpaces: true },
      ),
    ).toBe('first line\nsecond line');
  });

  it('clears OCR results for display and uploaded-image OCR work', () => {
    expect(
      shouldClearOcrResultsForPayload({
        mode: 'ocr',
        text: 'recognized text',
        autoTranslate: false,
        ocrIntent: 'display-text',
      }),
    ).toBe(true);
    expect(
      shouldClearOcrResultsForPayload({
        mode: 'ocr',
        text: '',
        autoTranslate: false,
        ocrIntent: 'file',
      }),
    ).toBe(true);
  });

  it('starts file OCR only for file intent', () => {
    expect(
      shouldStartFileOcrForPayload({
        mode: 'ocr',
        text: '',
        autoTranslate: false,
        ocrIntent: 'file',
      }),
    ).toBe(true);
  });
});
