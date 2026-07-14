import type { ReactElement } from 'react';

import { AdvancedPage } from './Advanced/AdvancedPage';
import { GeneralPage } from './General/GeneralPage';
import { HotkeysPage as OcrHotkeysPage } from './OCR/HotkeysPage';
import { OcrSettingsPage } from './OCR/OcrSettingsPage';
import { HotkeysPage as ScreenshotHotkeysPage } from './Screenshot/HotkeysPage';
import { SaveSettingsPage } from './Screenshot/SaveSettingsPage';
import { EditorPage as ScreenshotEditorPage } from './Screenshot/EditorPage';
import { OcrProvidersPage } from './Services/OcrProvidersPage';
import { TranslationProvidersPage } from './Services/TranslationProvidersPage';
import { TtsProvidersPage } from './Services/TtsProvidersPage';
import { HotkeysPage as TranslationHotkeysPage } from './Translation/HotkeysPage';
import { TranslationSettingsPage } from './Translation/TranslationSettingsPage';
import { FavoritesLibraryPage } from './Library/FavoritesLibraryPage';
import { HistoryLibraryPage } from './Library/HistoryLibraryPage';

export type MainTab =
  | 'screenshot'
  | 'translation'
  | 'ocr'
  | 'services'
  | 'favorites'
  | 'history'
  | 'general'
  | 'advanced';

export type ScreenshotSubTab = 'hotkeys' | 'save-settings' | 'editor';
export type TranslationSubTab = 'hotkeys' | 'translation-settings';
export type OcrSubTab = 'hotkeys' | 'ocr-settings';
export type ServicesSubTab = 'ocr' | 'translation' | 'tts';

export interface SecondaryNavItem<Key extends string = string> {
  key: Key;
  label: string;
  render: () => ReactElement;
}

interface SecondarySettingsSection<Key extends MainTab, SecondaryKey extends string> {
  key: Key;
  label: string;
  secondary: SecondaryNavItem<SecondaryKey>[];
}

interface SimpleSettingsSection<Key extends MainTab> {
  key: Key;
  label: string;
  render: () => ReactElement;
}

export type SettingsSection =
  | SecondarySettingsSection<'screenshot', ScreenshotSubTab>
  | SecondarySettingsSection<'translation', TranslationSubTab>
  | SecondarySettingsSection<'ocr', OcrSubTab>
  | SecondarySettingsSection<'services', ServicesSubTab>
  | SimpleSettingsSection<'favorites'>
  | SimpleSettingsSection<'history'>
  | SimpleSettingsSection<'general'>
  | SimpleSettingsSection<'advanced'>;

export const settingsSections: SettingsSection[] = [
  {
    key: 'screenshot',
    label: '截图',
    secondary: [
      { key: 'hotkeys', label: '快捷键', render: () => <ScreenshotHotkeysPage /> },
      { key: 'save-settings', label: '保存设置', render: () => <SaveSettingsPage /> },
      { key: 'editor', label: '编辑器', render: () => <ScreenshotEditorPage /> },
    ],
  },
  {
    key: 'translation',
    label: '翻译',
    secondary: [
      { key: 'hotkeys', label: '快捷键', render: () => <TranslationHotkeysPage /> },
      { key: 'translation-settings', label: '翻译设置', render: () => <TranslationSettingsPage /> },
    ],
  },
  {
    key: 'ocr',
    label: 'OCR',
    secondary: [
      { key: 'hotkeys', label: '快捷键', render: () => <OcrHotkeysPage /> },
      { key: 'ocr-settings', label: 'OCR 设置', render: () => <OcrSettingsPage /> },
    ],
  },
  {
    key: 'services',
    label: '服务',
    secondary: [
      { key: 'ocr', label: 'OCR 服务', render: () => <OcrProvidersPage /> },
      { key: 'translation', label: '翻译服务', render: () => <TranslationProvidersPage /> },
      { key: 'tts', label: '语音合成', render: () => <TtsProvidersPage /> },
    ],
  },
  {
    key: 'general',
    label: '通用',
    render: () => <GeneralPage />,
  },
  {
    key: 'advanced',
    label: '高级',
    render: () => <AdvancedPage />,
  },
  {
    key: 'favorites',
    label: '收藏夹',
    render: () => <FavoritesLibraryPage />,
  },
  {
    key: 'history',
    label: '历史记录',
    render: () => <HistoryLibraryPage />,
  },
];

export function findSettingsSection(tab: MainTab): SettingsSection {
  return settingsSections.find((section) => section.key === tab) ?? settingsSections[0];
}

export function findSecondaryNavItem(
  section: SettingsSection,
  key: string,
): SecondaryNavItem | null {
  if (!('secondary' in section)) {
    return null;
  }

  return section.secondary.find((item) => item.key === key) ?? section.secondary[0] ?? null;
}

export function isScreenshotSubTab(key: string): key is ScreenshotSubTab {
  return hasSecondaryKey('screenshot', key);
}

export function isTranslationSubTab(key: string): key is TranslationSubTab {
  return hasSecondaryKey('translation', key);
}

export function isOcrSubTab(key: string): key is OcrSubTab {
  return hasSecondaryKey('ocr', key);
}

export function isServicesSubTab(key: string): key is ServicesSubTab {
  return hasSecondaryKey('services', key);
}

function hasSecondaryKey(tab: MainTab, key: string): boolean {
  const section = findSettingsSection(tab);
  return 'secondary' in section && section.secondary.some((item) => item.key === key);
}
