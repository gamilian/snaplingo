import { describe, expect, it } from 'vitest';

import { resolveApplicationTheme } from './theme';

describe('application theme scope', () => {
  it('applies the configured theme only to the settings window', () => {
    expect(
      resolveApplicationTheme({
        configuredTheme: 'dark',
        isSettingsWindow: true,
        prefersDark: false,
      }),
    ).toBe('dark');
    expect(
      resolveApplicationTheme({
        configuredTheme: 'dark',
        isSettingsWindow: false,
        prefersDark: true,
      }),
    ).toBe('light');
  });

  it('resolves the system theme for settings', () => {
    expect(
      resolveApplicationTheme({
        configuredTheme: 'system',
        isSettingsWindow: true,
        prefersDark: true,
      }),
    ).toBe('dark');
  });
});
