import { describe, expect, it, vi } from 'vitest';

import { findSettingsSection } from './navigationModel';
import { createSettingsNavigationState, type SettingsSecondaryKeys } from './settingsNavigationState';

describe('settings navigation state', () => {
  it('keeps a valid persisted secondary key active', () => {
    const state = createSettingsNavigationState(
      findSettingsSection('translation'),
      secondaryKeys({ translation: 'history' }),
      noopSetters(),
    );

    expect(state.activeKey).toBe('history');
    expect(state.activeItem?.key).toBe('history');
  });

  it('falls back to the first secondary key when persisted state is stale', () => {
    const state = createSettingsNavigationState(
      findSettingsSection('translation'),
      secondaryKeys({ translation: 'removed-tab' }),
      noopSetters(),
    );

    expect(state.activeKey).toBe('hotkeys');
    expect(state.activeItem?.key).toBe('hotkeys');
  });

  it('returns no secondary key for a simple section', () => {
    const state = createSettingsNavigationState(
      findSettingsSection('general'),
      secondaryKeys(),
      noopSetters(),
    );

    expect(state.activeKey).toBe('');
    expect(state.activeItem).toBeNull();
  });

  it('ignores invalid secondary click keys', () => {
    const setters = noopSetters();
    const state = createSettingsNavigationState(
      findSettingsSection('ocr'),
      secondaryKeys({ ocr: 'hotkeys' }),
      setters,
    );

    state.setActiveKey('removed-tab');

    expect(setters.ocr).not.toHaveBeenCalled();
  });
});

function secondaryKeys(overrides: Partial<SettingsSecondaryKeys> = {}): SettingsSecondaryKeys {
  return {
    screenshot: 'hotkeys',
    translation: 'hotkeys',
    ocr: 'hotkeys',
    services: 'ocr',
    ...overrides,
  };
}

function noopSetters() {
  return {
    screenshot: vi.fn(),
    translation: vi.fn(),
    ocr: vi.fn(),
    services: vi.fn(),
  };
}
