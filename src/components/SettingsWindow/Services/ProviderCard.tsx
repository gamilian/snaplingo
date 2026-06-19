import { Provider } from '../../../stores/providerStore';

interface ProviderCardProps {
  provider: Provider;
  onActivate?: () => void;
  onDeactivate?: () => void;
  onConfigure?: () => void;
  onTest?: () => void;
  onRemove?: () => void;
}

export function ProviderCard({ provider, onActivate, onDeactivate, onConfigure, onTest, onRemove }: ProviderCardProps) {
  const getStatusBadge = () => {
    switch (provider.status) {
      case 'active':
        return <span className="px-2 py-0.5 text-xs bg-green-100 text-green-700 rounded font-medium">已激活</span>;
      case 'inactive':
        return <span className="px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded font-medium">未激活</span>;
      case 'unconfigured':
        return <span className="px-2 py-0.5 text-xs bg-yellow-100 text-yellow-700 rounded font-medium">未配置</span>;
    }
  };

  const getIcon = () => {
    // 根据 provider id 返回首字母或图标
    const firstChar = provider.name.charAt(0).toUpperCase();
    return (
      <div className="w-12 h-12 bg-blue-100 rounded-lg flex items-center justify-center flex-shrink-0">
        <span className="text-primary-600 font-semibold text-lg">{firstChar}</span>
      </div>
    );
  };

  return (
    <div className="bg-white border border-gray-200 rounded-lg p-5 hover:shadow-md transition-shadow">
      <div className="flex items-start space-x-4">
        {getIcon()}

        <div className="flex-1 min-w-0">
          <div className="flex items-center space-x-2 mb-1">
            <h3 className="font-semibold text-gray-800">{provider.name}</h3>
            {getStatusBadge()}
            {provider.isBuiltin && (
              <span className="px-2 py-0.5 text-xs bg-blue-50 text-primary-600 rounded font-medium">内置</span>
            )}
          </div>
          <p className="text-sm text-gray-600 mb-3">{provider.description}</p>

          <div className="flex items-center space-x-2">
            {provider.status === 'unconfigured' && onConfigure && (
              <button
                onClick={onConfigure}
                className="px-3 py-1.5 text-sm bg-primary-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
              >
                配置
              </button>
            )}

            {provider.status === 'inactive' && onActivate && (
              <button
                onClick={onActivate}
                className="px-3 py-1.5 text-sm bg-green-600 hover:bg-green-700 text-white rounded-lg transition-colors"
              >
                激活
              </button>
            )}

            {provider.status === 'active' && onDeactivate && (
              <button
                onClick={onDeactivate}
                className="px-3 py-1.5 text-sm bg-gray-600 hover:bg-gray-700 text-white rounded-lg transition-colors"
              >
                停用
              </button>
            )}

            {provider.status !== 'unconfigured' && onConfigure && (
              <button
                onClick={onConfigure}
                className="px-3 py-1.5 text-sm text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded-lg transition-colors"
              >
                设置
              </button>
            )}

            {onTest && provider.status !== 'unconfigured' && (
              <button
                onClick={onTest}
                className="px-3 py-1.5 text-sm text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded-lg transition-colors"
              >
                测试
              </button>
            )}

            {!provider.isBuiltin && onRemove && (
              <button
                onClick={onRemove}
                className="px-3 py-1.5 text-sm text-red-600 hover:text-red-800 hover:bg-red-50 rounded-lg transition-colors"
              >
                删除
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
