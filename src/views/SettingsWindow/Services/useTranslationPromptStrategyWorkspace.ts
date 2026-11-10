import { useState } from 'react';

import type {
  TranslationPromptStrategy,
  TranslationPromptStrategyConfig,
} from './providerViewTypes';
import {
  DEFAULT_PROMPT_STRATEGIES,
  DEFAULT_PROMPT_STRATEGY_ID,
  formatCustomProviderError,
  SMART_PROMPT_STRATEGY_ID,
} from './customTranslationProviderFormModel';

export interface TranslationPromptStrategyClients {
  listTranslationPromptStrategies(): Promise<TranslationPromptStrategyConfig>;
  saveTranslationPromptStrategies(
    config: TranslationPromptStrategyConfig,
  ): Promise<TranslationPromptStrategyConfig>;
}

export interface TranslationPromptStrategyWorkspace {
  promptStrategies: TranslationPromptStrategy[];
  selectedPromptStrategy: TranslationPromptStrategy | undefined;
  strategyDraftName: string;
  strategyDraftDescription: string;
  strategyDraftPrompt: string;
  strategyError: string | null;
  setStrategyDraftName(name: string): void;
  setStrategyDraftDescription(description: string): void;
  setStrategyDraftPrompt(prompt: string): void;
  clearStrategyDraft(): void;
  loadPromptStrategies(
    strategyId: string,
    fallbackStrategies?: TranslationPromptStrategy[],
  ): Promise<void>;
  handlePromptStrategyChange(strategyId: string): void;
  handleSaveStrategy(): Promise<void>;
  handleAddStrategy(): Promise<void>;
  handleDeleteStrategy(): Promise<void>;
}

export function useTranslationPromptStrategyWorkspace({
  clients,
  createStrategyId = () => `custom-${Date.now()}`,
  onSelectedStrategyIdChange,
  selectedStrategyId,
}: {
  selectedStrategyId: string;
  onSelectedStrategyIdChange(strategyId: string): void;
  clients: TranslationPromptStrategyClients;
  createStrategyId?: () => string;
}): TranslationPromptStrategyWorkspace {
  const [promptStrategies, setPromptStrategies] = useState<TranslationPromptStrategy[]>(
    DEFAULT_PROMPT_STRATEGIES,
  );
  const [strategyDraftName, setStrategyDraftName] = useState('');
  const [strategyDraftDescription, setStrategyDraftDescription] = useState('');
  const [strategyDraftPrompt, setStrategyDraftPrompt] = useState('');
  const [strategyError, setStrategyError] = useState<string | null>(null);
  const selectedPromptStrategy = promptStrategies.find(
    (strategy) => strategy.id === selectedStrategyId,
  );

  const clearStrategyDraft = () => {
    setStrategyDraftName('');
    setStrategyDraftDescription('');
    setStrategyDraftPrompt('');
  };

  const populateStrategyDraft = (
    strategyId: string,
    strategies: TranslationPromptStrategy[],
  ) => {
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
  };

  const loadPromptStrategies = async (
    strategyId: string,
    fallbackStrategies?: TranslationPromptStrategy[],
  ) => {
    if (fallbackStrategies) {
      setPromptStrategies(fallbackStrategies);
      populateStrategyDraft(strategyId, fallbackStrategies);
      return;
    }

    try {
      const config = await clients.listTranslationPromptStrategies();
      setPromptStrategies(config.strategies);
      populateStrategyDraft(strategyId, config.strategies);
    } catch (error) {
      console.error('Failed to load prompt strategies:', error);
      setPromptStrategies(DEFAULT_PROMPT_STRATEGIES);
      populateStrategyDraft(strategyId, DEFAULT_PROMPT_STRATEGIES);
    }
  };

  const handlePromptStrategyChange = (strategyId: string) => {
    onSelectedStrategyIdChange(strategyId);
    setStrategyError(null);
    populateStrategyDraft(strategyId, promptStrategies);
  };

  const persistPromptStrategies = async (
    strategies: TranslationPromptStrategy[],
    nextSelectedId: string,
  ) => {
    setStrategyError(null);
    try {
      const saved = await clients.saveTranslationPromptStrategies({ strategies });
      setPromptStrategies(saved.strategies);
      onSelectedStrategyIdChange(nextSelectedId);
      populateStrategyDraft(nextSelectedId, saved.strategies);
    } catch (error) {
      setStrategyError(`保存策略失败: ${formatCustomProviderError(error)}`);
    }
  };

  const validateDraft = () => {
    const trimmedName = strategyDraftName.trim();
    const trimmedPrompt = strategyDraftPrompt.trim();
    if (!trimmedName || !trimmedPrompt) {
      setStrategyError('策略名称和系统提示词不能为空');
      return null;
    }

    return {
      name: trimmedName,
      description: strategyDraftDescription.trim(),
      systemPrompt: trimmedPrompt,
    };
  };

  const handleSaveStrategy = async () => {
    if (!selectedPromptStrategy) return;

    const draft = validateDraft();
    if (!draft) return;

    const nextStrategies = promptStrategies.map((strategy) =>
      strategy.id === selectedPromptStrategy.id
        ? {
            ...strategy,
            name: draft.name,
            description: draft.description,
            system_prompt: draft.systemPrompt,
          }
        : strategy,
    );

    await persistPromptStrategies(nextStrategies, selectedPromptStrategy.id);
  };

  const handleAddStrategy = async () => {
    const draft = validateDraft();
    if (!draft) return;

    const strategy: TranslationPromptStrategy = {
      id: createStrategyId(),
      name: draft.name,
      description: draft.description,
      system_prompt: draft.systemPrompt,
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

  return {
    promptStrategies,
    selectedPromptStrategy,
    strategyDraftName,
    strategyDraftDescription,
    strategyDraftPrompt,
    strategyError,
    setStrategyDraftName,
    setStrategyDraftDescription,
    setStrategyDraftPrompt,
    clearStrategyDraft,
    loadPromptStrategies,
    handlePromptStrategyChange,
    handleSaveStrategy,
    handleAddStrategy,
    handleDeleteStrategy,
  };
}
