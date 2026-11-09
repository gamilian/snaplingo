import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import {
  ClearTextIcon,
  FavoriteIcon,
  PinIcon,
  RetryIcon,
  SwapIcon,
} from './icons';

describe('result window icons', () => {
  it('renders the compact pro retry icon path', () => {
    const markup = renderToStaticMarkup(<RetryIcon />);

    expect(markup).toContain('M4 12a8 8 0 1 0 2.35-5.65L4 8.7');
    expect(markup).toContain('M4 4v4.7h4.7');
  });

  it('renders the compact pro clear-text icon path', () => {
    const markup = renderToStaticMarkup(<ClearTextIcon />);

    expect(markup).toContain('M5 7h10');
    expect(markup).toContain('M5 12h7');
    expect(markup).toContain('M5 17h6');
    expect(markup).toContain('m16 10 4 4');
    expect(markup).toContain('m20 10-4 4');
  });

  it('renders the Bob-style pin icon path', () => {
    const markup = renderToStaticMarkup(<PinIcon />);

    expect(markup).toContain('M14 4v4l4 4v2H6v-2l4-4V4');
    expect(markup).toContain('M9 4h6');
    expect(markup).toContain('M12 14v7');
  });

  it('renders the slightly longer swap icon path', () => {
    const markup = renderToStaticMarkup(<SwapIcon />);

    expect(markup).toContain('M6 7h12');
    expect(markup).toContain('m15 4 3 3-3 3');
    expect(markup).toContain('M18 17H6');
    expect(markup).toContain('m9 14-3 3 3 3');
  });

  it('renders a recognizable favorite star', () => {
    const markup = renderToStaticMarkup(<FavoriteIcon />);

    expect(markup).toContain('m12 3 2.75 5.57');
  });
});
