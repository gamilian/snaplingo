export interface AddCustomTranslationProviderRequest {
  name: string;
  protocol: string;
  endpoint: string;
  model: string;
  api_key: string;
  reasoning_level?: string;
  prompt_strategy_id?: string;
  prompt_fallback_strategy_id?: string;
}

export interface UpdateCustomTranslationProviderRequest {
  name: string;
  protocol: string;
  endpoint: string;
  model: string;
  api_key?: string;
  reasoning_level?: string;
  prompt_strategy_id?: string;
  prompt_fallback_strategy_id?: string;
}

export interface ProviderModelInfo {
  id: string;
}

export interface TranslationPromptStrategy {
  id: string;
  name: string;
  description: string;
  system_prompt: string;
  is_builtin: boolean;
  is_deletable: boolean;
}

export interface TranslationPromptStrategyConfig {
  strategies: TranslationPromptStrategy[];
}
