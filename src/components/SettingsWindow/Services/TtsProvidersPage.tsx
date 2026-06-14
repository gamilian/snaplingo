import { useState } from 'react';
import { useProviderStore } from '../../../stores/providerStore';
import { ProviderCard } from './ProviderCard';
import { ProviderConfigDialog } from './ProviderConfigDialog';

export function TtsProvidersPage() {
  const providers = useProviderStore((state) => state.ttsProviders);
  const activeProvider = useProviderStore((state) => state.activeTtsProvider);
  const activateProvider = useProviderStore((state) => state.activateTtsProvider);
  const updateProviderConfig = useProviderStore((state) => state.updateProviderConfig);

  const [configuringProvider, setConfiguringProvider] = useState<string | null>(null);

  const handleActivate = (id: string) => {
    activateProvider(id);
  };

  const handleConfigure = (id: string) => {
    setConfiguringProvider(id);
  };

  const handleSaveConfig = async (config: any) => {
    if (configuringProvider) {
      await updateProviderConfig(configuringProvider, configuringProvider, config);
      activateProvider(configuringProvider);
    }
    setConfiguringProvider(null);
  };

  const handleTest = (_id: string) => {
    // TODO: 实现 Provider 测试
    alert('测试功能：将使用该 TTS 服务朗读一段示例文本\n\n此功能待实现');
  };

  const currentProvider = providers.find((p) => p.id === configuringProvider);

  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">语音合成</h2>
        <p className="text-gray-600">
          当前激活：<span className="font-medium text-blue-600">
            {providers.find((p) => p.id === activeProvider)?.name || '无'}
          </span>
        </p>
      </div>

      <div className="space-y-4">
        {providers.map((provider) => (
          <ProviderCard
            key={provider.id}
            provider={provider}
            onActivate={() => handleActivate(provider.id)}
            onConfigure={() => handleConfigure(provider.id)}
            onTest={() => handleTest(provider.id)}
          />
        ))}
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
