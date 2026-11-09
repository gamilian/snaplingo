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
  it.each(['settings', 'application'] as const)('updates %s windows when settings or system appearance change', async (scope) => {
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
      expect(document.documentElement.dataset).toMatchObject({ interfaceTheme: 'paper', theme: 'light', themeScope: scope });
      media.matches = true;
      act(() => listeners.forEach((listener) => listener()));
      expect(document.documentElement.dataset).toMatchObject({ interfaceTheme: 'paper', theme: 'dark' });

      await act(async () => root.render(<ThemeHost general={{ ...general, theme: 'light', interfaceTheme: 'studio' }} scope={scope} />));
      expect(document.documentElement.dataset).toMatchObject({ interfaceTheme: 'studio', theme: 'light' });
      expect(document.documentElement.style.colorScheme).toBe('light');
      expect(listeners.size).toBe(0);
    } finally {
      await act(async () => root.unmount());
      expect(listeners.size).toBe(0);
    }
  });
});
