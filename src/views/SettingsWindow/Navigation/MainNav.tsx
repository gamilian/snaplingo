import { useEffect, useState, type ReactNode } from 'react';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { useHistoryStore } from '../../../stores/historyStore';
import { useFavoritesStore } from '../../../stores/favoritesStore';
import { useScreenshotFavoritesStore } from '../../../stores/screenshotFavoritesStore';
import {
  FavoritesIcon,
  HistoryIcon,
  OcrIcon,
  ScreenshotIcon,
  ServicesIcon,
  SettingsIcon,
  TranslationIcon,
} from '../Icons';
import type { MainTab } from '../navigationModel';
import type { SettingsRuntime } from '../../../application/settings/runtime';
import { CustomNumberInput } from '../../../components/common/CustomNumberInput';
import { uiText } from '../../../application/settings/uiText';

interface MainNavProps {
  activeTab: MainTab;
  onTabChange: (tab: MainTab) => void;
  library?: SettingsRuntime['library'];
}

interface NavItem {
  key: MainTab;
  label: string;
  icon: ReactNode;
}

export function MainNav({ activeTab, onTabChange, library }: MainNavProps) {
  const language = useSettingsConfigStore((state) => state.general?.language);
  const t = (key: Parameters<typeof uiText>[1]) => uiText(language, key);
  const history = useSettingsConfigStore((state) => state.history);
  const updateHistorySettings = useSettingsConfigStore(
    (state) => state.updateHistorySettings,
  );
  const [isCapacityOpen, setCapacityOpen] = useState(false);
  const [autoCleanupEnabled, setAutoCleanupEnabled] = useState(false);
  const [retentionDays, setRetentionDays] = useState(30);
  const [historyCapacity, setHistoryCapacity] = useState(5000);
  const [favoriteCapacity, setFavoriteCapacity] = useState(1000);
  const [historyUsed, setHistoryUsed] = useState<number | null>(null);
  const [favoritesUsed, setFavoritesUsed] = useState<number | null>(null);
  const historyRevision = useHistoryStore((state) => state.revision);
  const favoriteRevision = useFavoritesStore((state) => state.revision);
  const screenshotFavoriteRevision = useScreenshotFavoritesStore(
    (state) => state.revision,
  );
  const settingsItems: NavItem[] = [
    { key: 'general', label: t('general'), icon: <SettingsIcon /> },
    { key: 'screenshot', label: t('screenshot'), icon: <ScreenshotIcon /> },
    { key: 'translation', label: t('translation'), icon: <TranslationIcon /> },
    { key: 'ocr', label: t('ocr'), icon: <OcrIcon /> },
    { key: 'services', label: t('services'), icon: <ServicesIcon /> },
  ];
  const libraryItems: NavItem[] = [
    { key: 'favorites', label: t('favorites'), icon: <FavoritesIcon /> },
    { key: 'history', label: t('history'), icon: <HistoryIcon /> },
  ];

  useEffect(() => {
    if (!history) return;
    setAutoCleanupEnabled(history.autoCleanupEnabled);
    setRetentionDays(history.retentionDays);
    setHistoryCapacity(history.maximumRecords);
    setFavoriteCapacity(history.maximumFavorites);
  }, [history]);

  useEffect(() => {
    if (!library) return;
    let cancelled = false;
    void Promise.all([
      library.queryHistory({
        filter: 'all',
        search: '',
        page: 0,
        pageSize: 1,
      }),
      library.queryFavorites({
        filter: 'all',
        search: '',
        page: 0,
        pageSize: 1,
      }),
    ]).then(
      ([historyPage, favoritesPage]) => {
        if (cancelled) return;
        setHistoryUsed(historyPage.total);
        setFavoritesUsed(favoritesPage.total);
      },
      () => {
        if (cancelled) return;
        setHistoryUsed(null);
        setFavoritesUsed(null);
      },
    );
    return () => {
      cancelled = true;
    };
  }, [
    favoriteRevision,
    historyRevision,
    library,
    screenshotFavoriteRevision,
  ]);

  const openCapacity = () => {
    if (history) {
      setAutoCleanupEnabled(history.autoCleanupEnabled);
      setRetentionDays(history.retentionDays);
      setHistoryCapacity(history.maximumRecords);
      setFavoriteCapacity(history.maximumFavorites);
    }
    setCapacityOpen(true);
  };

  const saveCapacity = async () => {
    if (!history) return;
    await updateHistorySettings({
      ...history,
      autoCleanupEnabled,
      retentionDays,
      maximumRecords: historyCapacity,
      maximumFavorites: favoriteCapacity,
    });
    setCapacityOpen(false);
  };

  return (
    <>
      <aside className="flex w-[202px] shrink-0 flex-col border-r border-line bg-sunken px-2.5 py-3">
        <div className="flex items-center gap-2.5 px-1.5 pb-3">
          <div className="grid h-7 w-7 shrink-0 place-items-center rounded-md bg-gradient-to-br from-accent to-accent-hover text-label font-bold tracking-[-0.03em] text-accent-on shadow-xs">
            S
          </div>
          <span className="truncate text-card font-bold tracking-[-0.018em] text-ink">
            SnapLingo
          </span>
        </div>

        <nav className="min-h-0 flex-1 overflow-y-auto pt-2">
          <NavGroup
            items={settingsItems}
            activeTab={activeTab}
            onTabChange={onTabChange}
          />
          <NavGroup
            label={t('library')}
            items={libraryItems}
            activeTab={activeTab}
            onTabChange={onTabChange}
          />
        </nav>

        <div className="mt-3 rounded-md border border-line bg-surface p-2.5">
          <div className="mb-2 flex items-center justify-between gap-2 border-b border-line pb-2">
            <span className="text-hint font-semibold text-ink">{t('historyAndFavorites')}</span>
            <button
              type="button"
              onClick={openCapacity}
              className="h-5 rounded-sm bg-accent-soft px-1.5 text-cap font-semibold text-accent-text hover:bg-accent-line"
            >
              {t('configure')}
            </button>
          </div>
          <CapacitySummary
            label={t('retentionDays')}
            value={history?.retentionDays ?? retentionDays}
            suffix={t('days')}
          />
          <CapacitySummary
            label={t('historyUsed')}
            value={historyUsed}
            capacity={history?.maximumRecords ?? historyCapacity}
          />
          <CapacitySummary
            label={t('favoritesUsed')}
            value={favoritesUsed}
            capacity={history?.maximumFavorites ?? favoriteCapacity}
          />
        </div>
      </aside>

      {isCapacityOpen && history && (
        <div
          className="fixed inset-0 z-[100] grid place-items-center bg-black/30 p-6 backdrop-blur-[2px]"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setCapacityOpen(false);
          }}
        >
          <section className="w-full max-w-[440px] overflow-hidden rounded-2xl border border-black/10 bg-surface shadow-2xl">
            <header className="flex items-start justify-between gap-4 border-b border-line px-5 py-4">
              <div>
                <h2 className="text-base font-bold text-ink">{t('historyAndFavorites')}</h2>
                <p className="mt-1 text-xs text-muted">
                  {t('capacityDescription')}
                </p>
              </div>
              <button
                type="button"
                onClick={() => setCapacityOpen(false)}
                className="grid h-8 w-8 place-items-center rounded-lg text-faint hover:bg-raised hover:text-ink"
                aria-label={t('close')}
              >
                ×
              </button>
            </header>
            <div className="space-y-5 px-5 py-5">
              <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-5">
                <div>
                  <div className="text-sm font-semibold text-ink">
                    {t('autoCleanupHistory')}
                  </div>
                  <div className="mt-1 text-hint text-muted">
                    {t('autoCleanupHistoryDescription')}
                  </div>
                </div>
                <button
                  type="button"
                  role="switch"
                  aria-label={t('autoCleanupHistory')}
                  aria-checked={autoCleanupEnabled}
                  onClick={() => setAutoCleanupEnabled((value) => !value)}
                  className={`relative h-[22px] w-10 rounded-full transition-colors ${
                    autoCleanupEnabled ? 'bg-accent' : 'bg-line-strong'
                  }`}
                >
                  <span
                    className={`absolute left-0.5 top-0.5 h-[18px] w-[18px] rounded-full bg-surface shadow transition-transform ${
                      autoCleanupEnabled ? 'translate-x-[18px]' : ''
                    }`}
                  />
                </button>
              </div>
              <CapacityField
                label={t('historyRetentionDays')}
                description={t('historyRetentionDaysDescription')}
                value={retentionDays}
                onChange={setRetentionDays}
                min={1}
                max={365}
                step={1}
              />
              <CapacityField
                label={t('maximumRecords')}
                description={t('maximumRecordsDescription')}
                value={historyCapacity}
                onChange={setHistoryCapacity}
                min={100}
                max={100000}
                step={100}
              />
              <CapacityField
                label={t('favoritesCapacity')}
                description={t('favoritesCapacityDescription')}
                value={favoriteCapacity}
                onChange={setFavoriteCapacity}
                min={100}
                max={100000}
                step={100}
              />
              <p className="rounded-lg bg-sunken px-3 py-2.5 text-hint leading-relaxed text-muted">
                {t('capacityNotice')}
              </p>
            </div>
            <footer className="flex justify-end gap-2 border-t border-line bg-sunken px-5 py-3.5">
              <button
                type="button"
                onClick={() => setCapacityOpen(false)}
                className="h-9 rounded-lg border border-line bg-surface px-4 text-xs font-medium text-muted hover:bg-sunken"
              >
                {t('cancel')}
              </button>
              <button
                type="button"
                onClick={() => void saveCapacity()}
                className="h-9 rounded-lg bg-accent px-4 text-xs font-semibold text-accent-on hover:bg-accent-hover"
              >
                {t('saveSettings')}
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
  label?: string;
  items: NavItem[];
  activeTab: MainTab;
  onTabChange: (tab: MainTab) => void;
}) {
  return (
    <div className="mb-3.5">
      {label && (
        <div className="px-2 pb-1.5 text-cap font-semibold uppercase tracking-[0.075em] text-faint">
          {label}
        </div>
      )}
      <div className="space-y-px">
        {items.map((item) => {
          const active = activeTab === item.key;
          return (
            <button
              key={item.key}
              type="button"
              aria-current={active ? 'page' : undefined}
              onClick={() => onTabChange(item.key)}
              className={`relative flex h-8 w-full items-center gap-2.5 rounded-md px-2 text-left text-label transition-colors ${
                active
                  ? 'bg-accent-soft font-semibold text-accent-text'
                  : 'text-muted hover:bg-raised/70 hover:text-ink'
              }`}
            >
              {active && (
                <span
                  aria-hidden="true"
                  className="absolute -left-2.5 top-1.5 bottom-1.5 w-0.5 rounded-r-sm bg-accent"
                />
              )}
              <span
                className={`shrink-0 transition-colors [&>svg]:h-4 [&>svg]:w-4 ${
                  active ? 'text-accent' : 'text-faint'
                }`}
              >
                {item.icon}
              </span>
              <span className="truncate">{item.label}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

function CapacitySummary({
  label,
  value,
  suffix,
  capacity,
}: {
  label: string;
  value: number | null;
  suffix?: string;
  capacity?: number;
}) {
  return (
    <div className="flex items-center justify-between gap-3 py-1 text-hint text-muted">
      <span>{label}</span>
      <span className="font-medium tabular-nums text-ink">
        {value === null
          ? '—'
          : `${value.toLocaleString()}${capacity === undefined ? '' : ` / ${capacity.toLocaleString()}`}${suffix ? ` ${suffix}` : ''}`}
      </span>
    </div>
  );
}

function CapacityField({
  label,
  description,
  value,
  onChange,
  min,
  max,
  step,
}: {
  label: string;
  description: string;
  value: number;
  onChange: (value: number) => void;
  min: number;
  max: number;
  step: number;
}) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_132px] items-center gap-5">
      <div>
        <div className="text-sm font-semibold text-ink">{label}</div>
        <div className="mt-1 text-hint text-muted">{description}</div>
      </div>
      <CustomNumberInput
        value={value}
        onChange={onChange}
        min={min}
        max={max}
        step={step}
      />
    </div>
  );
}
