import { useState } from 'react';
import { useProviderStore } from '../../../stores/providerStore';
import { ProviderCard } from './ProviderCard';
import { ProviderConfigDialog } from './ProviderConfigDialog';

export function TranslationProvidersPage() {
  const providers = useProviderStore((state) => state.translationProviders);
  const activeProviders = useProviderStore((state) => state.activeTranslationProviders);
  const activateProvider = useProviderStore((state) => state.activateTranslationProvider);
  const deactivateProvider = useProviderStore((state) => state.deactivateTranslationProvider);
  const updateProviderConfig = useProviderStore((state) => state.updateProviderConfig);

  const [configuringProvider, setConfiguringProvider] = useState<string | null>(null);

  const handleActivate = (id: string) => {
    activateProvider(id);
  };

  const handleDeactivate = (id: string) => {
    deactivateProvider(id);
  };

  const handleConfigure = (id: string) => {
    setConfiguringProvider(id);
  };

  const handleSaveConfig = (config: any) => {
    if (configuringProvider) {
      updateProviderConfig(configuringProvider, config);
      // 配置完成后自动激活
      activateProvider(configuringProvider);
    }
    setConfiguringProvider(null);
  };

  const handleTest = (_id: string) => {
    // TODO: 实现 Provider 测试
    alert('测试功能：将使用该翻译服务翻译一段示例文本\n\n此功能待实现');
  };

  const handleAddCustom = () => {
    // TODO: 打开添加自定义服务对话框
    alert('添加自定义翻译服务功能\n\n支持添加兼容 OpenAI / Claude / Gemini API 的服务\n\n此功能待实现');
  };

  const currentProvider = providers.find((p) => p.id === configuringProvider);

  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">翻译服务</h2>
        <p className="text-gray-600">
          已激活：
          {activeProviders.length > 0 ? (
            activeProviders.map((id) => {
              const provider = providers.find((p) => p.id === id);
              return provider ? (
                <span key={id} className="ml-2 px-2 py-0.5 text-xs bg-green-100 text-green-700 rounded font-medium">
                  {provider.name} ✓
                </span>
              ) : null;
            })
          ) : (
            <span className="ml-2 text-gray-500">无</span>
          )}
        </p>
        <p className="text-sm text-gray-500 mt-1">支持同时激活多个翻译服务，结果会并行显示</p>
      </div>

      <div className="space-y-4">
        {providers.map((provider) => (
          <ProviderCard
            key={provider.id}
            provider={provider}
            onActivate={() => handleActivate(provider.id)}
            onDeactivate={() => handleDeactivate(provider.id)}
            onConfigure={() => handleConfigure(provider.id)}
            onTest={() => handleTest(provider.id)}
          />
        ))}
      </div>

      <div className="pt-4 border-t border-gray-200">
        <button
          onClick={handleAddCustom}
          className="px-4 py-2 text-sm text-blue-600 hover:bg-blue-50 rounded-lg transition-colors font-medium"
        >
          + 添加自定义服务
        </button>
        <p className="text-xs text-gray-500 mt-2">支持添加兼容 OpenAI / Claude / Gemini API 的自定义翻译服务</p>
      </div>

      <ProviderConfigDialog
        isOpen={configuringProvider !== null}
        onClose={() => setConfiguringProvider(null)}
        onSave={handleSaveConfig}
        provider={currentProvider || null}
      />
    </div>
  );
}
