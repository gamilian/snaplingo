import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import type { CredentialField } from '../../../application/settings/ports';
import { Provider } from '../../../stores/providerStore';
import IconActionButton from '../../../components/common/IconActionButton';
import { ProviderBrandIcon } from '../../../components/common/ProviderBrandIcon';
import { useSettingsRuntime } from '../runtimeContext';

interface ProviderConfigDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (credentials: Record<string, string>) => void;
  provider: Provider | null;
  loadCredentialSchema?: (providerId: string) => Promise<CredentialField[]>;
  presentation?: 'dialog' | 'inline';
}

export function ProviderConfigDialog({
  isOpen,
  onClose,
  onSave,
  provider,
  loadCredentialSchema,
  presentation = 'dialog',
}: ProviderConfigDialogProps) {
  const runtime = useSettingsRuntime();
  const credentialSchemaLoader =
    loadCredentialSchema ?? runtime.providers.getTranslationCredentialSchema;
  const [fields, setFields] = useState<CredentialField[]>([]);
  const [credentials, setCredentials] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!isOpen || !provider) return;

    // Load credential schema for this provider
    setLoading(true);
    credentialSchemaLoader(provider.id)
      .then((schema) => {
        setFields(schema);

        // Initialize empty credentials
        const initialCreds: Record<string, string> = {};
        schema.forEach((field) => {
          initialCreds[field.name] = '';
        });
        if (provider.id === 'deeplx') {
          initialCreds.mode = 'deeplx';
        }
        setCredentials(initialCreds);
      })
      .catch((error) => {
        console.error('Failed to load credential schema:', error);
        alert(`加载凭证配置失败: ${error}`);
        onClose();
      })
      .finally(() => {
        setLoading(false);
      });
  }, [credentialSchemaLoader, isOpen, onClose, provider]);

  useEffect(() => {
    if (!isOpen || presentation !== 'dialog' || typeof window === 'undefined') return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        handleClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, presentation]);

  const handleSave = () => {
    if (provider?.id === 'deeplx') {
      const mode = credentials.mode === 'deepl' ? 'deepl' : 'deeplx';
      if (mode === 'deepl') {
        const apiKey = credentials.api_key?.trim();
        if (!apiKey) {
          alert('请填写：DeepL API Key');
          return;
        }
        onSave({ mode, api_key: apiKey });
      } else {
        const endpoint = credentials.endpoint?.trim();
        if (!endpoint) {
          alert('请填写：DeepLX API 地址');
          return;
        }
        onSave({ mode, endpoint });
      }
      handleClose();
      return;
    }

    // Validate all fields are filled
    for (const field of fields) {
      if (!credentials[field.name]?.trim()) {
        alert(`请填写：${field.label}`);
        return;
      }
    }

    onSave(credentials);
    handleClose();
  };

  const handleClose = () => {
    setFields([]);
    setCredentials({});
    onClose();
  };

  const updateCredential = (fieldName: string, value: string) => {
    setCredentials((prev) => ({
      ...prev,
      [fieldName]: value,
    }));
  };

  if (!isOpen || !provider) return null;

  const form = (
    <section
      role={presentation === 'dialog' ? 'dialog' : undefined}
      aria-modal={presentation === 'dialog' ? true : undefined}
      aria-label={`配置 ${provider.name}`}
      className={presentation === 'inline' ? 'provider-dialog provider-dialog-inline' : 'provider-dialog provider-dialog-compact'}
      onClick={(e) => e.stopPropagation()}
    >
      <header className="provider-dialog-header">
        <div className="flex min-w-0 items-center gap-3">
          {presentation === 'inline' ? (
            <IconActionButton
              onClick={handleClose}
              title="返回供应商列表"
              tooltipPlacement="bottom"
              className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-line bg-surface text-muted transition-colors hover:bg-sunken"
            >
              <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
              </svg>
            </IconActionButton>
          ) : (
            <ProviderBrandIcon {...provider} />
          )}
          <div className="min-w-0">
            <h3 className="truncate text-[16px] font-bold tracking-[-0.02em] text-ink">配置 {provider.name}</h3>
            {provider.description && <p className="mt-0.5 text-xs text-muted">{provider.description}</p>}
          </div>
        </div>
        {presentation === 'dialog' && (
          <button
            type="button"
            onClick={handleClose}
            aria-label="关闭"
            className="grid h-8 w-8 shrink-0 place-items-center rounded-lg text-faint transition-colors hover:bg-raised hover:text-ink"
          >
            ×
          </button>
        )}
      </header>

      <div className="provider-dialog-body">
        {loading ? (
          <div className="py-8 text-center text-xs text-muted">加载中...</div>
        ) : (
          <>
            {provider.id === 'deeplx' ? (
              <DeepLXCredentialFields
                credentials={credentials}
                onChange={updateCredential}
              />
            ) : (
              fields.map((field) => (
                <label key={field.name} className="provider-dialog-field">
                  <span>{field.label} <em className="not-italic text-danger">*</em></span>
                  <input
                    type={field.secret ? 'password' : 'text'}
                    value={credentials[field.name] || ''}
                    onChange={(e) => updateCredential(field.name, e.target.value)}
                    placeholder={`请输入 ${field.label}`}
                    className="provider-dialog-input"
                  />
                </label>
              ))
            )}

            {provider.id === 'baidu-translate' && (
              <p className="text-xs text-muted">
                获取地址：<a href="https://fanyi-api.baidu.com/" target="_blank" rel="noopener noreferrer" className="text-accent-text hover:underline">https://fanyi-api.baidu.com/</a>
              </p>
            )}

            {provider.id === 'baidu-ocr' && (
              <p className="text-xs text-muted">
                获取地址：<a href="https://cloud.baidu.com/product/ocr" target="_blank" rel="noopener noreferrer" className="text-accent-text hover:underline">https://cloud.baidu.com/product/ocr</a>
              </p>
            )}

            {provider.id === 'deeplx' && (
              <p className="text-xs text-muted">
                项目地址：<a href="https://github.com/OwO-Network/DeepLX" target="_blank" rel="noopener noreferrer" className="text-accent-text hover:underline">https://github.com/OwO-Network/DeepLX</a>
              </p>
            )}

            <div className="rounded-lg border border-line bg-sunken px-3.5 py-2.5 text-xs leading-relaxed text-muted">
              <span className="font-medium text-ink">安全提示：</span>凭证仅保存在本机 SnapLingo 数据库中，不会上传到任何服务器。
            </div>
          </>
        )}
      </div>

      <footer className="provider-dialog-footer justify-end">
        <button
          type="button"
          onClick={handleClose}
          className="h-8 rounded-lg border border-line bg-surface px-3.5 text-xs font-medium text-muted transition-colors hover:bg-raised hover:text-ink"
        >
          取消
        </button>
        <button
          type="button"
          onClick={handleSave}
          disabled={loading || isSaveDisabled(provider, fields, credentials)}
          className="h-8 rounded-lg bg-accent px-4 text-xs font-semibold text-accent-on transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:bg-line-strong"
        >
          保存配置
        </button>
      </footer>
    </section>
  );

  if (presentation === 'inline') {
    return form;
  }

  const backdrop = (
    <div
      className="provider-dialog-backdrop"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) handleClose();
      }}
    >
      {form}
    </div>
  );
  return typeof document === 'undefined' || !document.body ? backdrop : createPortal(backdrop, document.body);
}

function DeepLXCredentialFields({
  credentials,
  onChange,
}: {
  credentials: Record<string, string>;
  onChange: (fieldName: string, value: string) => void;
}) {
  const isStandardDeepL = credentials.mode === 'deepl';

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-4 rounded-lg border border-line bg-sunken px-4 py-3">
        <div>
          <div className="font-medium text-ink">标准 DeepL</div>
          <div className="text-xs text-muted">关闭时使用 DeepLX 服务地址</div>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={isStandardDeepL}
          onClick={() => onChange('mode', isStandardDeepL ? 'deeplx' : 'deepl')}
          className={`relative h-6 w-11 rounded-full transition-colors ${
            isStandardDeepL ? 'bg-accent' : 'bg-line-strong'
          }`}
        >
          <span
            className={`absolute left-0 top-0.5 h-5 w-5 rounded-full bg-surface shadow transition-transform ${
              isStandardDeepL ? 'translate-x-5' : 'translate-x-0.5'
            }`}
          />
        </button>
      </div>

      {isStandardDeepL ? (
        <div>
          <label className="block font-medium text-ink mb-2">
            DeepL API Key <span className="text-red-500">*</span>
          </label>
          <input
            type="password"
            value={credentials.api_key || ''}
            onChange={(e) => onChange('api_key', e.target.value)}
            placeholder="请输入 DeepL API Key"
            className="w-full px-4 py-2 border border-line-strong rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-accent"
          />
        </div>
      ) : (
        <div>
          <label className="block font-medium text-ink mb-2">
            DeepLX API 地址 <span className="text-red-500">*</span>
          </label>
          <input
            type="text"
            value={credentials.endpoint || ''}
            onChange={(e) => onChange('endpoint', e.target.value)}
            placeholder="例如：https://deeplx.example.com"
            className="w-full px-4 py-2 border border-line-strong rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-accent"
          />
        </div>
      )}
    </div>
  );
}

function isSaveDisabled(
  provider: Provider | null,
  fields: CredentialField[],
  credentials: Record<string, string>,
): boolean {
  if (provider?.id === 'deeplx') {
    return credentials.mode === 'deepl'
      ? !credentials.api_key?.trim()
      : !credentials.endpoint?.trim();
  }

  return fields.some((field) => !credentials[field.name]?.trim());
}
