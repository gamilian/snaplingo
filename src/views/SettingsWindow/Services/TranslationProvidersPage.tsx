import { useState, useEffect } from 'react';
import { useProviderStore } from '../../../stores/providerStore';
import { ProviderCard } from './ProviderCard';
import { ProviderConfigDialog } from './ProviderConfigDialog';
import { CustomTranslationProviderDialog } from './CustomTranslationProviderDialog';

export function TranslationProvidersPage() {
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

  const [configuringProvider, setConfiguringProvider] = useState<string | null>(null);
  const [addingCustomProvider, setAddingCustomProvider] = useState(false);
  const [draggedId, setDraggedId] = useState<string | null>(null);
  const [dragOverId, setDragOverId] = useState<string | null>(null);

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

  // 拖拽处理
  const handleDragStart = (e: React.DragEvent, id: string) => {
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', id);
    setDraggedId(id);
  };

  const handleDragOver = (e: React.DragEvent, id: string) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    if (draggedId && draggedId !== id) {
      setDragOverId(id);
    }
  };

  const handleDragLeave = () => {
    setDragOverId(null);
  };

  const handleDrop = async (e: React.DragEvent, targetId: string) => {
    e.preventDefault();
    setDragOverId(null);

    if (!draggedId || draggedId === targetId) {
      setDraggedId(null);
      return;
    }

    // 只允许重排 active providers
    if (!activeProviders.includes(draggedId) || !activeProviders.includes(targetId)) {
      setDraggedId(null);
      return;
    }

    // 计算新顺序
    const newOrder = [...activeProviders];
    const draggedIndex = newOrder.indexOf(draggedId);
    const targetIndex = newOrder.indexOf(targetId);

    newOrder.splice(draggedIndex, 1);
    newOrder.splice(targetIndex, 0, draggedId);

    try {
      await reorderProviders(newOrder);
    } catch (error) {
      alert(`重新排序失败: ${error}`);
    }

    setDraggedId(null);
  };

  const handleDragEnd = () => {
    setDraggedId(null);
    setDragOverId(null);
  };

  const currentProvider = providers.find((p) => p.id === configuringProvider);
  const isConfiguringCustomProvider = Boolean(
    currentProvider && !currentProvider.isBuiltin,
  );

  if (addingCustomProvider || currentProvider) {
    return (
      <div className="max-w-5xl">
        {currentProvider && !isConfiguringCustomProvider ? (
          <ProviderConfigDialog
            isOpen
            presentation="inline"
            onClose={() => setConfiguringProvider(null)}
            onSave={handleSaveConfig}
            provider={currentProvider}
          />
        ) : (
          <CustomTranslationProviderDialog
            isOpen
            presentation="inline"
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

  // 分离 active 和 inactive providers
  const activeProvidersList = activeProviders
    .map((id) => providers.find((p) => p.id === id))
    .filter((p): p is NonNullable<typeof p> => p !== undefined);

  const inactiveProvidersList = providers.filter(
    (p) => !activeProviders.includes(p.id)
  );
  const orderedProviders = [...activeProvidersList, ...inactiveProvidersList];

  return (
    <div className="max-w-5xl">
      <div className="-mx-[22px] divide-y divide-gray-100 overflow-hidden rounded-[11px] border border-gray-200 bg-white shadow-sm">
        {orderedProviders.map((provider) => {
          const isActive = activeProviders.includes(provider.id);

          return (
            <div
              key={provider.id}
              draggable={isActive}
              onDragStart={isActive ? (e) => handleDragStart(e, provider.id) : undefined}
              onDragOver={isActive ? (e) => handleDragOver(e, provider.id) : undefined}
              onDragLeave={isActive ? handleDragLeave : undefined}
              onDrop={isActive ? (e) => handleDrop(e, provider.id) : undefined}
              onDragEnd={isActive ? handleDragEnd : undefined}
              className={`group relative transition-all ${
                isActive ? 'cursor-grab active:cursor-grabbing' : ''
              } ${
                draggedId === provider.id ? 'opacity-50' : ''
              } ${
                dragOverId === provider.id ? 'ring-2 ring-inset ring-primary-300' : ''
              }`}
            >
              <ProviderCard
                provider={provider}
                onActivate={() => handleActivate(provider.id)}
                onDeactivate={() => handleDeactivate(provider.id)}
                onConfigure={() => handleConfigure(provider.id)}
                onTest={!provider.isBuiltin ? () => handleTest(provider.id) : undefined}
                onRemove={() => handleRemove(provider.id)}
                highlighted={isActive}
                leadingSlot={isActive ? (
                  <span
                    aria-label="拖动排序"
                    title="拖动排序"
                    className="grid grid-cols-2 gap-0.5 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
                  >
                    {Array.from({ length: 6 }).map((_, index) => (
                      <span
                        key={index}
                        className="h-1 w-1 rounded-full bg-gray-400"
                      />
                    ))}
                  </span>
                ) : undefined}
              />
            </div>
          );
        })}
      </div>

      <button
        type="button"
        title="添加自定义服务"
        aria-label="添加自定义服务"
        onClick={handleAddCustom}
        className="mt-3 inline-flex h-9 items-center gap-2 rounded-lg border border-gray-200 bg-white px-3.5 text-xs font-semibold text-gray-600 transition-colors hover:border-primary-200 hover:text-primary-600 focus:outline-none focus:ring-2 focus:ring-primary-100"
      >
        <PlusIcon />
        <span>添加自定义服务</span>
      </button>
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
