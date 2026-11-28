import { useState, useEffect } from 'react';
import { useProviderStore } from '../../../stores/providerStore';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { ProviderCard } from './ProviderCard';
import { ProviderConfigDialog } from './ProviderConfigDialog';
import { useSettingsRuntime } from '../runtimeContext';
import { ProviderDragHandle, useProviderSort } from './providerSorting';

export function OcrProvidersPage() {
  const runtime = useSettingsRuntime();
  const gpuAcceleration = useSettingsConfigStore((state) => state.ocr?.gpuAcceleration);
  const providers = useProviderStore((state) => state.ocrProviders);
  const activeProvider = useProviderStore((state) => state.activeOcrProvider);
  const loadProviders = useProviderStore((state) => state.loadOcrProviders);
  const activateProvider = useProviderStore((state) => state.activateOcrProvider);
  const configureProvider = useProviderStore((state) => state.configureOcrProvider);
  const reorderProviders = useProviderStore((state) => state.reorderOcrProviders);
  const sort = useProviderSort({
    ids: providers.map((provider) => provider.id),
    onReorder: (ids) => reorderProviders(ids),
  });

  const [configuringProvider, setConfiguringProvider] = useState<string | null>(null);

  useEffect(() => {
    loadProviders();
  }, [loadProviders, gpuAcceleration]);

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
      <div ref={sort.listRef} className="provider-sort-list" aria-label="OCR 服务排序">
        {sort.orderedIds.map((id) => {
          const provider = providers.find((provider) => provider.id === id)!;
          return (
            <div key={provider.id} className="provider-sort-row" {...sort.rowProps(provider.id)}>
              <ProviderCard
                provider={provider}
                onActivate={() => handleActivate(provider.id)}
                onConfigure={() => handleConfigure(provider.id)}
                highlighted={provider.id === activeProvider}
                leadingSlot={<ProviderDragHandle {...sort.handleProps(provider.id)} />}
              />
            </div>
          );
        })}
      </div>
      <span className="sr-only" role="status">{sort.announcement}</span>

      <ProviderConfigDialog
        isOpen={configuringProvider !== null}
        onClose={() => setConfiguringProvider(null)}
        onSave={handleSaveConfig}
        provider={currentProvider || null}
        loadCredentialSchema={runtime.providers.getOcrCredentialSchema}
      />
    </div>
  );
}
