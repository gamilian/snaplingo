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
      { key: 'general', label: '通用' },
      { key: 'screenshot', label: '截图' },
      { key: 'translation', label: '翻译' },
      { key: 'ocr', label: 'OCR' },
      { key: 'services', label: '服务' },
      { key: 'favorites', label: '收藏夹' },
      { key: 'history', label: '历史记录' },
    ]);
  });

  it('keeps secondary navigation items with their owning sections', () => {
    expect(secondaryKeys('screenshot')).toEqual([]);
    expect(secondaryKeys('translation')).toEqual([]);
    expect(secondaryKeys('ocr')).toEqual([]);
    expect(secondaryKeys('services')).toEqual(['ocr', 'translation', 'tts']);
  });

  it('falls back to the first secondary item when a persisted key is stale', () => {
    const section = findSettingsSection('services');

    expect(findSecondaryNavItem(section, 'removed-tab')?.key).toBe('ocr');
  });
});

function secondaryKeys(tab: MainTab): string[] {
  const section = findSettingsSection(tab);
  return 'secondary' in section ? section.secondary.map((item) => item.key) : [];
}
