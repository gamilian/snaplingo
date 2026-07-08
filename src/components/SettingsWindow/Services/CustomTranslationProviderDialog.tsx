import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import {
  listAnthropicModels,
  listGeminiModels,
  listOpenAICompatibleModels,
  listTranslationPromptStrategies,
  saveTranslationPromptStrategies,
  testAnthropicProvider,
  testGeminiProvider,
  testOpenAICompatibleProvider,
  testOpenAIResponsesProvider,
} from '../../../tauri/providers';
import type {
  AddCustomTranslationProviderRequest,
  OpenAICompatibleModelInfo,
  TranslationPromptStrategy,
  UpdateCustomTranslationProviderRequest,
} from '../../../tauri/providers';
import type { Provider } from '../../../stores/providerStore';
import IconActionButton from '../../common/IconActionButton';
import { getCustomProviderEndpointPreview } from './customTranslationProviderForm';
import {
  buildAddCustomProviderRequest,
  buildUpdateCustomProviderRequest,
  canSaveCustomProviderForm,
  DEFAULT_PROMPT_STRATEGY_ID,
  formatCustomProviderError,
  getInitialCustomProviderFormValues,
  getProtocolDefaults,
  getProtocolFamily,
  OPENAI_MODE_OPTIONS,
  PROTOCOL_OPTIONS,
  REASONING_OPTIONS,
  SMART_PROMPT_STRATEGY_ID,
  type LLMProtocol,
  type LLMProtocolFamily,
} from './customTranslationProviderFormModel';

const DEFAULT_PROMPT_STRATEGIES: TranslationPromptStrategy[] = [
  {
    id: DEFAULT_PROMPT_STRATEGY_ID,
    name: '通用翻译',
    description: '适合大多数普通文本。',
    system_prompt:
      'You are a professional translation engine. Translate the user text from {source_lang} to {target_lang}. Return only the translation.',
    is_builtin: true,
    is_deletable: false,
  },
];

interface CustomTranslationProviderDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (request: AddCustomTranslationProviderRequest) => Promise<void> | void;
  onUpdate?: (
    providerId: string,
    request: UpdateCustomTranslationProviderRequest,
  ) => Promise<void> | void;
  initialProvider?: Provider | null;
  presentation?: 'dialog' | 'inline';
}

export function CustomTranslationProviderDialog({
  isOpen,
  onClose,
  onSave,
  onUpdate,
  initialProvider = null,
  presentation = 'dialog',
}: CustomTranslationProviderDialogProps) {
  const initialValues = getInitialCustomProviderFormValues(initialProvider);
  const isEditing = Boolean(initialProvider);
  const [name, setName] = useState(initialValues.name);
  const [protocol, setProtocol] = useState<LLMProtocol>(initialValues.protocol);
  const [endpoint, setEndpoint] = useState(initialValues.endpoint);
  const [model, setModel] = useState(initialValues.model);
  const [apiKey, setApiKey] = useState('');
  const [reasoningLevel, setReasoningLevel] = useState<string>(
    initialValues.reasoningLevel,
  );
  const [promptStrategyId, setPromptStrategyId] = useState(initialValues.promptStrategyId);
  const [promptStrategies, setPromptStrategies] = useState<TranslationPromptStrategy[]>(
    DEFAULT_PROMPT_STRATEGIES,
  );
  const [strategyDraftName, setStrategyDraftName] = useState('');
  const [strategyDraftDescription, setStrategyDraftDescription] = useState('');
  const [strategyDraftPrompt, setStrategyDraftPrompt] = useState('');
  const [strategyError, setStrategyError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [models, setModels] = useState<OpenAICompatibleModelInfo[]>([]);
  const [isLoadingModels, setIsLoadingModels] = useState(false);
  const [modelListError, setModelListError] = useState<string | null>(null);
  const [isTestingProvider, setIsTestingProvider] = useState(false);
  const [testStatus, setTestStatus] = useState<{
    type: 'success' | 'error';
    message: string;
  } | null>(null);

  const endpointPreview = getCustomProviderEndpointPreview(protocol, endpoint, model);
  const selectedPromptStrategy = promptStrategies.find(
    (strategy) => strategy.id === promptStrategyId,
  );

  const resetProviderIntrospectionState = () => {
    setModels([]);
    setModelListError(null);
    setTestStatus(null);
  };

  useEffect(() => {
    if (!isOpen) return;

    const values = getInitialCustomProviderFormValues(initialProvider);
    setName(values.name);
    setProtocol(values.protocol);
    setEndpoint(values.endpoint);
    setModel(values.model);
    setApiKey('');
    setReasoningLevel(values.reasoningLevel);
    setPromptStrategyId(values.promptStrategyId);
    setSaveError(null);
    setStrategyError(null);
    resetProviderIntrospectionState();
    void loadPromptStrategies(values.promptStrategyId);
  }, [isOpen, initialProvider?.id]);

  const resetAndClose = () => {
    setName('');
    setProtocol('openai');
    setEndpoint('https://api.openai.com');
    setModel('gpt-4o');
    setApiKey('');
    setReasoningLevel('');
    setPromptStrategyId(SMART_PROMPT_STRATEGY_ID);
    clearStrategyDraft();
    setSaveError(null);
    setStrategyError(null);
    resetProviderIntrospectionState();
    onClose();
  };

  const handleSave = async () => {
    if (isSaving) return;

    const input = {
      name,
      protocol,
      endpoint,
      model,
      apiKey,
      reasoningLevel,
      promptStrategyId,
    };
    const request = isEditing
      ? buildUpdateCustomProviderRequest(input)
      : buildAddCustomProviderRequest(input);
    if (!request) return;

    setIsSaving(true);
    setSaveError(null);

    try {
      if (isEditing && initialProvider) {
        await onUpdate?.(
          initialProvider.id,
          request as UpdateCustomTranslationProviderRequest,
        );
      } else {
        await onSave(request as AddCustomTranslationProviderRequest);
      }
      setIsSaving(false);
      resetAndClose();
    } catch (error) {
      setSaveError(
        `${isEditing ? '保存' : '添加'}失败: ${formatCustomProviderError(error)}`,
      );
      setIsSaving(false);
    }
  };

  const handleProtocolFamilyChange = (newProtocol: LLMProtocolFamily) => {
    if (newProtocol === getProtocolFamily(protocol)) return;

    setProtocol(newProtocol);
    resetProviderIntrospectionState();
    const defaults = getProtocolDefaults(newProtocol);
    setEndpoint(defaults.endpoint);
    setModel(defaults.model);
  };

  const handleOpenAIModeChange = (newProtocol: LLMProtocol) => {
    if (newProtocol === protocol) return;

    setProtocol(newProtocol);
    resetProviderIntrospectionState();
    const defaults = getProtocolDefaults(newProtocol);
    setEndpoint(defaults.endpoint);
    setModel(defaults.model);
  };

  const handleLoadModels = async () => {
    if (isLoadingModels) return;

    const trimmedEndpoint = endpoint.trim();
    const trimmedApiKey = apiKey.trim();
    if (!trimmedEndpoint || !trimmedApiKey) return;

    setIsLoadingModels(true);
    setModelListError(null);
    setTestStatus(null);

    try {
      const request = {
        endpoint: trimmedEndpoint,
        api_key: trimmedApiKey,
      };
      const loadedModels =
        protocol === 'openai' || protocol === 'openai-responses'
          ? await listOpenAICompatibleModels(request)
          : protocol === 'anthropic'
            ? await listAnthropicModels(request)
            : await listGeminiModels(request);
      setModels(loadedModels);
      if (loadedModels.length === 0) {
        setModelListError('未返回可用模型');
      }
    } catch (error) {
      setModels([]);
      setModelListError(`获取模型失败: ${formatCustomProviderError(error)}`);
    } finally {
      setIsLoadingModels(false);
    }
  };

  const handleTestProvider = async () => {
    if (isTestingProvider) return;

    const trimmedEndpoint = endpoint.trim();
    const trimmedApiKey = apiKey.trim();
    const trimmedModel = model.trim();
    if (!trimmedEndpoint || !trimmedApiKey || !trimmedModel) return;

    setIsTestingProvider(true);
    setTestStatus(null);

    try {
      const request = {
        endpoint: trimmedEndpoint,
        api_key: trimmedApiKey,
        model: trimmedModel,
      };
      if (protocol === 'openai') {
        await testOpenAICompatibleProvider(request);
      } else if (protocol === 'openai-responses') {
        await testOpenAIResponsesProvider(request);
      } else if (protocol === 'anthropic') {
        await testAnthropicProvider(request);
      } else {
        await testGeminiProvider(request);
      }
      setTestStatus({ type: 'success', message: '检测成功' });
    } catch (error) {
      setTestStatus({
        type: 'error',
        message: `检测失败: ${formatCustomProviderError(error)}`,
      });
    } finally {
      setIsTestingProvider(false);
    }
  };

  const handleEndpointChange = (value: string) => {
    setEndpoint(value);
    resetProviderIntrospectionState();
  };

  const handleApiKeyChange = (value: string) => {
    setApiKey(value);
    resetProviderIntrospectionState();
  };

  const handleModelChange = (value: string) => {
    setModel(value);
    setTestStatus(null);
  };

  async function loadPromptStrategies(strategyId: string) {
    try {
      const config = await listTranslationPromptStrategies();
      setPromptStrategies(config.strategies);
      populateStrategyDraft(strategyId, config.strategies);
    } catch (error) {
      console.error('Failed to load prompt strategies:', error);
      populateStrategyDraft(strategyId, DEFAULT_PROMPT_STRATEGIES);
    }
  }

  const handlePromptStrategyChange = (strategyId: string) => {
    setPromptStrategyId(strategyId);
    setStrategyError(null);
    populateStrategyDraft(strategyId, promptStrategies);
  };

  const handleSaveStrategy = async () => {
    if (!selectedPromptStrategy) return;

    const trimmedName = strategyDraftName.trim();
    const trimmedPrompt = strategyDraftPrompt.trim();
    if (!trimmedName || !trimmedPrompt) {
      setStrategyError('策略名称和系统提示词不能为空');
      return;
    }

    const nextStrategies = promptStrategies.map((strategy) =>
      strategy.id === selectedPromptStrategy.id
        ? {
            ...strategy,
            name: trimmedName,
            description: strategyDraftDescription.trim(),
            system_prompt: trimmedPrompt,
          }
        : strategy,
    );

    await persistPromptStrategies(nextStrategies, selectedPromptStrategy.id);
  };

  const handleAddStrategy = async () => {
    const trimmedName = strategyDraftName.trim();
    const trimmedPrompt = strategyDraftPrompt.trim();
    if (!trimmedName || !trimmedPrompt) {
      setStrategyError('策略名称和系统提示词不能为空');
      return;
    }

    const strategy: TranslationPromptStrategy = {
      id: `custom-${Date.now()}`,
      name: trimmedName,
      description: strategyDraftDescription.trim(),
      system_prompt: trimmedPrompt,
      is_builtin: false,
      is_deletable: true,
    };

    await persistPromptStrategies([...promptStrategies, strategy], strategy.id);
  };

  const handleDeleteStrategy = async () => {
    if (!selectedPromptStrategy?.is_deletable) return;

    await persistPromptStrategies(
      promptStrategies.filter((strategy) => strategy.id !== selectedPromptStrategy.id),
      DEFAULT_PROMPT_STRATEGY_ID,
    );
  };

  async function persistPromptStrategies(
    strategies: TranslationPromptStrategy[],
    nextSelectedId: string,
  ) {
    setStrategyError(null);
    try {
      const saved = await saveTranslationPromptStrategies({ strategies });
      setPromptStrategies(saved.strategies);
      setPromptStrategyId(nextSelectedId);
      populateStrategyDraft(nextSelectedId, saved.strategies);
    } catch (error) {
      setStrategyError(`保存策略失败: ${formatCustomProviderError(error)}`);
    }
  }

  function populateStrategyDraft(
    strategyId: string,
    strategies: TranslationPromptStrategy[],
  ) {
    if (strategyId === SMART_PROMPT_STRATEGY_ID) {
      clearStrategyDraft();
      return;
    }

    const strategy = strategies.find((item) => item.id === strategyId);
    if (!strategy) {
      clearStrategyDraft();
      return;
    }

    setStrategyDraftName(strategy.name);
    setStrategyDraftDescription(strategy.description);
    setStrategyDraftPrompt(strategy.system_prompt);
  }

  function clearStrategyDraft() {
    setStrategyDraftName('');
    setStrategyDraftDescription('');
    setStrategyDraftPrompt('');
  }

  if (!isOpen) return null;

  const canSave = canSaveCustomProviderForm({
    name,
    protocol,
    endpoint,
    model,
    apiKey,
    reasoningLevel,
    promptStrategyId,
    isSaving,
    isEditing,
    canUpdate: Boolean(onUpdate),
  });
  const dialogTitle = isEditing
    ? `配置 ${initialProvider?.name || '自定义翻译服务'}`
    : '添加自定义翻译服务';
  const protocolFamily = getProtocolFamily(protocol);

  const form = (
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
              onClick={resetAndClose}
              disabled={isSaving}
              title="返回供应商列表"
              tooltipPlacement="bottom"
              className="flex h-9 w-9 items-center justify-center rounded-full border border-gray-200 bg-white text-gray-600 transition-colors hover:bg-gray-50 disabled:opacity-50"
            >
              <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
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
              onChange={(e) => setName(e.target.value)}
              placeholder="例如：我的 GPT-4"
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">协议</label>
            <div className="grid grid-cols-3 gap-2 rounded-lg bg-gray-100 p-1">
              {PROTOCOL_OPTIONS.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  onClick={() => handleProtocolFamilyChange(option.value)}
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
                {OPENAI_MODE_OPTIONS.map((option) => (
                  <button
                    key={option.value}
                    type="button"
                    onClick={() => handleOpenAIModeChange(option.value)}
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
              onChange={(e) => handleEndpointChange(e.target.value)}
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
              onChange={(e) => handleApiKeyChange(e.target.value)}
              placeholder={isEditing ? '留空则保持现有 API Key' : 'sk-...'}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
            />
          </div>

          <div>
            <div className="flex items-center justify-between gap-3 mb-1">
              <label className="block text-sm font-medium text-gray-700">模型</label>
              <button
                type="button"
                onClick={handleLoadModels}
                disabled={isLoadingModels || !endpoint.trim() || !apiKey.trim()}
                className="px-3 py-1.5 text-sm text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 disabled:text-gray-400 disabled:bg-gray-100 disabled:cursor-not-allowed transition-colors"
              >
                {isLoadingModels ? '获取中...' : '获取模型列表'}
              </button>
            </div>
            <input
              type="text"
              value={model}
              onChange={(e) => handleModelChange(e.target.value)}
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
                    onClick={() => handleModelChange(item.id)}
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
              onClick={handleTestProvider}
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
              {REASONING_OPTIONS.map((option) => (
                <button
                  key={option.value || 'default'}
                  type="button"
                  onClick={() => setReasoningLevel(option.value)}
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
                onClick={() => handlePromptStrategyChange(SMART_PROMPT_STRATEGY_ID)}
                className={`rounded-lg border px-3 py-1.5 text-sm font-medium transition-colors ${
                  promptStrategyId === SMART_PROMPT_STRATEGY_ID
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
                  onClick={() => handlePromptStrategyChange(strategy.id)}
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
                  onClick={handleDeleteStrategy}
                  className="text-xs font-medium text-red-600 hover:text-red-700"
                >
                  删除策略
                </button>
              )}
            </div>
            <input
              type="text"
              value={strategyDraftName}
              onChange={(event) => setStrategyDraftName(event.target.value)}
              placeholder="策略名称，例如：法律翻译"
              className="w-full px-3 py-2 border border-gray-300 rounded-lg bg-white focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
            />
            <input
              type="text"
              value={strategyDraftDescription}
              onChange={(event) => setStrategyDraftDescription(event.target.value)}
              placeholder="适用说明，例如：合同、条款、法律文本"
              className="w-full px-3 py-2 border border-gray-300 rounded-lg bg-white focus:ring-2 focus:ring-primary-500 focus:border-blue-500"
            />
            <textarea
              value={strategyDraftPrompt}
              onChange={(event) => setStrategyDraftPrompt(event.target.value)}
              placeholder="系统提示词，可使用 {source_lang} 和 {target_lang}"
              rows={4}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg bg-white focus:ring-2 focus:ring-primary-500 focus:border-blue-500 resize-none"
            />
            {strategyError && <p className="text-xs text-red-600">{strategyError}</p>}
            <div className="flex gap-2">
              <button
                type="button"
                onClick={handleSaveStrategy}
                disabled={!selectedPromptStrategy}
                className="px-3 py-1.5 text-sm text-gray-700 border border-gray-300 rounded-lg bg-white hover:bg-gray-50 disabled:text-gray-400 disabled:bg-gray-100 disabled:cursor-not-allowed transition-colors"
              >
                保存策略
              </button>
              <button
                type="button"
                onClick={handleAddStrategy}
                className="px-3 py-1.5 text-sm text-gray-700 border border-gray-300 rounded-lg bg-white hover:bg-gray-50 transition-colors"
              >
                新增为自定义策略
              </button>
            </div>
          </div>
        </div>

        {saveError && (
          <p className="mt-4 text-sm text-red-600">{saveError}</p>
        )}

        <div className="flex gap-3 mt-6">
          <button
            onClick={resetAndClose}
            disabled={isSaving}
            className="flex-1 px-4 py-2 text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors"
          >
            取消
          </button>
          <button
            onClick={handleSave}
            disabled={!canSave}
            className="flex-1 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
          >
            {isSaving ? '保存中...' : isEditing ? '保存配置' : '添加'}
          </button>
        </div>
    </div>
  );

  if (presentation === 'inline') {
    return form;
  }

  return createPortal(
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      {form}
    </div>,
    document.body,
  );
}
