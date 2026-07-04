import { describe, expect, it } from 'vitest';
import {
  defaultTargetLanguageForSource,
  getTranslationLanguageDisplayName,
  getTranslationLanguageSelectLabel,
  resolveTranslationRequestLanguages,
  swapTranslationLanguagePair,
} from './translationLanguages';

describe('translation language resolution', () => {
  it('uses Chinese-first bilingual labels in the selector', () => {
    expect(getTranslationLanguageSelectLabel('auto')).toBe('自动检测 Auto Detect');
    expect(getTranslationLanguageSelectLabel('zh-CN')).toBe(
      '中文简体 Chinese (Simplified)',
    );
    expect(getTranslationLanguageSelectLabel('en')).toBe('英语 English');
    expect(getTranslationLanguageDisplayName('zh')).toBe('中文');
    expect(getTranslationLanguageDisplayName('zh-TW')).toBe('中文繁体');
  });

  it('defaults non-Chinese sources to Simplified Chinese and Chinese to English', () => {
    expect(defaultTargetLanguageForSource('auto')).toBe('zh-CN');
    expect(defaultTargetLanguageForSource('en')).toBe('zh-CN');
    expect(defaultTargetLanguageForSource('ja')).toBe('zh-CN');
    expect(defaultTargetLanguageForSource('zh-CN')).toBe('en');
    expect(defaultTargetLanguageForSource('zh-TW')).toBe('en');
  });

  it('uses a sensible target when swapping from auto detection', () => {
    expect(swapTranslationLanguagePair('auto', 'zh-CN')).toEqual({
      sourceLang: 'zh-CN',
      targetLang: 'en',
    });
    expect(swapTranslationLanguagePair('auto', 'en')).toEqual({
      sourceLang: 'en',
      targetLang: 'zh-CN',
    });
  });

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

  it('resolves auto target language from the selected source language', () => {
    expect(
      resolveTranslationRequestLanguages('hello world', 'en', 'auto'),
    ).toEqual({
      sourceLang: 'en',
      targetLang: 'zh-CN',
    });
    expect(
      resolveTranslationRequestLanguages('你好，世界', 'zh-CN', 'auto'),
    ).toEqual({
      sourceLang: 'zh-CN',
      targetLang: 'en',
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
