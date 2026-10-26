// Local visual QA uses the production settings runtime with in-memory ports.
import { createSettingsRuntime } from '../../src/application/settings/runtime';
import { createRequiredPermissionsRuntime } from '../../src/application/permissions/runtime';
import { createAppUpdatesRuntime } from '../../src/application/updates/runtime';
import type { SettingsSnapshot, HotkeySnapshot, ProviderInfo } from '../../src/application/settings/ports';
import { initializeSettingsConfigStore } from '../../src/stores/settingsConfigStore';
import { initializeHotkeyConfigStore } from '../../src/stores/hotkeyConfigStore';
import { initializeProviderStore } from '../../src/stores/providerStore';

const key = 'snaplingo-production-theme-preview-v1';
const copy = <T,>(value: T): T => JSON.parse(JSON.stringify(value));
const noop = async () => undefined;
const unsubscribe = async () => () => undefined;
const emptyPage = async () => ({ items: [], total: 0 });
const disconnected = async () => { throw new Error('主题预览未连接网络服务'); };
const initial: SettingsSnapshot = {
  general: { language: 'zh-CN', interfaceTheme: 'native', theme: 'light', startOnBoot: false },
  screenshot: { savePath: '~/Pictures/SnapLingo', format: 'png', quality: 90, namingRule: 'timestamp', customFileName: 'SnapLingo', autoCopy: false, defaultStrokeWidth: 2, defaultFontSize: 24, rememberLastTool: true, showSelectionSize: true, showMagnifier: false, pinOpacity: 100, pinShadow: true, annotationColors: [[255,77,79,255],[40,167,69,255],[24,144,255,255],[250,219,20,255],[255,255,255,255],[0,0,0,255]] },
  translation: { defaultSourceLang: 'auto', defaultTargetLang: 'zh-CN', autoTranslate: true, autoCopy: false, preserveLineBreaks: true, incrementalTranslation: false, windowAlwaysOnTop: true, hideOnBlur: true, windowWidth: 660, maxWindowHeightRatio: 70 },
  ocr: { recognitionLanguage: 'auto', preserveFormatting: true, removeChineseSpaces: true, showConfidence: false },
  history: { autoCleanupEnabled: false, retentionDays: 30, maximumRecords: 5000, maximumFavorites: 1000 },
};
let snapshot = copy(initial);
try { const saved = JSON.parse(localStorage.getItem(key) || 'null'); if (saved) snapshot = { ...initial, ...saved, general: { ...initial.general, ...saved.general } }; } catch { /* Use the preview defaults. */ }
const save = async <K extends keyof SettingsSnapshot>(section: K, value: SettingsSnapshot[K]) => {
  snapshot = { ...snapshot, [section]: copy(value) };
  localStorage.setItem(key, JSON.stringify(snapshot));
  return copy(snapshot);
};
const defaults: HotkeySnapshot = {
  screenshot: { screenshot: 'F1', 'screenshot-copy': '⌘F1', pin: 'F3', 'pin-toggle-all': '⇧F3', 'pin-switch-group': '⌘F3' },
  translation: { 'selection-translate': '⌥D', 'screenshot-translate': '⌥S', 'show-window': '⌥A' },
  ocr: { 'screenshot-ocr': '⇧⌥S', 'silent-screenshot-ocr': '未设置', 'file-ocr': '未设置' },
};
let hotkeys = copy(defaults);
let providers: ProviderInfo[] = [
  { id: 'google-translate', name: 'Google 翻译', isConfigured: true, requiresApiKey: false, isActive: true, isBuiltin: true, protocol: null, endpoint: null, model: null, reasoningLevel: null, promptStrategyId: null, promptFallbackStrategyId: null },
  { id: 'deeplx', name: 'DeepLX', isConfigured: true, requiresApiKey: false, isActive: true, isBuiltin: true, protocol: null, endpoint: 'http://localhost:1188/translate', model: null, reasoningLevel: null, promptStrategyId: null, promptFallbackStrategyId: null },
  { id: 'baidu-translate', name: '百度翻译', isConfigured: false, requiresApiKey: true, isActive: false, isBuiltin: true, protocol: null, endpoint: null, model: null, reasoningLevel: null, promptStrategyId: null, promptFallbackStrategyId: null },
];
let ocrProviders = [
  { id: 'system-ocr', name: 'Apple Vision', isConfigured: true, requiresApiKey: false, isActive: true, isLocal: true },
  { id: 'baidu-ocr', name: '百度 OCR', isConfigured: false, requiresApiKey: true, isActive: false, isLocal: false },
];
const permissionStatus = { screenRecording: true, accessibility: true };
export const runtime = createSettingsRuntime({
  permissions: createRequiredPermissionsRuntime({ status: async () => permissionStatus, context: async () => ({ platform: 'macos', appPath: null, needsInstallation: false, canReset: false }), request: async () => permissionStatus, reset: async () => permissionStatus, restart: noop }),
  updates: createAppUpdatesRuntime({ check: async () => ({ currentVersion: '0.1.7', latestVersion: null, available: false, downloadSize: null, notes: '', publishedAt: null, platform: 'macos' }), downloadAndOpen: async () => '', openReleasePage: noop }),
  window: { openSettings: noop, selectScreenshotDirectory: async () => null, getAppVersion: async () => '0.1.7' },
  windowEvents: { subscribeNavigationRequested: unsubscribe },
  configurationEvents: { subscribeSettingsChanged: unsubscribe, subscribeProvidersChanged: unsubscribe, subscribeHotkeysChanged: unsubscribe, subscribeHistoryChanged: unsubscribe, subscribeFavoritesChanged: unsubscribe, subscribeScreenshotFavoritesChanged: unsubscribe },
  durableSettings: { getSettingsSnapshot: async () => copy(snapshot), updateGeneralSettings: input => save('general', input), updateScreenshotSettings: input => save('screenshot', input), updateAnnotationColors: colors => save('screenshot', { ...snapshot.screenshot, annotationColors: colors }), updateTranslationSettings: input => save('translation', input), updateOcrSettings: input => save('ocr', input), updateHistorySettings: input => save('history', input) },
  hotkeys: {
    beginHotkeyRecording: noop, endHotkeyRecording: noop, subscribeRecordedHotkey: unsubscribe,
    getHotkeySnapshot: async () => copy(hotkeys), getDefaultHotkeySnapshot: async () => copy(defaults),
    updateHotkey: async ({ category, action, hotkey }) => { hotkeys[category][action] = hotkey; return { snapshot: copy(hotkeys), accelerator: hotkey }; },
    resetHotkey: async (category, action) => { hotkeys[category][action] = defaults[category][action]; return { snapshot: copy(hotkeys), accelerator: hotkeys[category][action] }; },
    resetHotkeyCategory: async category => { hotkeys[category] = copy(defaults[category]); return copy(hotkeys); },
  },
  providers: {
    listTranslation: async () => copy(providers),
    activateTranslation: async id => { providers = providers.map(p => p.id === id ? { ...p, isActive: true } : p); },
    deactivateTranslation: async id => { providers = providers.map(p => p.id === id ? { ...p, isActive: false } : p); },
    reorderActiveTranslation: async ids => { providers.sort((a,b) => ids.indexOf(a.id) - ids.indexOf(b.id)); },
    getTranslationCredentialSchema: async () => [{ name: 'api_key', label: 'API 密钥', secret: true }],
    getOcrCredentialSchema: async () => [{ name: 'api_key', label: 'API 密钥', secret: true }],
    configureTranslationCredentials: async id => { providers = providers.map(p => p.id === id ? { ...p, isConfigured: true } : p); },
    addCustomTranslation: async request => { const provider = { ...providers[0], ...request, id: `preview-${Date.now()}`, isBuiltin: false }; providers.push(provider); return provider; },
    updateCustomTranslation: async (id, request) => { providers = providers.map(p => p.id === id ? { ...p, ...request } : p); return providers.find(p => p.id === id)!; },
    removeCustomTranslation: async id => { providers = providers.filter(p => p.id !== id); },
    testCustomTranslation: disconnected,
    listTranslationPromptStrategies: async () => ({ strategies: [] }), saveTranslationPromptStrategies: async config => config,
    listOpenAICompatibleModels: async () => [], testOpenAICompatible: disconnected, testOpenAIResponses: disconnected,
    listAnthropicModels: async () => [], testAnthropic: disconnected, listGeminiModels: async () => [], testGemini: disconnected,
    listOcr: async () => copy(ocrProviders),
    activateOcr: async id => { ocrProviders = ocrProviders.map(p => ({ ...p, isActive: p.id === id })); },
    configureOcrCredentials: noop,
  },
  libraryIndex: { queryHistoryIndex: emptyPage, queryFavoriteIndex: emptyPage },
  history: { getTranslationHistory: async () => [], getOcrHistory: async () => [], queryTranslationHistory: emptyPage, queryOcrHistory: emptyPage, deleteHistory: noop, updateHistoryNote: noop, replaceHistoryTags: noop, clearAllHistory: noop, clearHistory: noop, rerunOcrHistory: async () => '' },
  favorites: { addTranslationFavorite: async () => 1, addOcrFavorite: async () => 1, queryFavorites: emptyPage, updateFavoriteMetadata: noop, deleteFavorite: noop, rerunOcrFavorite: async () => '', listFavoriteTags: async () => [] },
  screenshotFavorites: { queryScreenshotFavorites: emptyPage, updateScreenshotFavoriteMetadata: noop, deleteScreenshotFavorite: noop, copyScreenshotFavorite: noop, revealScreenshotFavorite: noop },
  clipboard: { writeText: noop }, maintenance: { listAppLogs: async () => [], clearAppLogs: noop },
  tts: { listVoices: async () => [{ id: 'preview', name: '系统默认', locale: 'zh-CN' }], speak: noop },
});
initializeSettingsConfigStore(runtime.configuration.settings);
initializeHotkeyConfigStore(runtime.configuration.hotkeys);
initializeProviderStore(runtime.configuration.providers);
export const ready = Promise.all([runtime.configuration.settings.hydrate(), runtime.configuration.hotkeys.hydrate()]);
