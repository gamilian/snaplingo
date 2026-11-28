import type { ReactNode } from 'react';

import { Provider } from '../../../stores/providerStore';
import IconActionButton from '../../../components/common/IconActionButton';
import { ProviderBrandIcon } from '../../../components/common/ProviderBrandIcon';

interface ProviderCardProps {
  provider: Provider;
  onActivate?: () => void;
  onDeactivate?: () => void;
  onConfigure?: () => void;
  onTest?: () => void;
  onRemove?: () => void;
  leadingSlot?: ReactNode;
  highlighted?: boolean;
}

export function ProviderCard({
  provider,
  onActivate,
  onDeactivate,
  onConfigure,
  onTest,
  onRemove,
  leadingSlot,
  highlighted = false,
}: ProviderCardProps) {
  const isActive = provider.status === 'active';
  const canToggle = provider.status !== 'unconfigured' && Boolean(isActive ? onDeactivate : onActivate);
  const subtitle = providerSubtitle(provider);

  const handleToggle = () => {
    if (isActive) {
      onDeactivate?.();
      return;
    }

    onActivate?.();
  };

  return (
    <div
      className={`provider-card group/provider relative min-h-[72px] px-5 py-3.5 transition-colors duration-150 ${
        highlighted
          ? 'bg-accent-soft/60'
          : 'hover:bg-sunken'
      }`}
    >
      <div className="flex h-full items-center gap-4">
        {leadingSlot && <div className="flex w-5 flex-shrink-0 justify-center text-faint">{leadingSlot}</div>}

        <ProviderBrandIcon {...provider} />

        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h3 className="truncate text-base font-semibold text-ink">
              {provider.name}
            </h3>
            {provider.status === 'unconfigured' ? (
              <span className="rounded-full bg-amber-50 px-2 py-0.5 text-hint font-medium text-amber-700">
                未配置
              </span>
            ) : null}
            <span className="rounded-full bg-raised px-2 py-0.5 text-hint font-medium text-muted">
              {provider.isLocal ? '本地处理' : '发送到服务'}
            </span>
          </div>
          <p
            className={`mt-1 truncate text-sm ${
              provider.endpoint ? 'text-blue-500' : 'text-muted'
            }`}
          >
            {subtitle}
          </p>
        </div>

        <div className="flex flex-shrink-0 items-center justify-end gap-1.5">
          {canToggle ? (
            <button type="button" role="switch" aria-label={`启用 ${provider.name}`} aria-checked={isActive} onClick={handleToggle} className={`relative h-[21px] w-9 shrink-0 rounded-full transition-colors ${isActive ? 'bg-accent' : 'bg-line-strong'}`}>
              <span className={`absolute left-[2.5px] top-[2.5px] h-4 w-4 rounded-full bg-white shadow-sm transition-transform ${isActive ? 'translate-x-[15px]' : ''}`} />
            </button>
          ) : isActive ? <span className="rounded-full bg-accent-soft px-2 py-1 text-hint text-accent">已激活</span> : null}

          {onConfigure ? (
            <IconActionButton
              title="编辑"
              onClick={onConfigure}
              className="inline-flex h-9 w-9 items-center justify-center rounded-lg text-muted transition-colors hover:bg-raised hover:text-ink"
            >
              <EditIcon />
            </IconActionButton>
          ) : null}

          {onTest ? (
            <IconActionButton
              title="测试联通"
              onClick={onTest}
              className="inline-flex h-9 w-9 items-center justify-center rounded-lg text-muted transition-colors hover:bg-raised hover:text-ink"
            >
              <PulseIcon />
            </IconActionButton>
          ) : null}

          {!provider.isBuiltin && onRemove ? (
            <IconActionButton
              title="删除"
              onClick={onRemove}
              className="inline-flex h-9 w-9 items-center justify-center rounded-lg text-muted transition-colors hover:bg-red-50 hover:text-red-600"
            >
              <TrashIcon />
            </IconActionButton>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function providerSubtitle(provider: Provider) {
  if (provider.endpoint) {
    return provider.endpoint;
  }

  if (provider.description) {
    return provider.description;
  }

  switch (provider.id) {
    case 'paddle-ocr': {
      return `离线文字识别 · ${provider.acceleration?.fallbackReason ?? 'small · CPU · 识别后释放模型'}`;
    }
    case 'google-translate':
      return 'https://translate.googleapis.com';
    case 'deeplx':
      return 'DeepLX / 标准 DeepL';
    case 'baidu-translate':
      return '百度翻译开放平台';
    case 'baidu-ocr':
      return '百度智能云 OCR';
    case 'system-ocr':
      return '系统内置文字识别';
    case 'tesseract':
      return '本地 Tesseract 引擎';
    case 'system-tts':
      return '系统语音合成';
    default:
      return provider.protocol ? `${protocolLabel(provider.protocol)} 兼容接口` : '已接入服务';
  }
}

function protocolLabel(protocol: string) {
  switch (protocol) {
    case 'openai':
      return 'OpenAI';
    case 'openai-responses':
      return 'OpenAI Responses';
    case 'anthropic':
      return 'Anthropic';
    case 'gemini':
      return 'Gemini';
    default:
      return protocol;
  }
}

function EditIcon() {
  return (
    <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M16.862 4.487 19.5 7.125M18 14.25V19.5H4.5V6h5.25m1.17 9.33 7.19-7.19a1.86 1.86 0 0 0-2.63-2.63l-7.19 7.19-.88 3.51 3.51-.88Z" />
    </svg>
  );
}

function PulseIcon() {
  return (
    <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M3 12h3.5l2-5 4 10 2.5-5H21" />
    </svg>
  );
}

function TrashIcon() {
  return (
    <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M6 7.5h12m-9 0V6a1.5 1.5 0 0 1 1.5-1.5h3A1.5 1.5 0 0 1 15 6v1.5m1.5 0-.63 11.34A1.5 1.5 0 0 1 14.38 20.25H9.62a1.5 1.5 0 0 1-1.49-1.41L7.5 7.5" />
    </svg>
  );
}
