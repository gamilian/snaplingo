import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import type {
  AddCustomTranslationProviderRequest,
  OpenAICompatibleModelInfo,
  UpdateCustomTranslationProviderRequest,
} from '../../../tauri/providers';
import type { Provider } from '../../../stores/providerStore';
import { getCustomProviderEndpointPreview } from './customTranslationProviderForm';
import {
  buildAddCustomProviderRequest,
  buildUpdateCustomProviderRequest,
  canSaveCustomProviderForm,
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
import {
  loadCustomProviderModels,
  testCustomProviderConnection,
} from './customProviderIntrospection';
import { CustomTranslationProviderDialogView } from './CustomTranslationProviderDialogView';
import { useTranslationPromptStrategyWorkspace } from './useTranslationPromptStrategyWorkspace';

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
  const [promptStrategyId, setPromptStrategyId] = useState(
    initialValues.promptStrategyId,
  );
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
  const {
    clearStrategyDraft,
    handleAddStrategy,
    handleDeleteStrategy,
    handlePromptStrategyChange,
    handleSaveStrategy,
    loadPromptStrategies,
    promptStrategies,
    selectedPromptStrategy,
    setStrategyDraftDescription,
    setStrategyDraftName,
    setStrategyDraftPrompt,
    strategyDraftDescription,
    strategyDraftName,
    strategyDraftPrompt,
    strategyError,
  } = useTranslationPromptStrategyWorkspace({
    selectedStrategyId: promptStrategyId,
    onSelectedStrategyIdChange: setPromptStrategyId,
  });

  const endpointPreview = getCustomProviderEndpointPreview(protocol, endpoint, model);

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
      const result = await loadCustomProviderModels({
        protocol,
        endpoint: trimmedEndpoint,
        apiKey: trimmedApiKey,
      });
      setModels(result.models);
      setModelListError(result.error);
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
      const status = await testCustomProviderConnection({
        protocol,
        endpoint: trimmedEndpoint,
        apiKey: trimmedApiKey,
        model: trimmedModel,
      });
      setTestStatus(status);
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
    <CustomTranslationProviderDialogView
      apiKey={apiKey}
      canSave={canSave}
      dialogTitle={dialogTitle}
      endpoint={endpoint}
      endpointPreview={endpointPreview}
      isEditing={isEditing}
      isLoadingModels={isLoadingModels}
      isSaving={isSaving}
      isTestingProvider={isTestingProvider}
      model={model}
      modelListError={modelListError}
      models={models}
      name={name}
      onAddStrategy={handleAddStrategy}
      onApiKeyChange={handleApiKeyChange}
      onCancel={resetAndClose}
      onDeleteStrategy={handleDeleteStrategy}
      onEndpointChange={handleEndpointChange}
      onLoadModels={handleLoadModels}
      onModelChange={handleModelChange}
      onNameChange={setName}
      onOpenAIModeChange={handleOpenAIModeChange}
      onPromptStrategyChange={handlePromptStrategyChange}
      onProtocolFamilyChange={handleProtocolFamilyChange}
      onReasoningLevelChange={setReasoningLevel}
      onSave={handleSave}
      onSaveStrategy={handleSaveStrategy}
      onStrategyDraftDescriptionChange={setStrategyDraftDescription}
      onStrategyDraftNameChange={setStrategyDraftName}
      onStrategyDraftPromptChange={setStrategyDraftPrompt}
      onTestProvider={handleTestProvider}
      openAIModeOptions={OPENAI_MODE_OPTIONS}
      presentation={presentation}
      promptStrategies={promptStrategies}
      promptStrategyId={promptStrategyId}
      protocol={protocol}
      protocolFamily={protocolFamily}
      protocolOptions={PROTOCOL_OPTIONS}
      reasoningLevel={reasoningLevel}
      reasoningOptions={REASONING_OPTIONS}
      saveError={saveError}
      selectedPromptStrategy={selectedPromptStrategy}
      smartPromptStrategyId={SMART_PROMPT_STRATEGY_ID}
      strategyDraftDescription={strategyDraftDescription}
      strategyDraftName={strategyDraftName}
      strategyDraftPrompt={strategyDraftPrompt}
      strategyError={strategyError}
      testStatus={testStatus}
    />
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
