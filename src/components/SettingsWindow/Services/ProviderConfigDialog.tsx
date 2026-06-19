import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Provider } from '../../../stores/providerStore';

interface CredentialField {
  name: string;
  label: string;
  secret: boolean;
}

interface ProviderConfigDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (credentials: Record<string, string>) => void;
  provider: Provider | null;
}

export function ProviderConfigDialog({ isOpen, onClose, onSave, provider }: ProviderConfigDialogProps) {
  const [fields, setFields] = useState<CredentialField[]>([]);
  const [credentials, setCredentials] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!isOpen || !provider) return;

    // Load credential schema for this provider
    setLoading(true);
    invoke<CredentialField[]>('get_provider_credential_schema', {
      providerId: provider.id,
    })
      .then((schema) => {
        setFields(schema);

        // Initialize empty credentials
        const initialCreds: Record<string, string> = {};
        schema.forEach((field) => {
          initialCreds[field.name] = '';
        });
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
  }, [isOpen, provider]);

  const handleSave = () => {
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

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={handleClose}>
      <div
        className="bg-white rounded-xl shadow-2xl w-[500px] max-h-[80vh] overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="p-6 border-b border-gray-200">
          <h3 className="text-xl font-bold text-gray-900">配置 {provider.name}</h3>
          <p className="text-sm text-gray-600 mt-1">{provider.description}</p>
        </div>

        <div className="p-6 space-y-6">
          {loading ? (
            <div className="text-center py-8 text-gray-500">加载中...</div>
          ) : (
            <>
              {fields.map((field) => (
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
              ))}

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

              {provider.id === 'deepl' && (
                <p className="text-xs text-gray-500">
                  获取地址：<a href="https://www.deepl.com/pro-api" target="_blank" rel="noopener noreferrer" className="text-primary-600 hover:underline">https://www.deepl.com/pro-api</a>
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
            disabled={loading || fields.some((f) => !credentials[f.name]?.trim())}
            className="px-6 py-2 text-sm bg-primary-600 hover:bg-blue-700 text-white rounded-lg transition-colors disabled:bg-gray-300 disabled:cursor-not-allowed"
          >
            保存配置
          </button>
        </div>
      </div>
    </div>
  );
}
