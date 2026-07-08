import type {
  OpenAICompatibleModelInfo,
  TranslationPromptStrategy,
} from '../../../tauri/providers';
import IconActionButton from '../../common/IconActionButton';
import type {
  LLMProtocol,
  LLMProtocolFamily,
} from './customTranslationProviderFormModel';

interface Option<T extends string> {
  value: T;
  label: string;
}

interface TestStatus {
  type: 'success' | 'error';
  message: string;
}

export interface CustomTranslationProviderDialogViewProps {
  apiKey: string;
  canSave: boolean;
  dialogTitle: string;
  endpoint: string;
  endpointPreview: string;
  isEditing: boolean;
  isLoadingModels: boolean;
  isSaving: boolean;
  isTestingProvider: boolean;
  model: string;
  modelListError: string | null;
  models: OpenAICompatibleModelInfo[];
  name: string;
  openAIModeOptions: Array<Option<LLMProtocol>>;
  presentation: 'dialog' | 'inline';
  promptStrategies: TranslationPromptStrategy[];
  promptStrategyId: string;
  protocol: LLMProtocol;
  protocolFamily: LLMProtocolFamily;
  protocolOptions: Array<Option<LLMProtocolFamily>>;
  reasoningLevel: string;
  reasoningOptions: Array<Option<string>>;
  saveError: string | null;
  selectedPromptStrategy: TranslationPromptStrategy | undefined;
  smartPromptStrategyId: string;
  strategyDraftDescription: string;
  strategyDraftName: string;
  strategyDraftPrompt: string;
  strategyError: string | null;
  testStatus: TestStatus | null;
  onAddStrategy(): void | Promise<void>;
  onApiKeyChange(value: string): void;
  onCancel(): void;
  onDeleteStrategy(): void | Promise<void>;
  onEndpointChange(value: string): void;
  onLoadModels(): void | Promise<void>;
  onModelChange(value: string): void;
  onNameChange(value: string): void;
  onOpenAIModeChange(protocol: LLMProtocol): void;
  onPromptStrategyChange(strategyId: string): void;
  onProtocolFamilyChange(protocol: LLMProtocolFamily): void;
  onReasoningLevelChange(value: string): void;
  onSave(): void | Promise<void>;
  onSaveStrategy(): void | Promise<void>;
  onStrategyDraftDescriptionChange(value: string): void;
  onStrategyDraftNameChange(value: string): void;
  onStrategyDraftPromptChange(value: string): void;
  onTestProvider(): void | Promise<void>;
}

export function CustomTranslationProviderDialogView({
  apiKey,
  canSave,
  dialogTitle,
  endpoint,
  endpointPreview,
  isEditing,
  isLoadingModels,
  isSaving,
  isTestingProvider,
  model,
  modelListError,
  models,
  name,
  onAddStrategy,
  onApiKeyChange,
  onCancel,
  onDeleteStrategy,
  onEndpointChange,
  onLoadModels,
  onModelChange,
  onNameChange,
  onOpenAIModeChange,
  onPromptStrategyChange,
  onProtocolFamilyChange,
  onReasoningLevelChange,
  onSave,
  onSaveStrategy,
  onStrategyDraftDescriptionChange,
  onStrategyDraftNameChange,
  onStrategyDraftPromptChange,
  onTestProvider,
  openAIModeOptions,
  presentation,
  promptStrategies,
  promptStrategyId,
  protocol,
  protocolFamily,
  protocolOptions,
  reasoningLevel,
  reasoningOptions,
  saveError,
  selectedPromptStrategy,
  smartPromptStrategyId,
  strategyDraftDescription,
  strategyDraftName,
  strategyDraftPrompt,
  strategyError,
  testStatus,
}: CustomTranslationProviderDialogViewProps) {
  return (
    <div
      className={
        presentation === 'inline'
          ? 'w-full space-y-6'
          : 'bg-white rounded-2xl shadow-xl p-6 w-full max-w-2xl max-h-[90vh] overflow-y-auto'
      }
    >
      <div className="flex items-center gap-3">
        {presentation === 'inline' && (
          <IconActionButton
            onClick={onCancel}
            disabled={isSaving}
            title="返回供应商列表"
            tooltipPlacement="bottom"
            className="flex h-9 w-9 items-center justify-center rounded-full border border-gray-200 bg-white text-gray-600 transition-colors hover:bg-gray-50 disabled:opacity-50"
          >
            <svg
              className="h-4 w-4"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
            </svg>
          </IconActionButton>
        )}
        <h3 className="text-xl font-semibold text-gray-900">{dialogTitle}</h3>
      </div>

      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">名称</label>
          <input
            type="text"
            value={name}
            onChange={(e) => onNameChange(e.target.value)}
            placeholder="例如：我的 GPT-4"
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">协议</label>
          <div className="grid grid-cols-3 gap-2 rounded-lg bg-gray-100 p-1">
            {protocolOptions.map((option) => (
              <button
                key={option.value}
                type="button"
                onClick={() => onProtocolFamilyChange(option.value)}
                title={option.label}
                className={`rounded-md px-3 py-2 text-sm font-medium transition-colors ${
                  protocolFamily === option.value
                    ? 'bg-white text-gray-900 shadow-sm'
                    : 'text-gray-600 hover:text-gray-900'
                }`}
              >
                {option.label}
              </button>
            ))}
          </div>
        </div>

        {protocolFamily === 'openai' && (
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              OpenAI 模式
            </label>
            <div className="grid grid-cols-2 gap-2 rounded-lg bg-gray-100 p-1">
              {openAIModeOptions.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  onClick={() => onOpenAIModeChange(option.value)}
                  className={`rounded-md px-3 py-2 text-sm font-medium transition-colors ${
                    protocol === option.value
                      ? 'bg-white text-gray-900 shadow-sm'
                      : 'text-gray-600 hover:text-gray-900'
                  }`}
                >
                  {option.label}
                </button>
              ))}
            </div>
          </div>
        )}

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">API 地址</label>
          <input
            type="text"
            value={endpoint}
            onChange={(e) => onEndpointChange(e.target.value)}
            placeholder="https://api.openai.com"
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
          />
          {endpointPreview && (
            <p className="text-xs text-gray-500 mt-1">预览：{endpointPreview}</p>
          )}
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">API Key</label>
          <input
            type="password"
            value={apiKey}
            onChange={(e) => onApiKeyChange(e.target.value)}
            placeholder={isEditing ? '留空则保持现有 API Key' : 'sk-...'}
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
          />
        </div>

        <div>
          <div className="flex items-center justify-between gap-3 mb-1">
            <label className="block text-sm font-medium text-gray-700">模型</label>
            <button
              type="button"
              onClick={onLoadModels}
              disabled={isLoadingModels || !endpoint.trim() || !apiKey.trim()}
              className="px-3 py-1.5 text-sm text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 disabled:text-gray-400 disabled:bg-gray-100 disabled:cursor-not-allowed transition-colors"
            >
              {isLoadingModels ? '获取中...' : '获取模型列表'}
            </button>
          </div>
          <input
            type="text"
            value={model}
            onChange={(e) => onModelChange(e.target.value)}
            placeholder="gpt-4o / claude-3-5-sonnet-latest / gemini-1.5-flash"
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
          />
          {modelListError && (
            <p className="text-xs text-red-600 mt-1">{modelListError}</p>
          )}
          {models.length > 0 && (
            <div className="mt-2 border border-gray-200 rounded-lg max-h-48 overflow-y-auto divide-y divide-gray-100">
              {models.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => onModelChange(item.id)}
                  className={`w-full px-3 py-2 text-left text-sm hover:bg-blue-50 transition-colors ${
                    item.id === model ? 'bg-blue-50 text-primary-700 font-medium' : 'text-gray-700'
                  }`}
                >
                  {item.id}
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={onTestProvider}
            disabled={
              isTestingProvider || !endpoint.trim() || !apiKey.trim() || !model.trim()
            }
            className="px-3 py-1.5 text-sm text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 disabled:text-gray-400 disabled:bg-gray-100 disabled:cursor-not-allowed transition-colors"
          >
            {isTestingProvider ? '检测中...' : '检测连接'}
          </button>
          {testStatus && (
            <span
              className={`text-sm ${
                testStatus.type === 'success' ? 'text-green-700' : 'text-red-600'
              }`}
            >
              {testStatus.message}
            </span>
          )}
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Reasoning 强度 <span className="text-gray-400 text-xs">(可选)</span>
          </label>
          <div className="flex flex-wrap gap-2">
            {reasoningOptions.map((option) => (
              <button
                key={option.value || 'default'}
                type="button"
                onClick={() => onReasoningLevelChange(option.value)}
                className={`rounded-lg border px-3 py-1.5 text-sm font-medium transition-colors ${
                  reasoningLevel === option.value
                    ? 'border-primary-500 bg-blue-50 text-primary-700'
                    : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300 hover:bg-gray-50'
                }`}
              >
                {option.label}
              </button>
            ))}
          </div>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">翻译策略</label>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => onPromptStrategyChange(smartPromptStrategyId)}
              className={`rounded-lg border px-3 py-1.5 text-sm font-medium transition-colors ${
                promptStrategyId === smartPromptStrategyId
                  ? 'border-primary-500 bg-blue-50 text-primary-700'
                  : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300 hover:bg-gray-50'
              }`}
            >
              智能选择
            </button>
            {promptStrategies.map((strategy) => (
              <button
                key={strategy.id}
                type="button"
                onClick={() => onPromptStrategyChange(strategy.id)}
                className={`rounded-lg border px-3 py-1.5 text-sm font-medium transition-colors ${
                  promptStrategyId === strategy.id
                    ? 'border-primary-500 bg-blue-50 text-primary-700'
                    : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300 hover:bg-gray-50'
                }`}
              >
                {strategy.name}
              </button>
            ))}
          </div>
        </div>

        <div className="rounded-lg border border-gray-200 bg-gray-50 p-3 space-y-2">
          <div className="flex items-center justify-between gap-3">
            <label className="block text-sm font-medium text-gray-700">策略编辑</label>
            {selectedPromptStrategy?.is_deletable && (
              <button
                type="button"
                onClick={onDeleteStrategy}
                className="text-xs font-medium text-red-600 hover:text-red-700"
              >
                删除策略
              </button>
            )}
          </div>
          <input
            type="text"
            value={strategyDraftName}
            onChange={(event) => onStrategyDraftNameChange(event.target.value)}
            placeholder="策略名称，例如：法律翻译"
            className="w-full px-3 py-2 border border-gray-300 rounded-lg bg-white focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
          />
          <input
            type="text"
            value={strategyDraftDescription}
            onChange={(event) => onStrategyDraftDescriptionChange(event.target.value)}
            placeholder="适用说明，例如：合同、条款、法律文本"
            className="w-full px-3 py-2 border border-gray-300 rounded-lg bg-white focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
          />
          <textarea
            value={strategyDraftPrompt}
            onChange={(event) => onStrategyDraftPromptChange(event.target.value)}
            placeholder="系统提示词，可使用 {source_lang} 和 {target_lang}"
            rows={4}
            className="w-full px-3 py-2 border border-gray-300 rounded-lg bg-white focus:ring-2 focus:ring-primary-500 focus:border-blue-500 resize-none"
          />
          {strategyError && <p className="text-xs text-red-600">{strategyError}</p>}
          <div className="flex gap-2">
            <button
              type="button"
              onClick={onSaveStrategy}
              disabled={!selectedPromptStrategy}
              className="px-3 py-1.5 text-sm text-gray-700 border border-gray-300 rounded-lg bg-white hover:bg-gray-50 disabled:text-gray-400 disabled:bg-gray-100 disabled:cursor-not-allowed transition-colors"
            >
              保存策略
            </button>
            <button
              type="button"
              onClick={onAddStrategy}
              className="px-3 py-1.5 text-sm text-gray-700 border border-gray-300 rounded-lg bg-white hover:bg-gray-50 transition-colors"
            >
              新增为自定义策略
            </button>
          </div>
        </div>
      </div>

      {saveError && <p className="mt-4 text-sm text-red-600">{saveError}</p>}

      <div className="flex gap-3 mt-6">
        <button
          onClick={onCancel}
          disabled={isSaving}
          className="flex-1 px-4 py-2 text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors"
        >
          取消
        </button>
        <button
          onClick={onSave}
          disabled={!canSave}
          className="flex-1 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
        >
          {isSaving ? '保存中...' : isEditing ? '保存配置' : '添加'}
        </button>
      </div>
    </div>
  );
}
