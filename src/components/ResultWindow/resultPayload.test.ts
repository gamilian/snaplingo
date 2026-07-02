import { describe, expect, it } from 'vitest';
import {
  shouldApplyTranslationPayloadText,
  shouldClearTranslationResultsForPayload,
} from './resultPayload';

describe('result payload application', () => {
  it('does not clear existing translation content for empty manual opens', () => {
    expect(
      shouldApplyTranslationPayloadText({
        mode: 'translation',
        text: '',
        autoTranslate: false,
        startFileOcr: false,
      }),
    ).toBe(false);
  });

  it('applies new translation payloads that carry text or request auto translation', () => {
    expect(
      shouldApplyTranslationPayloadText({
        mode: 'translation',
        text: 'hello',
        autoTranslate: false,
        startFileOcr: false,
      }),
    ).toBe(true);
    expect(
      shouldApplyTranslationPayloadText({
        mode: 'translation',
        text: 'hello',
        autoTranslate: true,
        startFileOcr: false,
      }),
    ).toBe(true);
  });

  it('clears previous results for new auto translation payloads only', () => {
    expect(
      shouldClearTranslationResultsForPayload({
        mode: 'translation',
        text: 'new selected text',
        autoTranslate: true,
        startFileOcr: false,
      }),
    ).toBe(true);
    expect(
      shouldClearTranslationResultsForPayload({
        mode: 'translation',
        text: '',
        autoTranslate: false,
        startFileOcr: false,
      }),
    ).toBe(false);
  });
});
