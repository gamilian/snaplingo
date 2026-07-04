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
        bodyMaxHeightPx={144}
      />,
    );

    expect(markup).toContain('max-height:144px');
    expect(markup).toContain('rounded-[14px]');
    expect(markup).toContain('leading-[1.38]');
    expect(markup).toContain('overflow-y-auto');
  });
});
