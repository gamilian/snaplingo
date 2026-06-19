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
      className={`
        relative flex flex-col items-center justify-center w-full px-2 py-3 rounded-lg
        transition-all duration-150
        ${active
          ? 'bg-primary-50 text-primary-600 shadow-xs'
          : 'text-gray-500 hover:text-gray-700 hover:bg-gray-100 hover:-translate-y-0.5'
        }
      `}
    >
      {/* 左侧指示器 */}
      {active && (
        <div className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-6 bg-primary-600 rounded-r" />
      )}

      <div className="mb-1">{icon}</div>
      <span className={`text-[11px] font-medium ${active ? 'font-semibold' : ''}`}>
        {label}
      </span>
    </button>
  );
}

export function MainNav({ activeTab, onTabChange }: MainNavProps) {
  return (
    <div className="w-[88px] bg-white border-r border-gray-200 flex flex-col items-center py-6 gap-4">
      {/* Logo */}
      <div className="w-12 h-12 bg-gradient-to-br from-primary-500 to-primary-700 rounded-[18px] flex items-center justify-center shadow-lg">
        <svg className="w-7 h-7 text-white" fill="none" viewBox="0 0 28 28" stroke="currentColor">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2.5}
            d="M8 10L14 16L20 10"
          />
          <path
            strokeLinecap="round"
            strokeWidth={2.5}
            d="M14 8v8"
          />
          <circle cx="14" cy="20" r="2" fill="currentColor"/>
        </svg>
      </div>

      <div className="h-px w-10 bg-gray-200 my-2" />

      {/* 主导航按钮 */}
      <nav className="flex-1 flex flex-col gap-2 w-full px-3">
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
