import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface Provider {
  id: string;
  name: string;
  type: 'ocr' | 'translation' | 'tts';
  status: 'active' | 'inactive' | 'unconfigured';
  isBuiltin: boolean;
  description: string;
  requiresApiKey: boolean;
  config?: {
    apiKey?: string;
    endpoint?: string;
    model?: string;
    [key: string]: any;
  };
}

interface ProviderState {
  // Provider 列表
  ocrProviders: Provider[];
  translationProviders: Provider[];
  ttsProviders: Provider[];

  // 激活状态
  activeOcrProvider: string | null;
  activeTranslationProviders: string[]; // 翻译支持多选
  activeTtsProvider: string | null;

  // Actions
  activateOcrProvider: (id: string) => void;
  activateTranslationProvider: (id: string) => void;
  deactivateTranslationProvider: (id: string) => void;
  activateTtsProvider: (id: string) => void;

  updateProviderConfig: (id: string, config: any) => void;
  reorderTranslationProviders: (ids: string[]) => void;

  // 添加自定义 Provider（仅翻译）
  addCustomTranslationProvider: (provider: Omit<Provider, 'id' | 'type' | 'isBuiltin'>) => void;
  removeProvider: (id: string) => void;
}

// 内置 Provider 数据
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

const builtinTranslationProviders: Provider[] = [
  {
    id: 'google-translate',
    name: 'Google 翻译',
    type: 'translation',
    status: 'active',
    isBuiltin: true,
    description: '免费的 Google 翻译 API',
    requiresApiKey: false,
  },
  {
    id: 'deepl',
    name: 'DeepL',
    type: 'translation',
    status: 'unconfigured',
    isBuiltin: true,
    description: '高质量翻译服务',
    requiresApiKey: true,
  },
  {
    id: 'baidu-translate',
    name: '百度翻译',
    type: 'translation',
    status: 'unconfigured',
    isBuiltin: true,
    description: '百度翻译 API',
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

export const useProviderStore = create<ProviderState>()(
  persist(
    (set) => ({
      // 初始数据
      ocrProviders: builtinOcrProviders,
      translationProviders: builtinTranslationProviders,
      ttsProviders: builtinTtsProviders,

      activeOcrProvider: 'tesseract',
      activeTranslationProviders: ['google-translate'],
      activeTtsProvider: 'system-tts',

      // OCR Provider 激活（单选）
      activateOcrProvider: (id) =>
        set((state) => ({
          activeOcrProvider: id,
          ocrProviders: state.ocrProviders.map((p) =>
            p.id === id ? { ...p, status: 'active' as const } : { ...p, status: 'inactive' as const }
          ),
        })),

      // 翻译 Provider 激活（多选）
      activateTranslationProvider: (id) =>
        set((state) => ({
          activeTranslationProviders: [...state.activeTranslationProviders, id],
          translationProviders: state.translationProviders.map((p) =>
            p.id === id ? { ...p, status: 'active' as const } : p
          ),
        })),

      deactivateTranslationProvider: (id) =>
        set((state) => ({
          activeTranslationProviders: state.activeTranslationProviders.filter((pid) => pid !== id),
          translationProviders: state.translationProviders.map((p) =>
            p.id === id ? { ...p, status: 'inactive' as const } : p
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
      updateProviderConfig: (id, config) =>
        set((state) => ({
          ocrProviders: state.ocrProviders.map((p) =>
            p.id === id ? { ...p, config, status: 'inactive' as const } : p
          ),
          translationProviders: state.translationProviders.map((p) =>
            p.id === id ? { ...p, config, status: 'inactive' as const } : p
          ),
          ttsProviders: state.ttsProviders.map((p) =>
            p.id === id ? { ...p, config, status: 'inactive' as const } : p
          ),
        })),

      // 重新排序翻译 Provider
      reorderTranslationProviders: (ids) =>
        set((state) => {
          const orderedProviders = ids
            .map((id) => state.translationProviders.find((p) => p.id === id))
            .filter((p): p is Provider => p !== undefined);
          return { translationProviders: orderedProviders };
        }),

      // 添加自定义翻译 Provider
      addCustomTranslationProvider: (provider) =>
        set((state) => ({
          translationProviders: [
            ...state.translationProviders,
            {
              ...provider,
              id: `custom-${Date.now()}`,
              type: 'translation' as const,
              isBuiltin: false,
              status: 'unconfigured' as const,
            },
          ],
        })),

      // 移除 Provider（仅自定义）
      removeProvider: (id) =>
        set((state) => ({
          translationProviders: state.translationProviders.filter((p) => p.id !== id || p.isBuiltin),
        })),
    }),
    {
      name: 'snaplingo-providers',
    }
  )
);
