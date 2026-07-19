import type { ReactNode } from 'react';

import { Provider } from '../../../stores/providerStore';
import IconActionButton from '../../../components/common/IconActionButton';

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
  const canToggle = provider.status !== 'unconfigured' && Boolean(onActivate || onDeactivate);
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
      className={`group/provider relative min-h-[72px] px-5 py-3.5 transition-colors duration-150 ${
        highlighted
          ? 'bg-primary-50/60 before:absolute before:inset-y-2 before:left-0 before:w-[3px] before:rounded-r before:bg-primary-500'
          : 'hover:bg-gray-50'
      }`}
    >
      <div className="flex h-full items-center gap-4">
        <div className="flex w-5 flex-shrink-0 justify-center text-gray-300">
          {leadingSlot ?? <DragDots className="opacity-70" />}
        </div>

        <div className={`flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-xl ${providerIconClass(provider.id)}`}>
          <span className="text-xs font-semibold">
            {providerInitial(provider.name)}
          </span>
        </div>

        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h3 className="truncate text-base font-semibold text-gray-900">
              {provider.name}
            </h3>
            {provider.status === 'unconfigured' ? (
              <span className="rounded-full bg-amber-50 px-2 py-0.5 text-[11px] font-medium text-amber-700">
                未配置
              </span>
            ) : null}
          </div>
          <p
            className={`mt-1 truncate text-sm ${
              provider.endpoint ? 'text-blue-500' : 'text-gray-500'
            }`}
          >
            {subtitle}
          </p>
        </div>

        <div className="flex flex-shrink-0 items-center justify-end gap-2 opacity-0 transition-opacity duration-150 group-hover/provider:opacity-100 group-focus-within/provider:opacity-100">
          {canToggle ? (
            <button
              type="button"
              onClick={handleToggle}
              className={`inline-flex h-9 items-center gap-1.5 rounded-lg px-3 text-sm font-medium transition-colors ${
                isActive
                  ? 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                  : 'bg-primary-600 text-white hover:bg-primary-700'
              }`}
            >
              <PlayIcon className="h-4 w-4" />
              {isActive ? '停用' : '启用'}
            </button>
          ) : null}

          {onConfigure ? (
            <IconActionButton
              title="编辑"
              onClick={onConfigure}
              className="inline-flex h-9 w-9 items-center justify-center rounded-lg text-gray-500 transition-colors hover:bg-gray-100 hover:text-gray-800"
            >
              <EditIcon />
            </IconActionButton>
          ) : null}

          {onTest ? (
            <IconActionButton
              title="测试联通"
              onClick={onTest}
              className="inline-flex h-9 w-9 items-center justify-center rounded-lg text-gray-500 transition-colors hover:bg-gray-100 hover:text-gray-800"
            >
              <PulseIcon />
            </IconActionButton>
          ) : null}

          {!provider.isBuiltin && onRemove ? (
            <IconActionButton
              title="删除"
              onClick={onRemove}
              className="inline-flex h-9 w-9 items-center justify-center rounded-lg text-gray-500 transition-colors hover:bg-red-50 hover:text-red-600"
            >
              <TrashIcon />
            </IconActionButton>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function DragDots({ className = '' }: { className?: string }) {
  return (
    <span className={`grid grid-cols-2 gap-0.5 ${className}`}>
      {Array.from({ length: 6 }).map((_, index) => (
        <span key={index} className="h-1 w-1 rounded-full bg-current" />
      ))}
    </span>
  );
}

function providerInitial(name: string) {
  const trimmed = name.trim();
  if (!trimmed) return '?';
  return trimmed.length > 1 && /^[a-z]+$/i.test(trimmed.slice(0, 2))
    ? trimmed.slice(0, 2).toUpperCase()
    : trimmed.charAt(0).toUpperCase();
}

function providerSubtitle(provider: Provider) {
  if (provider.endpoint) {
    return provider.endpoint;
  }

  if (provider.description) {
    return provider.description;
  }

  switch (provider.id) {
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

function providerIconClass(providerId: string) {
  switch (providerId) {
    case 'google-translate':
      return 'border border-blue-100 bg-blue-50 text-blue-500';
    case 'deeplx':
      return 'border border-slate-200 bg-slate-50 text-slate-600';
    case 'baidu-translate':
    case 'baidu-ocr':
      return 'border border-indigo-100 bg-indigo-50 text-indigo-500';
    case 'system-ocr':
      return 'border border-emerald-100 bg-emerald-50 text-emerald-600';
    case 'tesseract':
      return 'border border-gray-200 bg-gray-50 text-gray-600';
    case 'system-tts':
      return 'border border-orange-100 bg-orange-50 text-orange-600';
    default:
      return 'border border-gray-200 bg-gray-50 text-gray-600';
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

function PlayIcon({ className = '' }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M8 5.75v12.5L18 12 8 5.75Z" />
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
