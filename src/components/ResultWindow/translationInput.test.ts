import { describe, expect, it } from 'vitest';
import { parseInputTranslationPayload } from './translationInput';

describe('translation input events', () => {
  it('keeps legacy string payloads as manual translation input', () => {
    expect(parseInputTranslationPayload('hello')).toEqual({
      text: 'hello',
      autoTranslate: false,
    });
  });

  it('reads screenshot translation payloads with auto translation enabled', () => {
    expect(
      parseInputTranslationPayload({
        text: 'hello',
        autoTranslate: true,
      }),
    ).toEqual({
      text: 'hello',
      autoTranslate: true,
    });
  });

  it('ignores malformed translation input payloads', () => {
    expect(parseInputTranslationPayload(null)).toBeNull();
    expect(parseInputTranslationPayload({ text: 42, autoTranslate: true })).toBeNull();
  });
});
