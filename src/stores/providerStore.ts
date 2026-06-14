import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';

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
}

interface ProviderInfo {
  id: string;
  name: string;
  is_configured: boolean;
  requires_api_key: boolean;
  is_active: boolean;
  is_builtin: boolean;
  protocol?: string;
  endpoint?: string;
  model?: string;
  reasoning_level?: string;
}

interface AddCustomTranslationProviderRequest {
  name: string;
  protocol: string;
  endpoint: string;
  model: string;
  api_key: string;
  reasoning_level?: string;
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
  removeTranslationProvider: (id: string) => Promise<void>;

  // Sync Actions (保留用于 OCR/TTS)
  activateOcrProvider: (id: string) => void;
  activateTtsProvider: (id: string) => void;

  updateProviderConfig: (id: string, providerId: string, config: any) => Promise<void>;
  reorderTranslationProviders: (ids: string[]) => Promise<void>;
}

// 内置 OCR/TTS Providers（暂时保留本地数据，后续迁移）
const builtinOcrProviders: Provider[] = [
  {
    id: 'tesseract',
    name: 'Tesseract',
    type: 'ocr',
    status: 'active',
    isBuiltin: true,
    description: '免费开源 OCR 引擎，本地运行',
    requiresApiKey: false,
  },
  {
    id: 'paddleocr',
    name: 'PaddleOCR',
    type: 'ocr',
    status: 'inactive',
    isBuiltin: true,
    description: '百度开源 OCR，中文识别优化',
    requiresApiKey: false,
  },
  {
    id: 'baidu-ocr',
    name: '百度 OCR',
    type: 'ocr',
    status: 'unconfigured',
    isBuiltin: true,
    description: '百度云 OCR API',
    requiresApiKey: true,
  },
];

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
    name: info.name,
    type: 'translation',
    status: info.is_active ? 'active' : (info.is_configured ? 'inactive' : 'unconfigured'),
    isBuiltin: info.is_builtin,
    requiresApiKey: info.requires_api_key,
    protocol: info.protocol,
    endpoint: info.endpoint,
    model: info.model,
    reasoningLevel: info.reasoning_level,
  };
}

export const useProviderStore = create<ProviderState>()(
  persist(
    (set, get) => ({
      // 初始数据
      ocrProviders: builtinOcrProviders,
      translationProviders: [], // 从后端加载
      ttsProviders: builtinTtsProviders,

      activeOcrProvider: 'tesseract',
      activeTranslationProviders: [],
      activeTtsProvider: 'system-tts',

      // 从后端加载翻译 Providers
      loadTranslationProviders: async () => {
        try {
          const providers = await invoke<ProviderInfo[]>('list_translation_providers');
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
          await invoke('activate_translation_provider', { providerId: id });
          await get().loadTranslationProviders();
        } catch (error) {
          console.error('Failed to activate provider:', error);
          throw error;
        }
      },

      // 停用翻译 Provider
      deactivateTranslationProvider: async (id: string) => {
        try {
          await invoke('deactivate_translation_provider', { providerId: id });
          await get().loadTranslationProviders();
        } catch (error) {
          console.error('Failed to deactivate provider:', error);
          throw error;
        }
      },

      // 添加自定义翻译 Provider
      addCustomTranslationProvider: async (request: AddCustomTranslationProviderRequest) => {
        try {
          await invoke('add_custom_translation_provider', { request });
          await get().loadTranslationProviders();
        } catch (error) {
          console.error('Failed to add custom provider:', error);
          throw error;
        }
      },

      // 删除翻译 Provider
      removeTranslationProvider: async (id: string) => {
        try {
          await invoke('remove_custom_translation_provider', { providerId: id });
          await get().loadTranslationProviders();
        } catch (error) {
          console.error('Failed to remove provider:', error);
          throw error;
        }
      },

      // OCR Provider 激活（单选）
      activateOcrProvider: (id) =>
        set((state) => ({
          activeOcrProvider: id,
          ocrProviders: state.ocrProviders.map((p) =>
            p.id === id ? { ...p, status: 'active' as const } : { ...p, status: 'inactive' as const }
          ),
        })),

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
          // Check if config is a credentials map (new format)
          if (typeof config === 'object' && !config.apiKey) {
            // New format: HashMap<String, String>
            await invoke('configure_translation_provider_credentials', {
              providerId,
              credentials: config,
            });
          } else if (config.apiKey) {
            // Legacy format: single api_key
            await invoke('configure_translation_provider', {
              providerId,
              apiKey: config.apiKey,
            });
          }
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
          await invoke('reorder_active_translation_providers', {
            providerIds: reorderedActiveIds,
          });

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
