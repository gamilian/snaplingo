import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import IconActionButton from './IconActionButton';

describe('icon action button', () => {
  it('keeps the native title and accessibility label on the trigger button', () => {
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

    expect(markup).toContain('title="朗读"');
    expect(markup).toContain('aria-label="朗读"');
    expect(markup).toContain('group relative');
    expect(markup).not.toContain('bg-slate-900/90');
  });
});
