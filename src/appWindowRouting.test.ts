import { describe, expect, it } from 'vitest';
import { isCaptureResultWindowLaunch } from './appWindowRouting';

describe('app window routing', () => {
  it('routes capture result windows by native window label even when URL search is missing', () => {
    expect(isCaptureResultWindowLaunch('capture-result', '')).toBe(true);
  });

  it('keeps the URL query fallback for browser tests and legacy launches', () => {
    expect(isCaptureResultWindowLaunch('main', '?window=capture-result')).toBe(true);
  });

  it('does not route the main settings window as a capture result window', () => {
    expect(isCaptureResultWindowLaunch('main', '')).toBe(false);
  });
});
