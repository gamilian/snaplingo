// Fixtures for the actual Services views. No installed settings or network calls.
import type { SettingsRuntime } from '../../../../src/application/settings/runtime';
import type { CredentialField, TranslationPromptStrategyConfig } from '../../../../src/application/settings/ports';
import { useProviderStore, type Provider } from '../../../../src/stores/providerStore';
import { useSettingsConfigStore } from '../../../../src/stores/settingsConfigStore';
import { useSettingsStore } from '../../../../src/stores/settingsStore';

export function initializeServicesReference(runtime: SettingsRuntime) {
  let promptSaves = 0;
  const ocrProviders: Provider[] = [
    { id: 'system-ocr', name: '系统 OCR', type: 'ocr', status: 'active', isBuiltin: true, isLocal: true, requiresApiKey: false },
    { id: 'paddle-ocr', name: 'PaddleOCR', type: 'ocr', status: 'inactive', isBuiltin: true, isLocal: true, requiresApiKey: false, acceleration: { gpuEnabled: true, device: 'cpu', fallbackReason: null } },
    { id: 'baidu-ocr', name: '百度 OCR', type: 'ocr', status: 'unconfigured', isBuiltin: true, requiresApiKey: true },
  ];
  const translationProviders: Provider[] = [
    { id: 'google-translate', name: 'Google 翻译', type: 'translation', status: 'active', isBuiltin: true, requiresApiKey: false },
    { id: 'custom-reference', name: '我的 GPT', type: 'translation', status: 'active', isBuiltin: false, requiresApiKey: true, protocol: 'openai-responses', endpoint: 'https://api.example.com', model: 'gpt-5-mini', reasoningLevel: 'medium', promptStrategyId: 'legal' },
    { id: 'deeplx', name: 'DeepLX', type: 'translation', status: 'inactive', isBuiltin: true, requiresApiKey: true },
    { id: 'baidu-translate', name: '百度翻译', type: 'translation', status: 'unconfigured', isBuiltin: true, requiresApiKey: true },
  ];
  const route = new URLSearchParams(location.search).get('route');
  useSettingsStore.setState({ activeMainTab: 'services', servicesSubTab: route === 'translation' || route === 'tts' ? route : 'ocr' });
  useProviderStore.setState({
    ocrProviders, translationProviders, activeOcrProvider: 'system-ocr', activeTranslationProviders: ['google-translate', 'custom-reference'],
    loadOcrProviders: async () => {}, loadTranslationProviders: async () => {},
  });
  useSettingsConfigStore.setState((state) => ({
    screenshot: { ...state.screenshot!, captureUiScale: '100' },
    ocr: { ...state.ocr!, gpuAcceleration: true },
    general: { ...state.general!, systemTtsVoice: 'com.fixture.voice.zh', systemTtsRate: 180 },
    updateGeneralSettings: async (input) => {
      useSettingsConfigStore.setState((current) => ({ general: { ...current.general!, ...input } }));
      const current = useSettingsConfigStore.getState();
      return { general: current.general!, screenshot: current.screenshot!, ocr: current.ocr!, translation: current.translation!, history: current.history! };
    },
  }));
  const credentialSchema = async (id: string): Promise<CredentialField[]> => id === 'deeplx' ? [] : [
    { name: id === 'baidu-translate' ? 'app_id' : 'api_key', label: id === 'baidu-translate' ? 'APP ID' : 'API Key', secret: false },
    { name: 'secret_key', label: id === 'baidu-translate' ? '密钥' : 'Secret Key', secret: true },
  ];
  Object.assign(runtime.window, { displayBounds: async () => null, applyInterfaceScale: async () => {}, subscribeNavigationRequested: async () => () => {} });
  Object.assign(runtime, {
    providers: {
      getOcrCredentialSchema: credentialSchema, getTranslationCredentialSchema: credentialSchema,
      listTranslationPromptStrategies: async () => ({ strategies: [
        { id: 'legal', name: '法律翻译', description: '合同、条款、法律文本', systemPrompt: '你是一位专业法律译者。\n将 {source_lang} 翻译为 {target_lang}。\n保留条款结构与专有名词。', isBuiltin: false, isDeletable: true },
      ] }),
      saveTranslationPromptStrategies: async (config: TranslationPromptStrategyConfig) => { promptSaves += 1; return structuredClone(config); },
    },
    tts: { listVoices: async () => [
      { id: 'com.fixture.voice.zh', name: '婷婷', locale: 'zh-CN' },
      { id: 'com.fixture.voice.en', name: 'Samantha', locale: 'en-US' },
    ], speak: async () => {} },
  });
  Object.assign(window, { servicesReference: {
    route: (route: 'ocr' | 'translation' | 'tts') => useSettingsStore.setState({ servicesSubTab: route }),
    snapshot: () => ({ route: useSettingsStore.getState().servicesSubTab, providers: useProviderStore.getState().translationProviders, general: useSettingsConfigStore.getState().general, promptSaves }),
  } });
}
