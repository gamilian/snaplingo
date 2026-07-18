import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import TranslationCard from './TranslationCard';

describe('translation card presentation', () => {
  it('keeps provider results compact enough to show multiple active providers', () => {
    const markup = renderToStaticMarkup(
      <TranslationCard
        providerId="google"
        providerName="Google Translate"
        status="success"
        text="First line\nSecond line\nThird line"
        bodyHeightPx={144}
        copyText={async () => undefined}
        speakText={async () => undefined}
      />,
    );

    expect(markup).toContain('rounded-[14px]');
    expect(markup).toContain('px-3');
    expect(markup).not.toContain('px-2.5');
    expect(markup).toContain('py-2');
    expect(markup).not.toContain('py-3');
    expect(markup).toContain('leading-[1.38]');
    expect(markup).not.toContain('height:144px');
    expect(markup).not.toContain('max-height:144px');
    expect(markup).not.toContain('overflow-y-auto');
  });

  it('fills the allocated body height while a provider is still loading', () => {
    const markup = renderToStaticMarkup(
      <TranslationCard
        providerId="openai"
        providerName="gpt-5-mini"
        status="pending"
        text=""
        bodyHeightPx={62}
        copyText={async () => undefined}
        speakText={async () => undefined}
      />,
    );

    expect(markup).toContain('style="height:62px;max-height:62px');
    expect(markup).toContain('aria-label="翻译中"');
    expect(markup).toContain('py-2');
    expect(markup).not.toContain('py-3');
  });

  it('does not repeat detected language in provider card headers', () => {
    const markup = renderToStaticMarkup(
      <TranslationCard
        providerId="google"
        providerName="Google Translate"
        status="success"
        text="你好"
        detectedLanguage="en"
        bodyHeightPx={80}
        copyText={async () => undefined}
        speakText={async () => undefined}
      />,
    );

    expect(markup).not.toContain('Detected: en');
  });

  it('does not render preserved blank lines from leading or trailing whitespace', () => {
    const markup = renderToStaticMarkup(
      <TranslationCard
        providerId="google"
        providerName="Google Translate"
        status="success"
        text={'\n\ntranslated result\n'}
        bodyHeightPx={80}
        copyText={async () => undefined}
        speakText={async () => undefined}
      />,
    );

    expect(markup).toContain('>translated result</p>');
    expect(markup).not.toContain('>\n\ntranslated result\n</p>');
  });

  it('renders an independent favorite action for a completed provider result', () => {
    const markup = renderToStaticMarkup(
      <TranslationCard
        providerId="google"
        status="success"
        text="你好"
        onFavorite={async () => undefined}
        copyText={async () => undefined}
        speakText={async () => undefined}
      />,
    );

    expect(markup).toContain('aria-label="收藏"');
    expect(markup).toContain('aria-pressed="false"');
  });
});
