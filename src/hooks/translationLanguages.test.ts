import { describe, expect, it } from 'vitest';
import { resolveTranslationRequestLanguages } from './translationLanguages';

describe('translation language resolution', () => {
  it('uses English as target for auto-detected Chinese text', () => {
    expect(
      resolveTranslationRequestLanguages('你好，世界', 'auto', 'auto'),
    ).toEqual({
      sourceLang: 'auto',
      targetLang: 'en',
    });
    expect(
      resolveTranslationRequestLanguages('你好，世界', 'auto', 'zh-CN'),
    ).toEqual({
      sourceLang: 'auto',
      targetLang: 'en',
    });
  });

  it('uses Simplified Chinese as target for auto-detected English text', () => {
    expect(
      resolveTranslationRequestLanguages('hello world', 'auto', 'auto'),
    ).toEqual({
      sourceLang: 'auto',
      targetLang: 'zh-CN',
    });
  });

  it('avoids Chinese-to-Chinese and English-to-English requests', () => {
    expect(
      resolveTranslationRequestLanguages('你好', 'zh-CN', 'zh-CN'),
    ).toEqual({
      sourceLang: 'zh-CN',
      targetLang: 'en',
    });
    expect(resolveTranslationRequestLanguages('hello', 'en', 'en')).toEqual({
      sourceLang: 'en',
      targetLang: 'zh-CN',
    });
  });
});
