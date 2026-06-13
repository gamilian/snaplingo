import { useState } from 'react';
import { Provider } from '../../../stores/providerStore';

interface ProviderConfigDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (config: any) => void;
  provider: Provider | null;
}

export function ProviderConfigDialog({ isOpen, onClose, onSave, provider }: ProviderConfigDialogProps) {
  const [apiKey, setApiKey] = useState(provider?.config?.apiKey || '');
  const [endpoint, setEndpoint] = useState(provider?.config?.endpoint || '');
  const [model, setModel] = useState(provider?.config?.model || '');

  const handleSave = () => {
    onSave({
      apiKey: apiKey.trim(),
      endpoint: endpoint.trim(),
      model: model.trim(),
    });
    onClose();
  };

  const handleClose = () => {
    setApiKey('');
    setEndpoint('');
    setModel('');
    onClose();
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
          {/* API Key */}
          {provider.requiresApiKey && (
            <div>
              <label className="block font-medium text-gray-700 mb-2">
                API Key <span className="text-red-500">*</span>
              </label>
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="请输入 API Key"
                className="w-full px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
              <p className="text-xs text-gray-500 mt-2">
                {provider.id === 'deepl' && '获取地址：https://www.deepl.com/pro-api'}
                {provider.id === 'baidu-ocr' && '获取地址：https://cloud.baidu.com/product/ocr'}
                {provider.id === 'baidu-translate' && '获取地址：https://fanyi-api.baidu.com/'}
              </p>
            </div>
          )}

          {/* API Endpoint（可选） */}
          <div>
            <label className="block font-medium text-gray-700 mb-2">
              API Endpoint <span className="text-gray-400 text-sm">(可选)</span>
            </label>
            <input
              type="text"
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
              placeholder="自定义 API 端点（留空使用默认）"
              className="w-full px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <p className="text-xs text-gray-500 mt-2">仅在使用自定义端点时填写</p>
          </div>

          {/* Model（可选） */}
          {provider.type === 'translation' && (
            <div>
              <label className="block font-medium text-gray-700 mb-2">
                模型 <span className="text-gray-400 text-sm">(可选)</span>
              </label>
              <input
                type="text"
                value={model}
                onChange={(e) => setModel(e.target.value)}
                placeholder="指定模型名称（留空使用默认）"
                className="w-full px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
              <p className="text-xs text-gray-500 mt-2">例如：gpt-4, claude-3-opus 等</p>
            </div>
          )}

          {/* 提示信息 */}
          <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
            <div className="flex items-start space-x-3">
              <svg
                className="w-5 h-5 text-blue-600 flex-shrink-0 mt-0.5"
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
                <p>API Key 将加密保存在本地，不会上传到任何服务器</p>
              </div>
            </div>
          </div>
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
            disabled={provider.requiresApiKey && !apiKey.trim()}
            className="px-6 py-2 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors disabled:bg-gray-300 disabled:cursor-not-allowed"
          >
            保存配置
          </button>
        </div>
      </div>
    </div>
  );
}
