import { useEffect, useState } from 'react';
import { CustomSelect } from '../../../components/common/CustomSelect';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { useSettingsStore } from '../../../stores/settingsStore';
import { SettingRow, SettingsGroup, SettingsToggle } from '../SettingsControls';
import { SettingsScrollPage } from '../SettingsScrollPage';
import { useSettingsRuntime } from '../runtimeContext';

export function GeneralPage() {
  const general = useSettingsConfigStore((state) => state.general);
  const updateGeneralSettings = useSettingsConfigStore(
    (state) => state.updateGeneralSettings,
  );
  const requestedSection = useSettingsStore((state) => state.requestedSection);
  const consumeRequestedSection = useSettingsStore(
    (state) => state.consumeRequestedSection,
  );

  if (!general) {
    return <div className="p-12 text-sm text-gray-500">设置加载中...</div>;
  }

  const updateGeneral = (input: Partial<typeof general>) => {
    void updateGeneralSettings(input);
  };

  return (
    <SettingsScrollPage
      title="通用"
      description="应用界面、网络、日志与维护"
      requestedSectionId={requestedSection}
      onRequestedSectionHandled={consumeRequestedSection}
      sections={[
        {
          id: 'interface',
          label: '界面与启动',
          description: '应用外观与系统启动行为',
          content: (
            <SettingsGroup title="界面">
              <SettingRow label="界面语言" description="选择应用显示的语言">
                <SelectField
                  value={general.language}
                  options={[
                    { value: 'zh-CN', label: '中文简体' },
                    { value: 'zh-TW', label: '中文繁體' },
                    { value: 'en', label: 'English' },
                    { value: 'ja', label: '日本語' },
                  ]}
                  onChange={(language) => updateGeneral({ language })}
                />
              </SettingRow>
              <SettingRow label="主题" description="选择应用的外观主题">
                <SelectField
                  value={general.theme}
                  options={[
                    { value: 'system', label: '跟随系统' },
                    { value: 'light', label: '浅色' },
                    { value: 'dark', label: '深色' },
                  ]}
                  onChange={(theme) => updateGeneral({ theme })}
                />
              </SettingRow>
              <SettingRow label="开机自启" description="系统启动时自动运行 SnapLingo">
                <SettingsToggle
                  label="开机自启"
                  checked={general.startOnBoot}
                  onChange={(startOnBoot) => updateGeneral({ startOnBoot })}
                />
              </SettingRow>
            </SettingsGroup>
          ),
        },
        {
          id: 'network',
          label: '网络',
          description: '代理、超时与失败重试策略',
          content: <NetworkSettings />,
        },
        {
          id: 'maintenance',
          label: '日志与维护',
          description: '日志记录和本地数据维护工具',
          content: <MaintenanceSettings />,
        },
        {
          id: 'experimental',
          label: '实验性功能',
          description: '尚在验证中的性能功能',
          content: <ExperimentalSettings />,
        },
        {
          id: 'about',
          label: '关于',
          description: '版本、开源协议与更新',
          content: (
            <SettingsGroup title="SnapLingo">
              <SettingRow label="当前版本">
                <AppVersionValue />
              </SettingRow>
              <SettingRow label="开源协议">
                <span className="text-xs font-medium text-gray-700">MIT License</span>
              </SettingRow>
              <SettingRow label="软件更新">
                <ActionButton disabled>暂未开放</ActionButton>
              </SettingRow>
            </SettingsGroup>
          ),
        },
      ]}
    />
  );
}

export function AppVersionValue() {
  const runtime = useSettingsRuntime();
  const [version, setVersion] = useState('读取中…');

  useEffect(() => {
    let disposed = false;
    runtime.window.version().then(
      (value) => {
        if (!disposed) setVersion(value);
      },
      () => {
        if (!disposed) setVersion('未知');
      },
    );
    return () => {
      disposed = true;
    };
  }, [runtime]);

  return <span className="text-xs font-medium text-gray-700">{version}</span>;
}

function NetworkSettings() {
  return (
    <SettingsGroup title="连接">
      <SettingRow label="代理设置" description="当前版本使用系统网络设置">
        <UnavailableBadge />
      </SettingRow>
      <SettingRow label="请求超时时间" description="尚未提供全局超时配置">
        <UnavailableBadge />
      </SettingRow>
      <SettingRow label="失败重试次数" description="由各服务当前实现决定">
        <UnavailableBadge />
      </SettingRow>
    </SettingsGroup>
  );
}

function MaintenanceSettings() {
  const runtime = useSettingsRuntime();

  return (
    <>
      <SettingsGroup title="日志">
        <SettingRow label="日志级别" description="当前版本不支持运行时修改">
          <UnavailableBadge />
        </SettingRow>
        <SettingRow label="保存日志到文件" description="便于排查问题">
          <UnavailableBadge />
        </SettingRow>
        <SettingRow label="日志目录">
          <ActionButton disabled>暂未开放</ActionButton>
        </SettingRow>
      </SettingsGroup>
      <SettingsGroup title="数据维护">
        <SettingRow label="历史记录">
          <ActionButton
            onClick={() => {
              if (confirm('确定要清空全部翻译和 OCR 历史吗？独立收藏不会被删除。')) {
                void runtime.history.clear();
              }
            }}
          >
            清除所有历史记录
          </ActionButton>
        </SettingRow>
        <SettingRow label="本地缓存">
          <ActionButton disabled>暂未开放</ActionButton>
        </SettingRow>
        <SettingRow label="应用设置" description="恢复为首次安装时的默认设置">
          <ActionButton danger disabled>暂未开放</ActionButton>
        </SettingRow>
      </SettingsGroup>
    </>
  );
}

function ExperimentalSettings() {
  return (
    <SettingsGroup title="性能">
      <SettingRow label="GPU 加速" description="使用 GPU 加速 OCR 识别（规划中）">
        <UnavailableBadge />
      </SettingRow>
      <SettingRow label="性能监控" description="显示 CPU、内存占用等性能指标">
        <UnavailableBadge />
      </SettingRow>
    </SettingsGroup>
  );
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
          ? 'cursor-not-allowed border-gray-100 bg-gray-50 text-gray-400'
          : danger
          ? 'border-red-200 bg-red-50 text-red-600 hover:bg-red-100'
          : 'border-gray-200 bg-white text-gray-600 hover:bg-gray-50'
      }`}
    >
      {children}
    </button>
  );
}

function UnavailableBadge() {
  return (
    <span className="rounded-md bg-gray-100 px-2.5 py-1.5 text-[11px] font-medium text-gray-400">
      暂未开放
    </span>
  );
}
