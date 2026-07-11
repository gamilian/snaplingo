import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type {
  AddCustomTranslationProviderRequest as ApplicationAddCustomTranslationProviderRequest,
  OcrProviderInfo,
  ProviderInfo,
  SettingsProvidersPort,
  UpdateCustomTranslationProviderRequest as ApplicationUpdateCustomTranslationProviderRequest,
} from '../application/settings/ports';

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

let providersRuntime: SettingsProvidersPort | null = null;

export function initializeProviderStore(runtime: SettingsProvidersPort) {
  providersRuntime = runtime;
}

function runtime() {
  if (!providersRuntime) {
    throw new Error('Provider store runtime has not been initialized');
  }

  return providersRuntime;
}

export interface Provider {
  id: string;
  name: string;
  type: 'ocr' | 'translation' | 'tts';
  status: 'active' | 'inactive' | 'unconfigured';
  isBuiltin: boolean;
  description?: string;
  requiresApiKey: boolean;
  config?: {
    apiKey?: string;
    endpoint?: string;
    model?: string;
    [key: string]: any;
  };
  // Custom provider 额外字段
  protocol?: string;
  endpoint?: string;
  model?: string;
  reasoningLevel?: string;
  promptStrategyId?: string;
  promptFallbackStrategyId?: string;
}

interface ProviderState {
  // Provider 列表
  ocrProviders: Provider[];
  translationProviders: Provider[];
  ttsProviders: Provider[];

  // 激活状态
  activeOcrProvider: string | null;
  activeTranslationProviders: string[];
  activeTtsProvider: string | null;

  // Async Actions (后端驱动)
  loadTranslationProviders: () => Promise<void>;
  activateTranslationProvider: (id: string) => Promise<void>;
  deactivateTranslationProvider: (id: string) => Promise<void>;
  addCustomTranslationProvider: (request: AddCustomTranslationProviderRequest) => Promise<void>;
  updateCustomTranslationProvider: (
    id: string,
    request: UpdateCustomTranslationProviderRequest,
  ) => Promise<void>;
  removeTranslationProvider: (id: string) => Promise<void>;
  testCustomTranslationProvider: (id: string) => Promise<void>;

  // OCR Actions (后端驱动)
  loadOcrProviders: () => Promise<void>;
  activateOcrProvider: (id: string) => Promise<void>;
  configureOcrProvider: (providerId: string, credentials: Record<string, string>) => Promise<void>;

  // TTS Actions (保留同步)
  activateTtsProvider: (id: string) => void;

  updateProviderConfig: (id: string, providerId: string, config: any) => Promise<void>;
  reorderTranslationProviders: (ids: string[]) => Promise<void>;
}

// 内置 TTS Providers（本地临时数据）
const builtinTtsProviders: Provider[] = [
  {
    id: 'system-tts',
    name: '系统语音',
    type: 'tts',
    status: 'active',
    isBuiltin: true,
    description: '使用系统内置的 TTS 引擎',
    requiresApiKey: false,
  },
];

// 转换后端 ProviderInfo 到前端 Provider
function convertProviderInfo(info: ProviderInfo): Provider {
  return {
    id: info.id,
    name: displayProviderName(info),
    type: 'translation',
    status: info.isActive ? 'active' : (info.isConfigured ? 'inactive' : 'unconfigured'),
    isBuiltin: info.isBuiltin,
    requiresApiKey: info.requiresApiKey,
    protocol: info.protocol ?? undefined,
    endpoint: info.endpoint ?? undefined,
    model: info.model ?? undefined,
    reasoningLevel: info.reasoningLevel ?? undefined,
    promptStrategyId: info.promptStrategyId ?? undefined,
    promptFallbackStrategyId: info.promptFallbackStrategyId ?? undefined,
  };
}

function displayProviderName(info: ProviderInfo): string {
  if (!info.isBuiltin && info.name.startsWith('custom-llm-') && info.model) {
    return info.model;
  }

  return info.name;
}

function normalizeTranslationCredentials(config: unknown): Record<string, string> {
  if (
    config &&
    typeof config === 'object' &&
    'apiKey' in config &&
    typeof (config as { apiKey?: unknown }).apiKey === 'string'
  ) {
    return { api_key: (config as { apiKey: string }).apiKey };
  }

  return config as Record<string, string>;
}

function toAddCustomTranslationRequest(
  request: AddCustomTranslationProviderRequest,
): ApplicationAddCustomTranslationProviderRequest {
  return {
    name: request.name,
    protocol: request.protocol,
    endpoint: request.endpoint,
    model: request.model,
    apiKey: request.api_key,
    reasoningLevel: request.reasoning_level,
    promptStrategyId: request.prompt_strategy_id,
    promptFallbackStrategyId: request.prompt_fallback_strategy_id,
  };
}

function toUpdateCustomTranslationRequest(
  request: UpdateCustomTranslationProviderRequest,
): ApplicationUpdateCustomTranslationProviderRequest {
  return {
    name: request.name,
    protocol: request.protocol,
    endpoint: request.endpoint,
    model: request.model,
    apiKey: request.api_key,
    reasoningLevel: request.reasoning_level,
    promptStrategyId: request.prompt_strategy_id,
    promptFallbackStrategyId: request.prompt_fallback_strategy_id,
  };
}

export const useProviderStore = create<ProviderState>()(
  persist(
    (set, get) => ({
      // 初始数据
      ocrProviders: [], // 从后端加载
      translationProviders: [], // 从后端加载
      ttsProviders: builtinTtsProviders,

      activeOcrProvider: null,
      activeTranslationProviders: [],
      activeTtsProvider: 'system-tts',

      // 从后端加载翻译 Providers
      loadTranslationProviders: async () => {
        try {
          const providers = await runtime().listTranslation();
          const converted = providers.map(convertProviderInfo);
          const activeIds = converted.filter(p => p.status === 'active').map(p => p.id);

          set({
            translationProviders: converted,
            activeTranslationProviders: activeIds,
          });
        } catch (error) {
          console.error('Failed to load translation providers:', error);
        }
      },

      // 激活翻译 Provider
      activateTranslationProvider: async (id: string) => {
        try {
          await runtime().activateTranslation(id);
          await get().loadTranslationProviders();
        } catch (error) {
          console.error('Failed to activate provider:', error);
          throw error;
        }
      },

      // 停用翻译 Provider
      deactivateTranslationProvider: async (id: string) => {
        try {
          await runtime().deactivateTranslation(id);
          await get().loadTranslationProviders();
        } catch (error) {
          console.error('Failed to deactivate provider:', error);
          throw error;
        }
      },

      // 添加自定义翻译 Provider
      addCustomTranslationProvider: async (request: AddCustomTranslationProviderRequest) => {
        try {
          await runtime().addCustomTranslation(
            toAddCustomTranslationRequest(request),
          );
          await get().loadTranslationProviders();
        } catch (error) {
          console.error('Failed to add custom provider:', error);
          throw error;
        }
      },

      updateCustomTranslationProvider: async (
        id: string,
        request: UpdateCustomTranslationProviderRequest,
      ) => {
        try {
          await runtime().updateCustomTranslation(
            id,
            toUpdateCustomTranslationRequest(request),
          );
          await get().loadTranslationProviders();
        } catch (error) {
          console.error('Failed to update custom provider:', error);
          throw error;
        }
      },

      // 删除翻译 Provider
      removeTranslationProvider: async (id: string) => {
        try {
          await runtime().removeCustomTranslation(id);
          await get().loadTranslationProviders();
        } catch (error) {
          console.error('Failed to remove provider:', error);
          throw error;
        }
      },

      testCustomTranslationProvider: async (id: string) => {
        await runtime().testCustomTranslation(id);
      },

      // 从后端加载 OCR Providers
      loadOcrProviders: async () => {
        try {
          const providers = await runtime().listOcr();
          const converted = providers.map((p: OcrProviderInfo) => ({
            id: p.id,
            name: p.name,
            type: 'ocr' as const,
            status: p.isActive ? 'active' as const : (p.isConfigured ? 'inactive' as const : 'unconfigured' as const),
            isBuiltin: true,
            requiresApiKey: p.requiresApiKey,
          }));

          const activeProvider = converted.find(p => p.status === 'active');

          set({
            ocrProviders: converted,
            activeOcrProvider: activeProvider?.id || null,
          });
        } catch (error) {
          console.error('Failed to load OCR providers:', error);
        }
      },

      // OCR Provider 激活
      activateOcrProvider: async (id: string) => {
        try {
          await runtime().activateOcr(id);
          await get().loadOcrProviders();
        } catch (error) {
          console.error('Failed to activate OCR provider:', error);
          throw error;
        }
      },

      // 配置 OCR Provider
      configureOcrProvider: async (providerId: string, credentials: Record<string, string>) => {
        try {
          await runtime().configureOcrCredentials(providerId, credentials);
          await get().loadOcrProviders();
        } catch (error) {
          console.error('Failed to configure OCR provider:', error);
          throw error;
        }
      },

      // TTS Provider 激活（单选）
      activateTtsProvider: (id) =>
        set((state) => ({
          activeTtsProvider: id,
          ttsProviders: state.ttsProviders.map((p) =>
            p.id === id ? { ...p, status: 'active' as const } : { ...p, status: 'inactive' as const }
          ),
        })),

      // 更新 Provider 配置
      updateProviderConfig: async (_id: string, providerId: string, config: any) => {
        try {
          const credentials = normalizeTranslationCredentials(config);
          await runtime().configureTranslationCredentials(providerId, credentials);
          await get().loadTranslationProviders();
        } catch (error) {
          console.error('Failed to update provider config:', error);
          throw error;
        }
      },

      // 重新排序翻译 Provider（本地状态）
      reorderTranslationProviders: async (ids: string[]) => {
        try {
          // 只对 active providers 进行排序
          const activeIds = get().activeTranslationProviders;
          const reorderedActiveIds = ids.filter((id) => activeIds.includes(id));

          if (reorderedActiveIds.length !== activeIds.length) {
            console.warn('Reorder skipped: not all active providers included');
            return;
          }

          // 调用后端命令更新顺序
          await runtime().reorderActiveTranslation(reorderedActiveIds);

          // 重新加载以同步状态
          await get().loadTranslationProviders();
        } catch (error) {
          console.error('Failed to reorder providers:', error);
          throw error;
        }
      },
    }),
    {
      name: 'snaplingo-providers',
      // 不再持久化 translationProviders 和 activeTranslationProviders
      partialize: (state) => ({
        ocrProviders: state.ocrProviders,
        ttsProviders: state.ttsProviders,
        activeOcrProvider: state.activeOcrProvider,
        activeTtsProvider: state.activeTtsProvider,
      }),
    }
  )
);
