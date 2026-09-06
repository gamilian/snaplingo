import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

describe('result window canvas', () => {
  it('keeps the standalone transparent window canvas clear around its rounded panel', () => {
    const css = readFileSync(new URL('./index.css', import.meta.url), 'utf8');
    const entry = readFileSync(new URL('../main.tsx', import.meta.url), 'utf8');

    expect(css).toMatch(
      /:root\[data-result-window='true'\],\s*:root\[data-result-window='true'\] body\s*{[^}]*background:\s*transparent/s,
    );
    expect(entry).toContain("document.documentElement.dataset.resultWindow = 'true'");
  });
});
