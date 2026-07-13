import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import IconActionButton from './IconActionButton';

describe('icon action button', () => {
  it('uses the custom tooltip without also enabling the native title tooltip', () => {
    const markup = renderToStaticMarkup(
      <IconActionButton
        title="朗读"
        onClick={vi.fn()}
        className="grid h-7 w-7 place-items-center"
        tooltipPlacement="bottom"
      >
        <span>icon</span>
      </IconActionButton>,
    );

    expect(markup).toContain('aria-label="朗读"');
    expect(markup).not.toContain('title="朗读"');
    expect(markup).toContain('group relative');
    expect(markup).not.toContain('bg-slate-900/90');
  });
});
