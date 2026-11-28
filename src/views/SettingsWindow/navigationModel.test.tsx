import { describe, expect, it } from 'vitest';
import { findSettingsSection, settingsSections, type MainTab } from './navigationModel';

describe('settings navigation model', () => {
  it('describes the seven primary destinations in sidebar order', () => {
    expect(settingsSections.slice(0, 7).map(({ key, label }) => ({ key, label }))).toEqual([
      { key: 'overview', label: '概览与快捷键' },
      { key: 'screenshot', label: '截图' },
      { key: 'translation', label: '翻译' },
      { key: 'ocr', label: 'OCR' },
      { key: 'services', label: '服务' },
      { key: 'general', label: '通用设置' },
      { key: 'history', label: '资料库' },
    ]);
  });

  it('gives OCR its own page and keeps the favorites destination accessible', () => {
    expect(findSettingsSection('ocr').label).toBe('OCR');
    expect(findSettingsSection('ocr').render().type).not.toBe(findSettingsSection('translation').render().type);
    expect(findSettingsSection('favorites').label).toBe('资料库');
    expect(findSettingsSection('favorites').render().type).not.toBe(findSettingsSection('history').render().type);
  });

  it('falls back to overview when a destination is stale', () => {
    expect(findSettingsSection('removed-tab' as MainTab).key).toBe('overview');
  });
});
