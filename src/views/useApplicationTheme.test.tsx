// @vitest-environment happy-dom

import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { GeneralSettings } from '../application/settings/ports';
import { useApplicationTheme } from './useApplicationTheme';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

afterEach(() => vi.restoreAllMocks());

function ThemeHost({ general, scope }: { general: GeneralSettings; scope: 'settings' | 'application' }) {
  useApplicationTheme(general, scope);
  return null;
}

describe('shared window theme', () => {
  it.each(['settings', 'application'] as const)('uses one native theme and follows system appearance in %s windows', async (scope) => {
    const listeners = new Set<() => void>();
    const media = {
      matches: false,
      addEventListener: (_event: string, listener: () => void) => listeners.add(listener),
      removeEventListener: (_event: string, listener: () => void) => listeners.delete(listener),
    };
    vi.spyOn(window, 'matchMedia').mockReturnValue(media as unknown as MediaQueryList);
    const container = document.createElement('div');
    const root = createRoot(container);
    const general: GeneralSettings = { language: 'zh-CN', theme: 'system', interfaceTheme: 'paper', startOnBoot: false };

    try {
      await act(async () => root.render(<ThemeHost general={general} scope={scope} />));
      expect(document.documentElement.dataset).toMatchObject({ interfaceTheme: 'native', theme: 'light', themeScope: scope });
      media.matches = true;
      act(() => listeners.forEach((listener) => listener()));
      expect(document.documentElement.dataset).toMatchObject({ interfaceTheme: 'native', theme: 'dark' });

      await act(async () => root.render(<ThemeHost general={{ ...general, theme: 'light', interfaceTheme: 'studio' }} scope={scope} />));
      expect(document.documentElement.dataset).toMatchObject({ interfaceTheme: 'native', theme: 'dark' });
      expect(document.documentElement.style.colorScheme).toBe('dark');
      expect(listeners.size).toBe(1);
      media.matches = false;
      act(() => listeners.forEach((listener) => listener()));
      expect(document.documentElement.dataset.theme).toBe('light');
    } finally {
      await act(async () => root.unmount());
      expect(listeners.size).toBe(0);
    }
  });
});
