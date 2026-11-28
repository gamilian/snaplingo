import type {
  ProviderModelInfo,
  TranslationPromptStrategy,
} from './providerViewTypes';
import { useState } from 'react';
import IconActionButton from '../../../components/common/IconActionButton';
import { ProviderBrandIcon } from '../../../components/common/ProviderBrandIcon';
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
  models: ProviderModelInfo[];
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
  const [strategyEditorOpen, setStrategyEditorOpen] = useState(false);
  const showStrategyEditor = strategyEditorOpen || Boolean(strategyError);
  const canTest = !isTestingProvider && Boolean(endpoint.trim() && apiKey.trim() && model.trim());
  const inputClassName = 'provider-dialog-input';

  return (
    <section
      role={presentation === 'dialog' ? 'dialog' : undefined}
      aria-modal={presentation === 'dialog' ? true : undefined}
      aria-label={dialogTitle}
      className={presentation === 'inline' ? 'provider-dialog provider-dialog-inline' : 'provider-dialog provider-dialog-wide'}
    >
      <header className="provider-dialog-header">
        <div className="flex min-w-0 items-center gap-3">
          {presentation === 'inline' ? (
            <IconActionButton
              onClick={onCancel}
              disabled={isSaving}
              title="返回供应商列表"
              tooltipPlacement="bottom"
              className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-line bg-surface text-muted transition-colors hover:bg-sunken disabled:opacity-50"
            >
              <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
              </svg>
            </IconActionButton>
          ) : (
            <ProviderBrandIcon id={protocolFamily} protocol={protocol} name={name || protocolFamily} />
          )}
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <h3 className="truncate text-[16px] font-bold tracking-[-0.02em] text-ink">{dialogTitle}</h3>
              <span className="rounded-full bg-accent-soft px-2 py-0.5 text-[11px] font-medium text-accent-text">翻译</span>
            </div>
            <p className="mt-0.5 text-xs text-muted">配置接口协议、模型与推理偏好</p>
          </div>
        </div>
        {presentation === 'dialog' && (
          <button
            type="button"
            onClick={onCancel}
            disabled={isSaving}
            aria-label="关闭"
            className="grid h-8 w-8 shrink-0 place-items-center rounded-lg text-faint transition-colors hover:bg-raised hover:text-ink disabled:opacity-40"
          >
            ×
          </button>
        )}
      </header>

      <div className="provider-dialog-body">
        <div className="provider-dialog-group">
          <div className="provider-dialog-group-title">基础与协议</div>
          <div className="grid gap-3.5 sm:grid-cols-2">
            <label className="provider-dialog-field">
              <span>名称</span>
              <input
                type="text"
                value={name}
                onChange={(e) => onNameChange(e.target.value)}
                placeholder="例如：我的 GPT-4"
                className={inputClassName}
              />
            </label>
            <div className="provider-dialog-field">
              <span>能力类型</span>
              <div className="flex h-9 items-center justify-between rounded-lg border border-line bg-sunken px-3 text-xs">
                <span className="font-medium text-ink">翻译 Provider</span>
                <span className="text-[11px] text-faint">当前仅支持翻译</span>
              </div>
            </div>
          </div>

          <div className="grid gap-3.5 sm:grid-cols-[minmax(0,1fr)_auto]">
            <div className="provider-dialog-field">
              <span>接口协议</span>
              <div className="provider-dialog-segment grid-cols-3">
                {protocolOptions.map((option) => (
                  <button
                    key={option.value}
                    type="button"
                    onClick={() => onProtocolFamilyChange(option.value)}
                    title={option.label}
                    aria-pressed={protocolFamily === option.value}
                    className="provider-dialog-segment-item"
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            </div>

            {protocolFamily === 'openai' && (
              <div className="provider-dialog-field">
                <span>OpenAI 模式</span>
                <div className="provider-dialog-segment grid-cols-2">
                  {openAIModeOptions.map((option) => (
                    <button
                      key={option.value}
                      type="button"
                      onClick={() => onOpenAIModeChange(option.value)}
                      aria-pressed={protocol === option.value}
                      className="provider-dialog-segment-item"
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>

        <div className="provider-dialog-group">
          <div className="provider-dialog-group-title">连接与模型</div>
          <label className="provider-dialog-field">
            <span>API 地址</span>
            <input
              type="text"
              value={endpoint}
              onChange={(e) => onEndpointChange(e.target.value)}
              placeholder="https://api.openai.com"
              className={inputClassName}
            />
            {endpointPreview && (
              <span className="truncate font-mono text-[11px] text-faint">预览：{endpointPreview}</span>
            )}
          </label>

          <div className="grid gap-3.5 sm:grid-cols-2">
            <label className="provider-dialog-field">
              <span>API Key</span>
              <input
                type="password"
                value={apiKey}
                onChange={(e) => onApiKeyChange(e.target.value)}
                placeholder={isEditing ? '留空则保持现有 API Key' : 'sk-...'}
                className={inputClassName}
              />
            </label>

            <div className="provider-dialog-field">
              <div className="flex items-center justify-between gap-2">
                <span>模型</span>
                <button
                  type="button"
                  onClick={onLoadModels}
                  disabled={isLoadingModels || !endpoint.trim() || !apiKey.trim()}
                  className="text-[11px] font-medium text-accent-text hover:underline disabled:cursor-not-allowed disabled:text-faint disabled:no-underline"
                >
                  {isLoadingModels ? '获取中...' : '获取模型列表'}
                </button>
              </div>
              <input
                type="text"
                value={model}
                onChange={(e) => onModelChange(e.target.value)}
                placeholder="gpt-4o / claude-3-5-sonnet-latest / gemini-1.5-flash"
                className={inputClassName}
              />
            </div>
          </div>

          {modelListError && <p className="text-xs text-danger">{modelListError}</p>}
          {models.length > 0 && (
            <div className="max-h-36 divide-y divide-line overflow-y-auto rounded-lg border border-line bg-surface">
              {models.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => onModelChange(item.id)}
                  className={`w-full px-3 py-1.5 text-left text-xs transition-colors hover:bg-accent-soft/60 ${
                    item.id === model ? 'bg-accent-soft text-accent-text font-medium' : 'text-ink'
                  }`}
                >
                  {item.id}
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="provider-dialog-group">
          <div className="provider-dialog-group-title">推理与提示词</div>
          <div className="provider-dialog-field">
            <span>Reasoning 强度 <em className="not-italic text-faint">(可选)</em></span>
            <div className="flex flex-wrap gap-1.5">
              {reasoningOptions.map((option) => (
                <button
                  key={option.value || 'default'}
                  type="button"
                  onClick={() => onReasoningLevelChange(option.value)}
                  aria-pressed={reasoningLevel === option.value}
                  className="provider-dialog-chip"
                >
                  {option.label}
                </button>
              ))}
            </div>
          </div>

          <div className="provider-dialog-field">
            <div className="flex items-center justify-between gap-2">
              <span>翻译策略</span>
              <button
                type="button"
                onClick={() => setStrategyEditorOpen((open) => !open)}
                className="text-[11px] font-medium text-muted transition-colors hover:text-ink"
              >
                {showStrategyEditor ? '收起策略编辑' : '编辑策略'}
              </button>
            </div>
            <div className="flex flex-wrap gap-1.5">
              <button
                type="button"
                onClick={() => onPromptStrategyChange(smartPromptStrategyId)}
                aria-pressed={promptStrategyId === smartPromptStrategyId}
                className="provider-dialog-chip"
              >
                智能选择
              </button>
              {promptStrategies.map((strategy) => (
                <button
                  key={strategy.id}
                  type="button"
                  onClick={() => onPromptStrategyChange(strategy.id)}
                  aria-pressed={promptStrategyId === strategy.id}
                  className="provider-dialog-chip"
                >
                  {strategy.name}
                </button>
              ))}
            </div>
          </div>

          {showStrategyEditor && (
            <div className="space-y-2.5 rounded-lg border border-line bg-sunken p-3">
              <div className="flex items-center justify-between gap-3">
                <span className="text-xs font-medium text-ink">策略编辑</span>
                {selectedPromptStrategy?.is_deletable && (
                  <button
                    type="button"
                    onClick={onDeleteStrategy}
                    className="text-xs font-medium text-danger hover:underline"
                  >
                    删除策略
                  </button>
                )}
              </div>
              <div className="grid gap-2 sm:grid-cols-2">
                <input
                  type="text"
                  value={strategyDraftName}
                  onChange={(event) => onStrategyDraftNameChange(event.target.value)}
                  placeholder="策略名称，例如：法律翻译"
                  className={inputClassName}
                />
                <input
                  type="text"
                  value={strategyDraftDescription}
                  onChange={(event) => onStrategyDraftDescriptionChange(event.target.value)}
                  placeholder="适用说明，例如：合同、条款、法律文本"
                  className={inputClassName}
                />
              </div>
              <textarea
                value={strategyDraftPrompt}
                onChange={(event) => onStrategyDraftPromptChange(event.target.value)}
                placeholder="系统提示词，可使用 {source_lang} 和 {target_lang}"
                rows={3}
                className="provider-dialog-input min-h-[78px] resize-none py-2"
              />
              {strategyError && <p className="text-xs text-danger">{strategyError}</p>}
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={onSaveStrategy}
                  disabled={!selectedPromptStrategy}
                  className="settings-action disabled:cursor-not-allowed disabled:opacity-45"
                >
                  保存策略
                </button>
                <button
                  type="button"
                  onClick={onAddStrategy}
                  className="settings-action"
                >
                  新增为自定义策略
                </button>
              </div>
            </div>
          )}
        </div>

        {saveError && (
          <p role="alert" className="rounded-lg border border-danger/30 bg-danger-soft px-3 py-2 text-xs text-danger">
            {saveError}
          </p>
        )}
      </div>

      <footer className="provider-dialog-footer">
        <div className="flex min-w-0 items-center gap-2.5">
          <button
            type="button"
            onClick={onTestProvider}
            disabled={!canTest}
            className="settings-action h-8 px-3 disabled:cursor-not-allowed disabled:opacity-45"
          >
            {isTestingProvider ? '检测中...' : '检测连接'}
          </button>
          {testStatus && (
            <span className={`truncate text-xs ${testStatus.type === 'success' ? 'text-good' : 'text-danger'}`}>
              {testStatus.message}
            </span>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <button
            type="button"
            onClick={onCancel}
            disabled={isSaving}
            className="h-8 rounded-lg border border-line bg-surface px-3.5 text-xs font-medium text-muted transition-colors hover:bg-raised hover:text-ink disabled:opacity-45"
          >
            取消
          </button>
          <button
            type="button"
            onClick={onSave}
            disabled={!canSave}
            className="h-8 rounded-lg bg-accent px-4 text-xs font-semibold text-accent-on transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:bg-line-strong"
          >
            {isSaving ? '保存中...' : isEditing ? '保存配置' : '添加'}
          </button>
        </div>
      </footer>
    </section>
  );
}
