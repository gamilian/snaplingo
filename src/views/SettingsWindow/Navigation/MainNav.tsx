import { useEffect, useState, type ReactNode } from 'react';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import {
  AdvancedIcon,
  FavoritesIcon,
  HistoryIcon,
  OcrIcon,
  ScreenshotIcon,
  ServicesIcon,
  SettingsIcon,
  TranslationIcon,
} from '../Icons';
import type { MainTab } from '../navigationModel';
import { CustomNumberInput } from '../../../components/common/CustomNumberInput';

interface MainNavProps {
  activeTab: MainTab;
  onTabChange: (tab: MainTab) => void;
}

interface NavItem {
  key: MainTab;
  label: string;
  icon: ReactNode;
}

const settingsItems: NavItem[] = [
  { key: 'general', label: '通用', icon: <SettingsIcon /> },
  { key: 'screenshot', label: '截图', icon: <ScreenshotIcon /> },
  { key: 'translation', label: '翻译', icon: <TranslationIcon /> },
  { key: 'ocr', label: 'OCR', icon: <OcrIcon /> },
  { key: 'services', label: '服务', icon: <ServicesIcon /> },
  { key: 'advanced', label: '高级', icon: <AdvancedIcon /> },
];

const libraryItems: NavItem[] = [
  { key: 'favorites', label: '收藏夹', icon: <FavoritesIcon /> },
  { key: 'history', label: '历史记录', icon: <HistoryIcon /> },
];

export function MainNav({ activeTab, onTabChange }: MainNavProps) {
  const history = useSettingsConfigStore((state) => state.history);
  const updateHistorySettings = useSettingsConfigStore(
    (state) => state.updateHistorySettings,
  );
  const [isCapacityOpen, setCapacityOpen] = useState(false);
  const [historyCapacity, setHistoryCapacity] = useState(5000);
  const [favoriteCapacity, setFavoriteCapacity] = useState(1000);

  useEffect(() => {
    if (!history) return;
    setHistoryCapacity(history.maximumRecords);
    setFavoriteCapacity(history.maximumFavorites);
  }, [history]);

  const openCapacity = () => {
    if (history) {
      setHistoryCapacity(history.maximumRecords);
      setFavoriteCapacity(history.maximumFavorites);
    }
    setCapacityOpen(true);
  };

  const saveCapacity = async () => {
    if (!history) return;
    await updateHistorySettings({
      ...history,
      maximumRecords: historyCapacity,
      maximumFavorites: favoriteCapacity,
    });
    setCapacityOpen(false);
  };

  return (
    <>
      <aside className="flex w-[190px] shrink-0 flex-col border-r border-gray-200 bg-gray-50 px-3 py-4">
        <div className="flex h-12 items-center gap-3 border-b border-gray-200 px-2 pb-4">
          <div className="grid h-9 w-9 place-items-center rounded-[11px] bg-primary-600 text-sm font-extrabold text-white shadow-sm">
            S
          </div>
          <span className="text-[15px] font-bold tracking-[-0.02em] text-gray-900">
            SnapLingo
          </span>
        </div>

        <nav className="min-h-0 flex-1 overflow-y-auto pt-4">
          <NavGroup
            label="设置"
            items={settingsItems}
            activeTab={activeTab}
            onTabChange={onTabChange}
          />
          <NavGroup
            label="资料库"
            items={libraryItems}
            activeTab={activeTab}
            onTabChange={onTabChange}
          />
        </nav>

        <div className="rounded-[10px] border border-gray-200 bg-white p-3">
          <div className="mb-2 flex items-center justify-between gap-2">
            <span className="text-[11px] font-bold text-gray-700">容量</span>
            <button
              type="button"
              onClick={openCapacity}
              className="text-[11px] font-medium text-primary-600 hover:text-primary-700"
            >
              设置
            </button>
          </div>
          <CapacitySummary
            label="历史上限"
            value={history?.maximumRecords ?? historyCapacity}
          />
          <CapacitySummary
            label="收藏上限"
            value={history?.maximumFavorites ?? favoriteCapacity}
          />
        </div>
      </aside>

      {isCapacityOpen && history && (
        <div
          className="fixed inset-0 z-[100] grid place-items-center bg-gray-950/30 p-6 backdrop-blur-[2px]"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setCapacityOpen(false);
          }}
        >
          <section className="w-full max-w-[440px] overflow-hidden rounded-2xl border border-black/10 bg-white shadow-2xl">
            <header className="flex items-start justify-between gap-4 border-b border-gray-100 px-5 py-4">
              <div>
                <h2 className="text-base font-bold text-gray-900">资料库容量</h2>
                <p className="mt-1 text-xs text-gray-500">
                  控制历史记录和收藏夹占用的记录数量
                </p>
              </div>
              <button
                type="button"
                onClick={() => setCapacityOpen(false)}
                className="grid h-8 w-8 place-items-center rounded-lg text-gray-400 hover:bg-gray-100 hover:text-gray-700"
                aria-label="关闭"
              >
                ×
              </button>
            </header>
            <div className="space-y-5 px-5 py-5">
              <CapacityField
                label="历史记录容量"
                description="超过上限后自动清理最早的记录"
                value={historyCapacity}
                onChange={setHistoryCapacity}
              />
              <CapacityField
                label="收藏夹容量"
                description="达到上限后停止新增，不会自动删除收藏"
                value={favoriteCapacity}
                onChange={setFavoriteCapacity}
              />
              <p className="rounded-lg bg-gray-50 px-3 py-2.5 text-[11px] leading-relaxed text-gray-500">
                收藏是用户主动保存的内容。容量不足时会提示先整理收藏，避免静默删除重要资料。
              </p>
            </div>
            <footer className="flex justify-end gap-2 border-t border-gray-100 bg-gray-50 px-5 py-3.5">
              <button
                type="button"
                onClick={() => setCapacityOpen(false)}
                className="h-9 rounded-lg border border-gray-200 bg-white px-4 text-xs font-medium text-gray-600 hover:bg-gray-50"
              >
                取消
              </button>
              <button
                type="button"
                onClick={() => void saveCapacity()}
                className="h-9 rounded-lg bg-primary-600 px-4 text-xs font-semibold text-white hover:bg-primary-700"
              >
                保存设置
              </button>
            </footer>
          </section>
        </div>
      )}
    </>
  );
}

function NavGroup({
  label,
  items,
  activeTab,
  onTabChange,
}: {
  label: string;
  items: NavItem[];
  activeTab: MainTab;
  onTabChange: (tab: MainTab) => void;
}) {
  return (
    <div className="mb-4">
      <div className="px-2 pb-1.5 text-[10px] font-bold tracking-[0.08em] text-gray-400">
        {label}
      </div>
      <div className="space-y-0.5">
        {items.map((item) => {
          const active = activeTab === item.key;
          return (
            <button
              key={item.key}
              type="button"
              onClick={() => onTabChange(item.key)}
              className={`flex h-9 w-full items-center gap-2.5 rounded-lg px-2 text-left text-[13px] transition-colors ${
                active
                  ? 'bg-primary-50 font-semibold text-primary-700'
                  : 'text-gray-600 hover:bg-white hover:text-gray-900'
              }`}
            >
              <span
                className={`grid h-6 w-6 place-items-center rounded-md [&>svg]:h-3.5 [&>svg]:w-3.5 ${
                  active
                    ? 'bg-primary-100 text-primary-600'
                    : 'bg-gray-200/70 text-gray-500'
                }`}
              >
                {item.icon}
              </span>
              <span>{item.label}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

function CapacitySummary({ label, value }: { label: string; value: number }) {
  return (
    <div className="flex items-center justify-between gap-3 py-1 text-[10px] text-gray-500">
      <span>{label}</span>
      <span className="font-medium tabular-nums text-gray-700">
        {value.toLocaleString()}
      </span>
    </div>
  );
}

function CapacityField({
  label,
  description,
  value,
  onChange,
}: {
  label: string;
  description: string;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_132px] items-center gap-5">
      <div>
        <div className="text-sm font-semibold text-gray-800">{label}</div>
        <div className="mt-1 text-[11px] text-gray-500">{description}</div>
      </div>
      <CustomNumberInput
        value={value}
        onChange={onChange}
        min={100}
        max={100000}
        step={100}
      />
    </div>
  );
}
