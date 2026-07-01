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

  const currentProvider = providers.find((p) => p.id === configuringProvider);

  return (
    <div className="max-w-5xl space-y-3">
      <div className="space-y-3">
        {providers.map((provider) => (
          <ProviderCard
            key={provider.id}
            provider={provider}
            onActivate={() => handleActivate(provider.id)}
            onConfigure={() => handleConfigure(provider.id)}
            highlighted={provider.id === activeProvider}
          />
        ))}
      </div>

      <ProviderConfigDialog
        isOpen={configuringProvider !== null}
        presentation="inline"
        onClose={() => setConfiguringProvider(null)}
        onSave={handleSaveConfig}
        provider={currentProvider || null}
        loadCredentialSchema={getOcrProviderCredentialSchema}
      />
    </div>
  );
}
