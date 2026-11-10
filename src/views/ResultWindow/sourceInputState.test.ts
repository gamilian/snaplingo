import { describe, expect, it, vi } from 'vitest';
import {
  rememberSourceInputCollapsed,
  resolveSourceInputCollapsed,
} from './sourceInputState';

describe('result window source input state', () => {
  it('honors explicit collapsed and expanded settings', () => {
    const storage = { getItem: vi.fn(() => 'collapsed') };

    expect(resolveSourceInputCollapsed('selection', 'collapsed', storage)).toBe(true);
    expect(resolveSourceInputCollapsed('selection', 'expanded', storage)).toBe(false);
    expect(storage.getItem).not.toHaveBeenCalled();
  });

  it('restores and records the last state per trigger origin', () => {
    const getItem = vi.fn((key: string) =>
      key.endsWith('.screenshot') ? 'collapsed' : 'expanded',
    );
    const setItem = vi.fn();

    expect(resolveSourceInputCollapsed('screenshot', 'last', { getItem })).toBe(true);
    expect(resolveSourceInputCollapsed('selection', 'last', { getItem })).toBe(false);
    rememberSourceInputCollapsed('selection', true, { setItem });

    expect(setItem).toHaveBeenCalledWith(
      'snaplingo.result-window.source-input.selection',
      'collapsed',
    );
  });
});
