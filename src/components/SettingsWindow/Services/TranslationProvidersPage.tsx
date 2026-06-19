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
  const removeProvider = useProviderStore((state) => state.removeTranslationProvider);
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

  const handleTest = (_id: string) => {
    // TODO: 实现 Provider 测试
    alert('测试功能：将使用该翻译服务翻译一段示例文本\n\n此功能待实现');
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
    try {
      await addCustomProvider(request);
      setAddingCustomProvider(false);
    } catch (error) {
      alert(`添加失败: ${error}`);
    }
  };

  // 拖拽处理
  const handleDragStart = (id: string) => {
    setDraggedId(id);
  };

  const handleDragOver = (e: React.DragEvent, id: string) => {
    e.preventDefault();
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

  // 分离 active 和 inactive providers
  const activeProvidersList = activeProviders
    .map((id) => providers.find((p) => p.id === id))
    .filter((p): p is NonNullable<typeof p> => p !== undefined);

  const inactiveProvidersList = providers.filter(
    (p) => !activeProviders.includes(p.id)
  );

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
        <p className="text-sm text-gray-500 mt-1">
          支持同时激活多个翻译服务，结果会并行显示。拖拽已激活的服务可调整顺序。
        </p>
      </div>

      {/* 已激活的服务（可拖拽排序） */}
      {activeProvidersList.length > 0 && (
        <div>
          <h3 className="text-lg font-semibold text-gray-800 mb-3">已激活 ({activeProvidersList.length})</h3>
          <div className="space-y-4">
            {activeProvidersList.map((provider) => (
              <div
                key={provider.id}
                draggable
                onDragStart={() => handleDragStart(provider.id)}
                onDragOver={(e) => handleDragOver(e, provider.id)}
                onDragLeave={handleDragLeave}
                onDrop={(e) => handleDrop(e, provider.id)}
                onDragEnd={handleDragEnd}
                className={`transition-all ${
                  draggedId === provider.id ? 'opacity-50' : ''
                } ${
                  dragOverId === provider.id ? 'border-2 border-blue-500 rounded-lg' : ''
                }`}
              >
                <ProviderCard
                  provider={provider}
                  onActivate={() => handleActivate(provider.id)}
                  onDeactivate={() => handleDeactivate(provider.id)}
                  onConfigure={() => handleConfigure(provider.id)}
                  onTest={() => handleTest(provider.id)}
                  onRemove={() => handleRemove(provider.id)}
                />
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 未激活的服务 */}
      {inactiveProvidersList.length > 0 && (
        <div>
          <h3 className="text-lg font-semibold text-gray-800 mb-3">可用服务 ({inactiveProvidersList.length})</h3>
          <div className="space-y-4">
            {inactiveProvidersList.map((provider) => (
              <ProviderCard
                key={provider.id}
                provider={provider}
                onActivate={() => handleActivate(provider.id)}
                onDeactivate={() => handleDeactivate(provider.id)}
                onConfigure={() => handleConfigure(provider.id)}
                onTest={() => handleTest(provider.id)}
                onRemove={() => handleRemove(provider.id)}
              />
            ))}
          </div>
        </div>
      )}

      <div className="pt-4 border-t border-gray-200">
        <button
          onClick={handleAddCustom}
          className="px-4 py-2 text-sm text-primary-600 hover:bg-blue-50 rounded-lg transition-colors font-medium"
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

      <CustomTranslationProviderDialog
        isOpen={addingCustomProvider}
        onClose={() => setAddingCustomProvider(false)}
        onSave={handleSaveCustomProvider}
      />
    </div>
  );
}
