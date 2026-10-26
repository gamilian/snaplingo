import { useEffect, useState } from 'react';
import { CustomNumberInput } from '../../../components/common/CustomNumberInput';
import { CustomSelect } from '../../../components/common/CustomSelect';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { useSettingsStore } from '../../../stores/settingsStore';
import { SettingRow, SettingsGroup, SettingsToggle } from '../SettingsControls';
import { SettingsScrollPage } from '../SettingsScrollPage';
import { useSettingsRuntime } from '../runtimeContext';
import { uiText } from '../../../application/settings/uiText';
import { PermissionControls, useRequiredPermissions } from '../../PermissionControls';
import { SoftwareUpdateSettings } from './SoftwareUpdateSettings';
import { ThemePicker } from './ThemePicker';

export function GeneralPage() {
  const t = useUiText();
  const general = useSettingsConfigStore((state) => state.general);
  const updateGeneralSettings = useSettingsConfigStore(
    (state) => state.updateGeneralSettings,
  );
  const requestedSection = useSettingsStore((state) => state.requestedSection);
  const consumeRequestedSection = useSettingsStore(
    (state) => state.consumeRequestedSection,
  );

  if (!general) {
    return <div className="p-12 text-sm text-muted">{t('loading')}</div>;
  }

  const updateGeneral = (input: Partial<typeof general>) => {
    void updateGeneralSettings(input);
  };

  return (
    <SettingsScrollPage
      title={t('general')}
     
      autoSaveLabel={t('autoSave')}
      requestedSectionId={requestedSection}
      onRequestedSectionHandled={consumeRequestedSection}
      sections={[
        {
          id: 'interface',
          label: t('interfaceAndStartup'),
          content: (
            <SettingsGroup title={t('interface')}>
              <ThemePicker
                value={general.interfaceTheme ?? 'native'}
                onChange={(interfaceTheme) => updateGeneralSettings({ interfaceTheme })}
              />
              <SettingRow label={t('interfaceLanguage')}>
                <SelectField
                  value={general.language}
                  options={[
                    { value: 'zh-CN', label: '中文简体' },
                  ]}
                  onChange={(language) => updateGeneral({ language })}
                />
              </SettingRow>
              <SettingRow label={t('appearanceMode')}>
                <SelectField
                  value={general.theme}
                  options={[
                    { value: 'system', label: t('followSystem') },
                    { value: 'light', label: t('light') },
                    { value: 'dark', label: t('dark') },
                  ]}
                  onChange={(theme) => updateGeneral({ theme })}
                />
              </SettingRow>
              <SettingRow label={t('startOnBoot')}>
                <SettingsToggle
                  label={t('startOnBoot')}
                  checked={general.startOnBoot}
                  onChange={(startOnBoot) => updateGeneral({ startOnBoot })}
                />
              </SettingRow>
            </SettingsGroup>
          ),
        },
        {
          id: 'permissions',
          label: '系统权限',
          content: <PermissionSettings />,
        },
        {
          id: 'network',
          label: t('network'),
          content: <NetworkSettings />,
        },
        {
          id: 'maintenance',
          label: t('logsAndMaintenance'),
          content: <MaintenanceSettings />,
        },
        {
          id: 'experimental',
          label: t('experimental'),
          content: <ExperimentalSettings />,
        },
        {
          id: 'about',
          label: t('about'),
          content: (
            <SettingsGroup>
              <SettingRow label={t('currentVersion')}>
                <AppVersionValue />
              </SettingRow>
              <SettingRow label={t('openSourceLicense')}>
                <span className="text-xs font-medium text-ink">MIT License</span>
              </SettingRow>
              <SoftwareUpdateSettings />
            </SettingsGroup>
          ),
        },
      ]}
    />
  );
}

function PermissionSettings() {
  const runtime = useSettingsRuntime().permissions;
  const snapshot = useRequiredPermissions(runtime);
  return (
    <SettingsGroup>
      <div className="py-4">
        <PermissionControls runtime={runtime} snapshot={snapshot} />
      </div>
    </SettingsGroup>
  );
}

export function AppVersionValue() {
  const language = useSettingsConfigStore((state) => state.general?.language);
  const t = (key: Parameters<typeof uiText>[1]) => uiText(language, key);
  const runtime = useSettingsRuntime();
  const [version, setVersion] = useState(t('reading'));

  useEffect(() => {
    let disposed = false;
    runtime.window.version().then(
      (value) => {
        if (!disposed) setVersion(value);
      },
      () => {
        if (!disposed) setVersion(t('unknown'));
      },
    );
    return () => {
      disposed = true;
    };
  }, [language, runtime]);

  return <span className="text-xs font-medium text-ink">{version}</span>;
}

function NetworkSettings() {
  const t = useUiText();
  const general = useSettingsConfigStore((state) => state.general);
  const updateGeneralSettings = useSettingsConfigStore(
    (state) => state.updateGeneralSettings,
  );
  if (!general) return null;
  const updateGeneral = (input: Partial<typeof general>) => {
    void updateGeneralSettings(input);
  };

  return (
    <SettingsGroup title={t('connection')}>
      <SettingRow label={t('proxy')}>
        <SelectField
          value={general.proxyMode ?? 'system'}
          options={[
            { value: 'system', label: t('followSystem') },
            { value: 'none', label: t('noProxy') },
            { value: 'manual', label: t('manualProxy') },
          ]}
          onChange={(proxyMode) => updateGeneral({ proxyMode: proxyMode as 'system' | 'none' | 'manual' })}
        />
      </SettingRow>
      {(general.proxyMode ?? 'system') === 'manual' && (
        <SettingRow label={t('proxyAddress')}>
          <input
            key={general.proxyUrl ?? ''}
            defaultValue={general.proxyUrl ?? ''}
            onBlur={(event) => updateGeneral({ proxyUrl: event.currentTarget.value })}
            placeholder="http://127.0.0.1:7890"
            className="h-9 w-[230px] rounded-lg border border-line bg-surface px-3 text-xs text-ink outline-none focus:border-accent focus:ring-2 focus:ring-accent-line"
          />
        </SettingRow>
      )}
      <SettingRow label={t('requestTimeout')}>
        <CustomNumberInput
          value={general.requestTimeoutMs ?? 10_000}
          onChange={(requestTimeoutMs) => updateGeneral({ requestTimeoutMs })}
          min={1_000}
          max={120_000}
          step={1_000}
          suffix="ms"
          className="w-[230px]"
        />
      </SettingRow>
      <SettingRow label={t('retryCount')}>
        <CustomNumberInput
          value={general.retryCount ?? 1}
          onChange={(retryCount) => updateGeneral({ retryCount })}
          min={0}
          max={5}
          step={1}
          className="w-[230px]"
        />
      </SettingRow>
    </SettingsGroup>
  );
}

function MaintenanceSettings() {
  const t = useUiText();
  const runtime = useSettingsRuntime();
  const general = useSettingsConfigStore((state) => state.general);
  const updateGeneralSettings = useSettingsConfigStore(
    (state) => state.updateGeneralSettings,
  );
  const [logs, setLogs] = useState<Array<{ id: number; timestamp: string; level: string; target: string; message: string }>>([]);
  const reloadLogs = () => {
    void runtime.maintenance.listAppLogs(8).then(setLogs, () => setLogs([]));
  };

  useEffect(() => {
    reloadLogs();
  }, [runtime]);

  if (!general) return null;

  return (
    <>
      <SettingsGroup title={t('logs')}>
        <SettingRow label={t('logLevel')}>
          <SelectField
            value={general.logLevel ?? 'info'}
            options={[
              { value: 'debug', label: 'Debug' },
              { value: 'info', label: 'Info' },
              { value: 'warn', label: 'Warn' },
              { value: 'error', label: 'Error' },
            ]}
            onChange={(logLevel) => void updateGeneralSettings({ logLevel: logLevel as 'debug' | 'info' | 'warn' | 'error' })}
          />
        </SettingRow>
        <SettingRow label={t('logRetentionDays')}>
          <CustomNumberInput
            value={general.logRetentionDays ?? 7}
            onChange={(logRetentionDays) =>
              void updateGeneralSettings({ logRetentionDays })
            }
            min={1}
            max={365}
            step={1}
            suffix={t('days')}
            className="w-[230px]"
          />
        </SettingRow>
        <SettingRow label={t('logStorage')}>
          <span className="text-xs font-medium text-muted">
            {t('recentLogs').replace('{count}', String(logs.length))}
          </span>
        </SettingRow>
        <SettingRow label={t('logActions')}>
          <div className="flex gap-2">
            <ActionButton onClick={reloadLogs}>{t('refreshLogs')}</ActionButton>
            <ActionButton
              danger
              onClick={() => {
                if (!confirm(t('clearLogsConfirm'))) return;
                void runtime.maintenance.clearAppLogs().then(reloadLogs);
              }}
            >
              {t('clearLogs')}
            </ActionButton>
          </div>
        </SettingRow>
        {logs.length > 0 && (
          <div className="max-h-40 overflow-auto rounded-lg border border-line bg-sunken px-3 py-2 font-mono text-hint leading-5 text-muted">
            {logs.map((entry) => (
              <div key={entry.id} className="truncate">
                [{entry.level}] {entry.message}
              </div>
            ))}
          </div>
        )}
      </SettingsGroup>
      <SettingsGroup title={t('dataMaintenance')}>
        <SettingRow label={t('historyRecords')}>
          <ActionButton
            onClick={() => {
              if (confirm(t('clearHistoryConfirm'))) {
                void runtime.history.clear();
              }
            }}
          >
            {t('clearAllHistory')}
          </ActionButton>
        </SettingRow>
        <SettingRow label={t('appSettings')}>
          <ActionButton danger disabled>{t('unavailable')}</ActionButton>
        </SettingRow>
      </SettingsGroup>
    </>
  );
}

function ExperimentalSettings() {
  const t = useUiText();
  const general = useSettingsConfigStore((state) => state.general);
  const updateGeneralSettings = useSettingsConfigStore(
    (state) => state.updateGeneralSettings,
  );
  if (!general) return null;

  return (
    <SettingsGroup title={t('performance')}>
      <SettingRow label={t('gpuComposition')}>
        <SettingsToggle
          label={t('gpuComposition')}
          checked={general.experimentalGpuAcceleration ?? false}
          onChange={(experimentalGpuAcceleration) =>
            void updateGeneralSettings({ experimentalGpuAcceleration })
          }
        />
      </SettingRow>
      <SettingRow label={t('performanceMonitoring')}>
        <SettingsToggle
          label={t('performanceMonitoring')}
          checked={general.performanceMonitoring ?? false}
          onChange={(performanceMonitoring) =>
            void updateGeneralSettings({ performanceMonitoring })
          }
        />
      </SettingRow>
    </SettingsGroup>
  );
}

function useUiText() {
  const language = useSettingsConfigStore((state) => state.general?.language);
  return (key: Parameters<typeof uiText>[1]) => uiText(language, key);
}

function SelectField({
  value,
  options,
  onChange,
}: {
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
}) {
  return (
    <div className="w-[230px]">
      <CustomSelect value={value} options={options} onChange={onChange} />
    </div>
  );
}

function ActionButton({
  children,
  danger = false,
  disabled = false,
  onClick,
}: {
  children: string;
  danger?: boolean;
  disabled?: boolean;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={`h-9 min-w-[132px] rounded-lg border px-3 text-xs font-semibold transition-colors ${
        disabled
          ? 'cursor-not-allowed border-line bg-sunken text-faint'
          : danger
          ? 'border-red-200 bg-red-50 text-red-600 hover:bg-red-100'
          : 'border-line bg-surface text-muted hover:bg-sunken'
      }`}
    >
      {children}
    </button>
  );
}
