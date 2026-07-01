import { useEffect, useState } from 'react';
import { getProviderCredentialSchema } from '../../../tauri/providers';
import type { CredentialField } from '../../../tauri/providers';
import { Provider } from '../../../stores/providerStore';

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
  loadCredentialSchema = getProviderCredentialSchema,
  presentation = 'dialog',
}: ProviderConfigDialogProps) {
  const [fields, setFields] = useState<CredentialField[]>([]);
  const [credentials, setCredentials] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!isOpen || !provider) return;

    // Load credential schema for this provider
    setLoading(true);
    loadCredentialSchema(provider.id)
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
  }, [isOpen, provider, loadCredentialSchema]);

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
    <div
      className={
        presentation === 'inline'
          ? 'w-full'
          : 'bg-white rounded-xl shadow-2xl w-[500px] max-h-[80vh] overflow-y-auto'
      }
      onClick={(e) => e.stopPropagation()}
    >
        <div className="p-6 border-b border-gray-200">
          <div className="flex items-center gap-3">
            {presentation === 'inline' && (
              <button
                type="button"
                onClick={handleClose}
                aria-label="返回供应商列表"
                title="返回供应商列表"
                className="flex h-9 w-9 items-center justify-center rounded-full border border-gray-200 bg-white text-gray-600 transition-colors hover:bg-gray-50"
              >
                <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
                </svg>
              </button>
            )}
            <h3 className="text-xl font-bold text-gray-900">配置 {provider.name}</h3>
          </div>
          <p className="text-sm text-gray-600 mt-1">{provider.description}</p>
        </div>

        <div className="p-6 space-y-6">
          {loading ? (
            <div className="text-center py-8 text-gray-500">加载中...</div>
          ) : (
            <>
              {provider.id === 'deeplx' ? (
                <DeepLXCredentialFields
                  credentials={credentials}
                  onChange={updateCredential}
                />
              ) : (
                fields.map((field) => (
                  <div key={field.name}>
                    <label className="block font-medium text-gray-700 mb-2">
                      {field.label} <span className="text-red-500">*</span>
                    </label>
                    <input
                      type={field.secret ? 'password' : 'text'}
                      value={credentials[field.name] || ''}
                      onChange={(e) => updateCredential(field.name, e.target.value)}
                      placeholder={`请输入 ${field.label}`}
                      className="w-full px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
                    />
                  </div>
                ))
              )}

              {/* Baidu 特殊提示 */}
              {provider.id === 'baidu-translate' && (
                <p className="text-xs text-gray-500">
                  获取地址：<a href="https://fanyi-api.baidu.com/" target="_blank" rel="noopener noreferrer" className="text-primary-600 hover:underline">https://fanyi-api.baidu.com/</a>
                </p>
              )}

              {provider.id === 'baidu-ocr' && (
                <p className="text-xs text-gray-500">
                  获取地址：<a href="https://cloud.baidu.com/product/ocr" target="_blank" rel="noopener noreferrer" className="text-primary-600 hover:underline">https://cloud.baidu.com/product/ocr</a>
                </p>
              )}

              {provider.id === 'deeplx' && (
                <p className="text-xs text-gray-500">
                  项目地址：<a href="https://github.com/OwO-Network/DeepLX" target="_blank" rel="noopener noreferrer" className="text-primary-600 hover:underline">https://github.com/OwO-Network/DeepLX</a>
                </p>
              )}

              {/* 提示信息 */}
              <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
                <div className="flex items-start space-x-3">
                  <svg
                    className="w-5 h-5 text-primary-600 flex-shrink-0 mt-0.5"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                    />
                  </svg>
                  <div className="text-sm text-blue-800">
                    <p className="font-medium mb-1">安全提示</p>
                    <p>凭证将加密保存在系统密钥链中，不会上传到任何服务器</p>
                  </div>
                </div>
              </div>
            </>
          )}
        </div>

        <div className="p-6 border-t border-gray-200 flex items-center justify-between">
          <button
            onClick={handleClose}
            className="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded-lg transition-colors"
          >
            取消
          </button>
          <button
            onClick={handleSave}
            disabled={loading || isSaveDisabled(provider, fields, credentials)}
            className="px-6 py-2 text-sm bg-primary-600 hover:bg-blue-700 text-white rounded-lg transition-colors disabled:bg-gray-300 disabled:cursor-not-allowed"
          >
            保存配置
          </button>
        </div>
    </div>
  );

  if (presentation === 'inline') {
    return form;
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={handleClose}>
      {form}
    </div>
  );
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
      <div className="flex items-center justify-between gap-4 rounded-lg border border-gray-200 bg-gray-50 px-4 py-3">
        <div>
          <div className="font-medium text-gray-800">标准 DeepL</div>
          <div className="text-xs text-gray-500">关闭时使用 DeepLX 服务地址</div>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={isStandardDeepL}
          onClick={() => onChange('mode', isStandardDeepL ? 'deeplx' : 'deepl')}
          className={`relative h-6 w-11 rounded-full transition-colors ${
            isStandardDeepL ? 'bg-primary-600' : 'bg-gray-300'
          }`}
        >
          <span
            className={`absolute left-0 top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform ${
              isStandardDeepL ? 'translate-x-5' : 'translate-x-0.5'
            }`}
          />
        </button>
      </div>

      {isStandardDeepL ? (
        <div>
          <label className="block font-medium text-gray-700 mb-2">
            DeepL API Key <span className="text-red-500">*</span>
          </label>
          <input
            type="password"
            value={credentials.api_key || ''}
            onChange={(e) => onChange('api_key', e.target.value)}
            placeholder="请输入 DeepL API Key"
            className="w-full px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
        </div>
      ) : (
        <div>
          <label className="block font-medium text-gray-700 mb-2">
            DeepLX API 地址 <span className="text-red-500">*</span>
          </label>
          <input
            type="text"
            value={credentials.endpoint || ''}
            onChange={(e) => onChange('endpoint', e.target.value)}
            placeholder="例如：https://deeplx.example.com"
            className="w-full px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
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
