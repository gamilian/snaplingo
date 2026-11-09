const CHINESE_TEXT_PATTERN = /[\u3400-\u9fff\uf900-\ufaff]/;

export const TRANSLATION_LANGUAGES = [
  { code: 'auto', chineseName: '自动检测', englishName: 'Auto Detect' },
  { code: 'en', chineseName: '英语', englishName: 'English' },
  {
    code: 'zh-CN',
    chineseName: '中文简体',
    englishName: 'Chinese (Simplified)',
  },
  {
    code: 'zh-TW',
    chineseName: '中文繁体',
    englishName: 'Chinese (Traditional)',
  },
  { code: 'ja', chineseName: '日语', englishName: 'Japanese' },
  { code: 'ko', chineseName: '韩语', englishName: 'Korean' },
  { code: 'es', chineseName: '西班牙语', englishName: 'Spanish' },
  { code: 'fr', chineseName: '法语', englishName: 'French' },
  { code: 'de', chineseName: '德语', englishName: 'German' },
  { code: 'ru', chineseName: '俄语', englishName: 'Russian' },
  { code: 'ar', chineseName: '阿拉伯语', englishName: 'Arabic' },
] as const;

function languageByCode(languageCode: string) {
  if (languageCode === 'zh') {
    return { code: 'zh', chineseName: '中文', englishName: 'Chinese' };
  }
  return TRANSLATION_LANGUAGES.find(
    (language) => language.code === languageCode,
  );
}

function isChineseLanguage(lang: string) {
  return lang === 'zh-CN' || lang === 'zh-TW' || lang === 'zh';
}

function inferredSourceLanguage(text: string, sourceLang: string) {
  if (sourceLang !== 'auto') return sourceLang;
  return CHINESE_TEXT_PATTERN.test(text) ? 'zh-CN' : 'en';
}

export function getTranslationLanguageSelectLabel(languageCode: string) {
  const language = languageByCode(languageCode);
  if (!language) return languageCode;
  return `${language.chineseName} ${language.englishName}`;
}

export function getTranslationLanguageDisplayName(languageCode: string) {
  return languageByCode(languageCode)?.chineseName ?? languageCode;
}

export function defaultTargetLanguageForSource(sourceLang: string) {
  return isChineseLanguage(sourceLang) ? 'en' : 'zh-CN';
}

export function swapTranslationLanguagePair(
  sourceLang: string,
  targetLang: string,
) {
  if (sourceLang === 'auto') {
    return {
      sourceLang: targetLang,
      targetLang: defaultTargetLanguageForSource(targetLang),
    };
  }
  return { sourceLang: targetLang, targetLang: sourceLang };
}

export function resolveTranslationRequestLanguages(
  text: string,
  sourceLang: string,
  targetLang: string,
) {
  const inferredSource = inferredSourceLanguage(text, sourceLang);
  let resolvedTarget =
    targetLang === 'auto'
      ? defaultTargetLanguageForSource(inferredSource)
      : targetLang;

  if (
    (isChineseLanguage(inferredSource) && isChineseLanguage(resolvedTarget)) ||
    inferredSource === resolvedTarget
  ) {
    resolvedTarget = defaultTargetLanguageForSource(inferredSource);
  }

  return { sourceLang, targetLang: resolvedTarget };
}
