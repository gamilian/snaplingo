import { describe, expect, it } from 'vitest';

import {
  findSecondaryNavItem,
  findSettingsSection,
  settingsSections,
  type MainTab,
} from './navigationModel';

describe('settings navigation model', () => {
  it('describes the main settings sections in sidebar order', () => {
    expect(settingsSections.map(({ key, label }) => ({ key, label }))).toEqual([
      { key: 'screenshot', label: '截图' },
      { key: 'translation', label: '翻译' },
      { key: 'ocr', label: 'OCR' },
      { key: 'services', label: '服务' },
      { key: 'general', label: '通用' },
      { key: 'advanced', label: '高级' },
    ]);
  });

  it('keeps secondary navigation items with their owning sections', () => {
    expect(secondaryKeys('screenshot')).toEqual([
      'hotkeys',
      'save-settings',
      'editor',
      'favorites',
    ]);
    expect(secondaryKeys('translation')).toEqual([
      'hotkeys',
      'translation-settings',
      'history',
      'favorites',
    ]);
    expect(secondaryKeys('ocr')).toEqual([
      'hotkeys',
      'ocr-settings',
      'history',
      'favorites',
    ]);
    expect(secondaryKeys('services')).toEqual(['ocr', 'translation', 'tts']);
  });

  it('falls back to the first secondary item when a persisted key is stale', () => {
    const section = findSettingsSection('translation');

    expect(findSecondaryNavItem(section, 'removed-tab')?.key).toBe('hotkeys');
  });
});

function secondaryKeys(tab: MainTab): string[] {
  const section = findSettingsSection(tab);
  return 'secondary' in section ? section.secondary.map((item) => item.key) : [];
}
