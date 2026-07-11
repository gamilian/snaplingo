import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import {
  resultWindowScrollAreaIndicatorClassName,
  resultWindowScrollAreaMetrics,
  resultWindowScrollAreaThumbClassName,
} from './ResultWindowScrollArea';

describe('result window scroll area', () => {
  it('computes a visible overlay thumb while the content is scrollable', () => {
    const metrics = resultWindowScrollAreaMetrics({
      clientHeight: 400,
      scrollHeight: 1000,
      scrollTop: 300,
    });

    expect(metrics.isScrollable).toBe(true);
    expect(metrics.thumbHeightPx).toBeGreaterThanOrEqual(24);
    expect(metrics.thumbTopPx).toBeGreaterThan(12);
    expect(metrics.thumbTopPx + metrics.thumbHeightPx).toBeLessThanOrEqual(388);
  });

  it('hides the overlay thumb when content does not overflow', () => {
    expect(
      resultWindowScrollAreaMetrics({
        clientHeight: 400,
        scrollHeight: 400,
        scrollTop: 0,
      }).isScrollable,
    ).toBe(false);
  });

  it('uses a very subtle overlay scrollbar with a soft enter and exit animation', () => {
    const visibleClassName = resultWindowScrollAreaIndicatorClassName(true);
    const hiddenClassName = resultWindowScrollAreaIndicatorClassName(false);
    const thumbClassName = resultWindowScrollAreaThumbClassName();

    expect(visibleClassName).toContain('right-0.5');
    expect(visibleClassName).not.toContain('right-1.5');
    expect(visibleClassName).toContain('opacity-100');
    expect(visibleClassName).toContain('translate-x-0');
    expect(visibleClassName).toContain('duration-200');
    expect(hiddenClassName).toContain('opacity-0');
    expect(hiddenClassName).toContain('translate-x-1');
    expect(hiddenClassName).toContain('duration-500');
    expect(visibleClassName).toContain('transition-[opacity,transform]');
    expect(visibleClassName).toContain('ease-[cubic-bezier(0.16,1,0.3,1)]');
    expect(visibleClassName).toContain('w-[3px]');
    expect(visibleClassName).not.toContain('w-1.5');

    expect(thumbClassName).toContain('w-[3px]');
    expect(thumbClassName).toContain('bg-slate-400/35');
    expect(thumbClassName).not.toContain('bg-slate-400/80');
  });

  it('hides native scrollbars so the overlay scrollbar is the only visible scrollbar', () => {
    const css = readFileSync(new URL('../../styles/index.css', import.meta.url), 'utf8');

    expect(css).toMatch(
      /\.result-window-scrollbar\s*{[^}]*scrollbar-width:\s*none/s,
    );
    expect(css).toMatch(
      /\.result-window-scrollbar\s*{[^}]*-ms-overflow-style:\s*none/s,
    );
    expect(css).toMatch(
      /\.result-window-scrollbar::\-webkit-scrollbar\s*{[^}]*display:\s*none/s,
    );
    expect(css).not.toContain('result-window-scrollbar--active');
  });
});
