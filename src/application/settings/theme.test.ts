import { describe, expect, it } from 'vitest';
import { resolveApplicationTheme, resolveInterfaceTheme } from './theme';

describe('application themes', () => {
  it.each(['native', 'studio', 'paper'] as const)('preserves the %s interface theme', (theme) => {
    expect(resolveInterfaceTheme(theme)).toBe(theme);
  });

  it.each([undefined, '', 'unknown'])('uses Native for legacy or unknown interface themes: %s', (value) => {
    expect(resolveInterfaceTheme(value)).toBe('native');
  });

  it.each([
    ['light', true, 'light'],
    ['dark', false, 'dark'],
    ['system', true, 'dark'],
    ['system', false, 'light'],
  ] as const)('resolves %s appearance with system dark=%s for every window', (configuredTheme, prefersDark, expected) => {
    expect(resolveApplicationTheme({ configuredTheme, prefersDark })).toBe(expected);
  });
});
