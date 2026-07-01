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
      />
    </div>
  );
}
