import { useState } from 'react';

interface AddCustomTranslationProviderRequest {
  name: string;
  protocol: string;
  endpoint: string;
  model: string;
  api_key: string;
  reasoning_level?: string;
}

interface CustomTranslationProviderDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (request: AddCustomTranslationProviderRequest) => void;
}

export function CustomTranslationProviderDialog({
  isOpen,
  onClose,
  onSave,
}: CustomTranslationProviderDialogProps) {
  const [name, setName] = useState('');
  const [protocol, setProtocol] = useState<'openai' | 'anthropic' | 'gemini'>('openai');
  const [endpoint, setEndpoint] = useState('https://api.openai.com');
  const [model, setModel] = useState('gpt-4o');
  const [apiKey, setApiKey] = useState('');
  const [reasoningLevel, setReasoningLevel] = useState<string>('');

  const resetAndClose = () => {
    setName('');
    setProtocol('openai');
    setEndpoint('https://api.openai.com');
    setModel('gpt-4o');
    setApiKey('');
    setReasoningLevel('');
    onClose();
  };

  const handleSave = () => {
    const trimmedName = name.trim();
    const trimmedApiKey = apiKey.trim();
    const trimmedEndpoint = endpoint.trim();
    const trimmedModel = model.trim();

    if (!trimmedName || !trimmedApiKey || !trimmedEndpoint || !trimmedModel) return;

    onSave({
      name: trimmedName,
      protocol,
      endpoint: trimmedEndpoint,
      model: trimmedModel,
      api_key: trimmedApiKey,
      reasoning_level: reasoningLevel || undefined,
    });
    resetAndClose();
  };

  const handleProtocolChange = (newProtocol: 'openai' | 'anthropic' | 'gemini') => {
    setProtocol(newProtocol);
    // 自动填充默认值
    switch (newProtocol) {
      case 'openai':
        setEndpoint('https://api.openai.com');
        setModel('gpt-4o');
        break;
      case 'anthropic':
        setEndpoint('https://api.anthropic.com');
        setModel('claude-3-5-sonnet-latest');
        break;
      case 'gemini':
        setEndpoint('https://generativelanguage.googleapis.com');
        setModel('gemini-1.5-flash');
        break;
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-2xl shadow-xl p-6 w-full max-w-md">
        <h3 className="text-xl font-semibold text-gray-900 mb-4">添加自定义翻译服务</h3>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">名称</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="例如：我的 GPT-4"
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">协议</label>
            <select
              value={protocol}
              onChange={(e) => handleProtocolChange(e.target.value as any)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
            >
              <option value="openai">OpenAI</option>
              <option value="anthropic">Anthropic (Claude)</option>
              <option value="gemini">Google Gemini</option>
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">API Endpoint</label>
            <input
              type="text"
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
              placeholder="https://api.openai.com"
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">模型</label>
            <input
              type="text"
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder="gpt-4o / claude-3-5-sonnet-latest / gemini-1.5-flash"
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">API Key</label>
            <input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="sk-..."
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Reasoning 强度 <span className="text-gray-400 text-xs">(可选)</span>
            </label>
            <select
              value={reasoningLevel}
              onChange={(e) => setReasoningLevel(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
            >
              <option value="">无（默认）</option>
              <option value="minimal">Minimal</option>
              <option value="low">Low</option>
              <option value="medium">Medium</option>
              <option value="high">High</option>
              <option value="xhigh">XHigh</option>
            </select>
            <p className="text-xs text-gray-500 mt-1">
              仅支持推理模型（o1/o3, Claude Sonnet 3.7+, Gemini 2.0+）
            </p>
          </div>
        </div>

        <div className="flex gap-3 mt-6">
          <button
            onClick={resetAndClose}
            className="flex-1 px-4 py-2 text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors"
          >
            取消
          </button>
          <button
            onClick={handleSave}
            disabled={!name.trim() || !apiKey.trim() || !endpoint.trim() || !model.trim()}
            className="flex-1 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
          >
            添加
          </button>
        </div>
      </div>
    </div>
  );
}
