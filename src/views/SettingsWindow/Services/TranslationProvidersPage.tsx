import { useState, useEffect } from 'react';
import { useProviderStore } from '../../../stores/providerStore';
import { ProviderCard } from './ProviderCard';
import { ProviderConfigDialog } from './ProviderConfigDialog';
import { CustomTranslationProviderDialog } from './CustomTranslationProviderDialog';
import { ProviderDragHandle, useProviderSort } from './providerSorting';

export function TranslationProvidersPage({ addRequest = 0, showAddAction = true }: { addRequest?: number; showAddAction?: boolean } = {}) {
  const providers = useProviderStore((state) => state.translationProviders);
  const activeProviders = useProviderStore((state) => state.activeTranslationProviders);
  const loadProviders = useProviderStore((state) => state.loadTranslationProviders);
  const activateProvider = useProviderStore((state) => state.activateTranslationProvider);
  const deactivateProvider = useProviderStore((state) => state.deactivateTranslationProvider);
  const updateProviderConfig = useProviderStore((state) => state.updateProviderConfig);
  const addCustomProvider = useProviderStore((state) => state.addCustomTranslationProvider);
  const updateCustomProvider = useProviderStore((state) => state.updateCustomTranslationProvider);
  const removeProvider = useProviderStore((state) => state.removeTranslationProvider);
  const testCustomProvider = useProviderStore((state) => state.testCustomTranslationProvider);
  const reorderProviders = useProviderStore((state) => state.reorderTranslationProviders);
  const reorderInactiveProviders = useProviderStore((state) => state.reorderInactiveTranslationProviders);

  const [configuringProvider, setConfiguringProvider] = useState<string | null>(null);
  const [addingCustomProvider, setAddingCustomProvider] = useState(false);
  useEffect(() => {
    if (addRequest > 0) { setConfiguringProvider(null); setAddingCustomProvider(true); }
  }, [addRequest]);
  const activeProvidersList = activeProviders
    .map((id) => providers.find((provider) => provider.id === id))
    .filter((provider): provider is NonNullable<typeof provider> => provider !== undefined);
  const inactiveProvidersList = providers.filter((provider) => !activeProviders.includes(provider.id));
  const orderedProviders = [...activeProvidersList, ...inactiveProvidersList];
  const sort = useProviderSort({
    ids: orderedProviders.map((provider) => provider.id),
    groupForId: (id) => activeProviders.includes(id) ? 'active' : 'inactive',
    onReorder: async (ids, draggedId) => {
      if (activeProviders.includes(draggedId)) {
        await reorderProviders(ids.filter((id) => activeProviders.includes(id)));
      } else {
        await reorderInactiveProviders(ids.filter((id) => !activeProviders.includes(id)));
      }
    },
  });

  // 加载 providers
  useEffect(() => {
    loadProviders();
  }, [loadProviders]);

  const handleActivate = (id: string) => {
    activateProvider(id);
  };

  const handleDeactivate = (id: string) => {
    deactivateProvider(id);
  };

  const handleConfigure = (id: string) => {
    setConfiguringProvider(id);
  };

  const handleSaveConfig = async (config: any) => {
    if (configuringProvider) {
      await updateProviderConfig(configuringProvider, configuringProvider, config);
      // 配置完成后自动激活
      await activateProvider(configuringProvider);
    }
    setConfiguringProvider(null);
  };

  const handleTest = async (id: string) => {
    try {
      await testCustomProvider(id);
      alert('检测成功');
    } catch (error) {
      alert(`检测失败: ${error}`);
    }
  };

  const handleRemove = async (id: string) => {
    if (confirm('确定要删除这个自定义翻译服务吗？')) {
      try {
        await removeProvider(id);
      } catch (error) {
        alert(`删除失败: ${error}`);
      }
    }
  };

  const handleAddCustom = () => {
    setAddingCustomProvider(true);
  };

  const handleSaveCustomProvider = async (request: any) => {
    await addCustomProvider(request);
  };

  const handleUpdateCustomProvider = async (providerId: string, request: any) => {
    await updateCustomProvider(providerId, request);
    await activateProvider(providerId);
    setConfiguringProvider(null);
  };

  const currentProvider = providers.find((p) => p.id === configuringProvider);
  const isConfiguringCustomProvider = Boolean(
    currentProvider && !currentProvider.isBuiltin,
  );

  return (
    <div className="max-w-5xl">
      <div ref={sort.listRef} className="provider-sort-list" aria-label="翻译服务排序">
        {sort.orderedIds.map((id) => {
          const provider = providers.find((provider) => provider.id === id)!;
          const isActive = activeProviders.includes(provider.id);

          return (
            <div
              key={provider.id}
              data-translation-provider-id={provider.id}
              className="provider-sort-row"
              {...sort.rowProps(provider.id)}
            >
              <ProviderCard
                provider={provider}
                onActivate={() => handleActivate(provider.id)}
                onDeactivate={() => handleDeactivate(provider.id)}
                onConfigure={() => handleConfigure(provider.id)}
                onTest={!provider.isBuiltin ? () => handleTest(provider.id) : undefined}
                onRemove={() => handleRemove(provider.id)}
                highlighted={isActive}
                leadingSlot={<ProviderDragHandle {...sort.handleProps(provider.id)} />}
              />
            </div>
          );
        })}
      </div>
      <span className="sr-only" role="status">{sort.announcement}</span>

      {showAddAction && <button
        type="button"
        title="添加 Provider"
        aria-label="添加 Provider"
        onClick={handleAddCustom}
        className="mt-3 inline-flex h-9 items-center gap-2 rounded-lg border border-line bg-surface px-3.5 text-xs font-semibold text-muted transition-colors hover:border-accent-line hover:text-accent-text focus:outline-none focus:ring-2 focus:ring-accent-line"
      >
        <PlusIcon />
        <span>添加 Provider</span>
      </button>}

      {currentProvider && !isConfiguringCustomProvider && (
        <ProviderConfigDialog
          isOpen
          onClose={() => setConfiguringProvider(null)}
          onSave={handleSaveConfig}
          provider={currentProvider}
        />
      )}

      {(addingCustomProvider || isConfiguringCustomProvider) && (
        <CustomTranslationProviderDialog
          isOpen
          onClose={() => {
            setAddingCustomProvider(false);
            setConfiguringProvider(null);
          }}
          onSave={handleSaveCustomProvider}
          onUpdate={handleUpdateCustomProvider}
          initialProvider={isConfiguringCustomProvider ? currentProvider : null}
        />
      )}
    </div>
  );
}

function PlusIcon() {
  return (
    <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M12 5v14m7-7H5" />
    </svg>
  );
}
