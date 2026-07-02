const CHINESE_TEXT_PATTERN = /[\u3400-\u9fff\uf900-\ufaff]/;

function isChineseLanguage(lang: string) {
  return lang === 'zh-CN' || lang === 'zh-TW' || lang === 'zh';
}

function inferredSourceLanguage(text: string, sourceLang: string) {
  if (sourceLang !== 'auto') return sourceLang;
  return CHINESE_TEXT_PATTERN.test(text) ? 'zh-CN' : 'en';
}

export function resolveTranslationRequestLanguages(
  text: string,
  sourceLang: string,
  targetLang: string,
) {
  const inferredSource = inferredSourceLanguage(text, sourceLang);
  let resolvedTarget = targetLang === 'auto' ? 'zh-CN' : targetLang;

  if (isChineseLanguage(inferredSource) && isChineseLanguage(resolvedTarget)) {
    resolvedTarget = 'en';
  } else if (inferredSource === 'en' && resolvedTarget === 'en') {
    resolvedTarget = 'zh-CN';
  }

  return {
    sourceLang,
    targetLang: resolvedTarget,
  };
}
