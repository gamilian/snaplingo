import { MainNav } from './Navigation/MainNav';
import { SecondaryNav } from './Navigation/SecondaryNav';
import { HotkeysPage as ScreenshotHotkeysPage } from './Screenshot/HotkeysPage';
import { SaveSettingsPage } from './Screenshot/SaveSettingsPage';
import { EditorPage as ScreenshotEditorPage } from './Screenshot/EditorPage';
import { FavoritesPage as ScreenshotFavoritesPage } from './Screenshot/FavoritesPage';
import { HotkeysPage as TranslationHotkeysPage } from './Translation/HotkeysPage';
import { TranslationSettingsPage } from './Translation/TranslationSettingsPage';
import { HistoryPage as TranslationHistoryPage } from './Translation/HistoryPage';
import { FavoritesPage as TranslationFavoritesPage } from './Translation/FavoritesPage';
import { HotkeysPage as OcrHotkeysPage } from './OCR/HotkeysPage';
import { OcrSettingsPage } from './OCR/OcrSettingsPage';
import { HistoryPage as OcrHistoryPage } from './OCR/HistoryPage';
import { FavoritesPage as OcrFavoritesPage } from './OCR/FavoritesPage';
import { OcrProvidersPage } from './Services/OcrProvidersPage';
import { TranslationProvidersPage } from './Services/TranslationProvidersPage';
import { TtsProvidersPage } from './Services/TtsProvidersPage';
import { GeneralPage } from './General/GeneralPage';
import { AdvancedPage } from './Advanced/AdvancedPage';
import { useSettingsStore } from '../../stores/settingsStore';
import { useState } from 'react';

export function SettingsWindow() {
  const activeMainTab = useSettingsStore((state) => state.activeMainTab);
  const setActiveMainTab = useSettingsStore((state) => state.setActiveMainTab);

  return (
    <div className="flex h-screen bg-[#f5f5f7]">
      {/* 左侧主导航 */}
      <MainNav activeTab={activeMainTab} onTabChange={setActiveMainTab} />

      {/* 右侧内容区 */}
      <div className="flex-1 overflow-hidden flex">
        {activeMainTab === 'screenshot' && <ScreenshotContent />}
        {activeMainTab === 'translation' && <TranslationContent />}
        {activeMainTab === 'ocr' && <OcrContent />}
        {activeMainTab === 'services' && <ServicesContent />}
        {activeMainTab === 'general' && <GeneralContent />}
        {activeMainTab === 'advanced' && <AdvancedContent />}
      </div>
    </div>
  );
}

// 截图 tab 内容 - 带二级导航
function ScreenshotContent() {
  const screenshotSubTab = useSettingsStore((state) => state.screenshotSubTab);
  const setScreenshotSubTab = useSettingsStore((state) => state.setScreenshotSubTab);

  const navItems = [
    { key: 'hotkeys', label: '快捷键' },
    { key: 'save-settings', label: '保存设置' },
    { key: 'editor', label: '编辑器' },
    { key: 'favorites', label: '收藏夹' },
  ];

  return (
    <>
      <SecondaryNav
        items={navItems}
        activeItem={screenshotSubTab}
        onItemChange={(key) => setScreenshotSubTab(key as any)}
      />
      <div className="flex-1 overflow-y-auto p-12">
        {screenshotSubTab === 'hotkeys' && <ScreenshotHotkeysPage />}
        {screenshotSubTab === 'save-settings' && <SaveSettingsPage />}
        {screenshotSubTab === 'editor' && <ScreenshotEditorPage />}
        {screenshotSubTab === 'favorites' && <ScreenshotFavoritesPage />}
      </div>
    </>
  );
}

// 翻译 tab 内容 - 带二级导航
function TranslationContent() {
  const translationSubTab = useSettingsStore((state) => state.translationSubTab);
  const setTranslationSubTab = useSettingsStore((state) => state.setTranslationSubTab);

  const navItems = [
    { key: 'hotkeys', label: '快捷键' },
    { key: 'translation-settings', label: '翻译设置' },
    { key: 'history', label: '历史记录' },
    { key: 'favorites', label: '收藏夹' },
  ];

  return (
    <>
      <SecondaryNav
        items={navItems}
        activeItem={translationSubTab}
        onItemChange={(key) => setTranslationSubTab(key as any)}
      />
      <div className="flex-1 overflow-y-auto p-12">
        {translationSubTab === 'hotkeys' && <TranslationHotkeysPage />}
        {translationSubTab === 'translation-settings' && <TranslationSettingsPage />}
        {translationSubTab === 'history' && <TranslationHistoryPage />}
        {translationSubTab === 'favorites' && <TranslationFavoritesPage />}
      </div>
    </>
  );
}

// OCR tab 内容 - 带二级导航
function OcrContent() {
  const ocrSubTab = useSettingsStore((state) => state.ocrSubTab);
  const setOcrSubTab = useSettingsStore((state) => state.setOcrSubTab);

  const navItems = [
    { key: 'hotkeys', label: '快捷键' },
    { key: 'ocr-settings', label: 'OCR 设置' },
    { key: 'history', label: '历史记录' },
    { key: 'favorites', label: '收藏夹' },
  ];

  return (
    <>
      <SecondaryNav
        items={navItems}
        activeItem={ocrSubTab}
        onItemChange={(key) => setOcrSubTab(key as any)}
      />
      <div className="flex-1 overflow-y-auto p-12">
        {ocrSubTab === 'hotkeys' && <OcrHotkeysPage />}
        {ocrSubTab === 'ocr-settings' && <OcrSettingsPage />}
        {ocrSubTab === 'history' && <OcrHistoryPage />}
        {ocrSubTab === 'favorites' && <OcrFavoritesPage />}
      </div>
    </>
  );
}

// 服务 tab 内容 - 使用顶部标签切换
function ServicesContent() {
  const [activeServiceTab, setActiveServiceTab] = useState<'ocr' | 'translation' | 'tts'>('ocr');

  return (
    <div className="flex-1 flex flex-col">
      {/* 顶部标签切换 */}
      <div className="bg-white border-b border-gray-200 px-12 pt-8">
        <div className="flex space-x-8">
          <button
            onClick={() => setActiveServiceTab('ocr')}
            className={`pb-4 text-sm font-medium transition-colors border-b-2 ${
              activeServiceTab === 'ocr'
                ? 'border-blue-500 text-primary-600'
                : 'border-transparent text-gray-600 hover:text-gray-800'
            }`}
          >
            OCR 服务
          </button>
          <button
            onClick={() => setActiveServiceTab('translation')}
            className={`pb-4 text-sm font-medium transition-colors border-b-2 ${
              activeServiceTab === 'translation'
                ? 'border-blue-500 text-primary-600'
                : 'border-transparent text-gray-600 hover:text-gray-800'
            }`}
          >
            翻译服务
          </button>
          <button
            onClick={() => setActiveServiceTab('tts')}
            className={`pb-4 text-sm font-medium transition-colors border-b-2 ${
              activeServiceTab === 'tts'
                ? 'border-blue-500 text-primary-600'
                : 'border-transparent text-gray-600 hover:text-gray-800'
            }`}
          >
            语音合成
          </button>
        </div>
      </div>

      {/* 内容区 */}
      <div className="flex-1 overflow-y-auto p-12">
        {activeServiceTab === 'ocr' && <OcrProvidersPage />}
        {activeServiceTab === 'translation' && <TranslationProvidersPage />}
        {activeServiceTab === 'tts' && <TtsProvidersPage />}
      </div>
    </div>
  );
}

// 通用 tab 内容（无二级导航）
function GeneralContent() {
  return (
    <div className="flex-1 overflow-y-auto p-12">
      <GeneralPage />
    </div>
  );
}

// 高级 tab 内容（无二级导航）
function AdvancedContent() {
  return (
    <div className="flex-1 overflow-y-auto p-12">
      <AdvancedPage />
    </div>
  );
}
