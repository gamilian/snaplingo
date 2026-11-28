import type { ReactElement } from 'react';
import type { SettingsNavigationTab } from '../../application/settings/navigation';
import { OverviewPage } from './OverviewPage';
import { GeneralPage } from './General/GeneralPage';
import { ScreenshotSettingsPage } from './Screenshot/ScreenshotSettingsPage';
import { ServicesPage } from './Services/ServicesPage';
import { TranslationSettingsScrollPage } from './Translation/TranslationSettingsScrollPage';
import { OcrSettingsScrollPage } from './OCR/OcrSettingsScrollPage';
import { FavoritesLibraryPage } from './Library/FavoritesLibraryPage';
import { HistoryLibraryPage } from './Library/HistoryLibraryPage';

export type MainTab = SettingsNavigationTab;

export interface SettingsSection {
  key: MainTab;
  label: string;
  render: () => ReactElement;
}

export const settingsSections: SettingsSection[] = [
  { key: 'overview', label: '概览与快捷键', render: () => <OverviewPage /> },
  { key: 'screenshot', label: '截图', render: () => <ScreenshotSettingsPage /> },
  { key: 'translation', label: '翻译', render: () => <TranslationSettingsScrollPage /> },
  { key: 'ocr', label: 'OCR', render: () => <OcrSettingsScrollPage /> },
  { key: 'services', label: '服务', render: () => <ServicesPage /> },
  { key: 'general', label: '通用设置', render: () => <GeneralPage /> },
  { key: 'history', label: '资料库', render: () => <HistoryLibraryPage /> },
  // Existing favorites navigation requests and persisted destinations remain valid.
  { key: 'favorites', label: '资料库', render: () => <FavoritesLibraryPage /> },
];

export function findSettingsSection(tab: MainTab): SettingsSection {
  return settingsSections.find((section) => section.key === tab) ?? settingsSections[0];
}
