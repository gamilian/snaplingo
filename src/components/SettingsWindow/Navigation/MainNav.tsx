import React from 'react';
import {
  ScreenshotIcon,
  TranslationIcon,
  OcrIcon,
  ServicesIcon,
  SettingsIcon,
  AdvancedIcon,
} from '../Icons';

export type MainTab = 'screenshot' | 'translation' | 'ocr' | 'services' | 'general' | 'advanced';

interface MainNavProps {
  activeTab: MainTab;
  onTabChange: (tab: MainTab) => void;
}

interface NavButtonProps {
  icon: React.ReactNode;
  label: string;
  active: boolean;
  onClick: () => void;
}

function NavButton({ icon, label, active, onClick }: NavButtonProps) {
  return (
    <button
      onClick={onClick}
      className={`flex flex-col items-center justify-center w-16 h-16 rounded-xl transition-all ${
        active ? 'bg-white shadow-md text-blue-600' : 'text-gray-500 hover:text-gray-700 hover:bg-white/50'
      }`}
    >
      <div className="mb-1">{icon}</div>
      <span className="text-[10px] font-medium">{label}</span>
    </button>
  );
}

export function MainNav({ activeTab, onTabChange }: MainNavProps) {
  return (
    <div className="w-24 bg-gradient-to-b from-gray-50 to-gray-100 border-r border-gray-200 flex flex-col items-center py-8 space-y-6">
      {/* Logo */}
      <div className="w-12 h-12 bg-gradient-to-br from-blue-500 to-blue-600 rounded-2xl flex items-center justify-center shadow-lg">
        <svg className="w-7 h-7 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"
          />
        </svg>
      </div>

      <div className="h-px w-12 bg-gray-300" />

      {/* 主导航按钮 */}
      <nav className="flex-1 flex flex-col space-y-2">
        <NavButton
          icon={<ScreenshotIcon />}
          label="截图"
          active={activeTab === 'screenshot'}
          onClick={() => onTabChange('screenshot')}
        />
        <NavButton
          icon={<TranslationIcon />}
          label="翻译"
          active={activeTab === 'translation'}
          onClick={() => onTabChange('translation')}
        />
        <NavButton
          icon={<OcrIcon />}
          label="OCR"
          active={activeTab === 'ocr'}
          onClick={() => onTabChange('ocr')}
        />
        <NavButton
          icon={<ServicesIcon />}
          label="服务"
          active={activeTab === 'services'}
          onClick={() => onTabChange('services')}
        />
        <NavButton
          icon={<SettingsIcon />}
          label="通用"
          active={activeTab === 'general'}
          onClick={() => onTabChange('general')}
        />
        <NavButton
          icon={<AdvancedIcon />}
          label="高级"
          active={activeTab === 'advanced'}
          onClick={() => onTabChange('advanced')}
        />
      </nav>
    </div>
  );
}
