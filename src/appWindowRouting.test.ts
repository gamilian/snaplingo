import { describe, expect, it } from 'vitest';
import {
  isCaptureResultWindowLaunch,
  isSettingsWindowLaunch,
} from './appWindowRouting';

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

  it('recognizes settings window launch only by its native label', () => {
    expect(isSettingsWindowLaunch('settings', '')).toBe(true);
    expect(isSettingsWindowLaunch('main', '?window=settings')).toBe(false);
    expect(isSettingsWindowLaunch('capture-result', '?window=capture-result')).toBe(false);
  });
});
