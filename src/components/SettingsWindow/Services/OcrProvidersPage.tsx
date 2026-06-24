import { useState, useEffect } from 'react';
import { useProviderStore } from '../../../stores/providerStore';
import { ProviderCard } from './ProviderCard';
import { ProviderConfigDialog } from './ProviderConfigDialog';
import { getOcrProviderCredentialSchema } from '../../../tauri/providers';

export function OcrProvidersPage() {
  const providers = useProviderStore((state) => state.ocrProviders);
  const activeProvider = useProviderStore((state) => state.activeOcrProvider);
  const loadProviders = useProviderStore((state) => state.loadOcrProviders);
  const activateProvider = useProviderStore((state) => state.activateOcrProvider);
  const configureProvider = useProviderStore((state) => state.configureOcrProvider);

  const [configuringProvider, setConfiguringProvider] = useState<string | null>(null);

  useEffect(() => {
    loadProviders();
  }, [loadProviders]);

  const handleActivate = async (id: string) => {
    try {
      await activateProvider(id);
    } catch (error) {
      console.error('Failed to activate provider:', error);
    }
  };

  const handleConfigure = async (id: string) => {
    const provider = providers.find((p) => p.id === id);
    if (provider && !provider.requiresApiKey) {
      await activateProvider(id);
      return;
    }

    setConfiguringProvider(id);
  };

  const handleSaveConfig = async (credentials: Record<string, string>) => {
    if (configuringProvider) {
      try {
        await configureProvider(configuringProvider, credentials);
        // 配置完成后自动激活
        await activateProvider(configuringProvider);
      } catch (error) {
        console.error('Failed to configure provider:', error);
      }
    }
    setConfiguringProvider(null);
  };

  const handleTest = (_id: string) => {
    // TODO: 实现 Provider 测试
    alert('测试功能：将调用该 OCR 服务识别一张示例图片\n\n此功能待实现');
  };

  const currentProvider = providers.find((p) => p.id === configuringProvider);

  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">OCR 服务</h2>
        <p className="text-gray-600">
          当前激活：<span className="font-medium text-primary-600">
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
        loadCredentialSchema={getOcrProviderCredentialSchema}
      />
    </div>
  );
}
