// Render the existing views with isolated data for native-page visual comparison.
// This fixture does not initialize Tauri or change the installed application's settings.
import { createRoot } from 'react-dom/client';
import ThirdPartyNotices from '../views/SettingsWindow/General/ThirdPartyNotices';
import { GeneralPage } from '../views/SettingsWindow/General/GeneralPage';
import { MainNav } from '../views/SettingsWindow/Navigation/MainNav';
import { SettingsRuntimeProvider } from '../views/SettingsWindow/runtimeContext';
import { useSettingsConfigStore } from '../stores/settingsConfigStore';
import type { SettingsRuntime } from '../application/settings/runtime';
import { OcrSettingsScrollPage } from '../views/SettingsWindow/OCR/OcrSettingsScrollPage';
import { TranslationSettingsScrollPage } from '../views/SettingsWindow/Translation/TranslationSettingsScrollPage';
import { ScreenshotSettingsPage } from '../views/SettingsWindow/Screenshot/ScreenshotSettingsPage';
import { initializeFeatureReference } from './settings_feature_fixture';
import { initializeLibraryReference } from './settings_library_fixture';
import { FavoritesLibraryPage } from '../views/SettingsWindow/Library/FavoritesLibraryPage';
import { HistoryLibraryPage } from '../views/SettingsWindow/Library/HistoryLibraryPage';
import { SettingsWindow } from '../views/SettingsWindow';
import { initializeServicesReference } from './settings_services_fixture';
import '../styles/index.css';

const parameters = new URLSearchParams(location.search);
const theme = parameters.get('theme') ?? 'native';
const appearance = parameters.get('appearance') ?? 'light';
const page = parameters.get('page')==='services'?'services':parameters.get('page')==='favorites'?'favorites':parameters.get('page')==='history'?'history':parameters.get('page')==='screenshot'?'screenshot':parameters.get('page') === 'ocr' ? 'ocr' : parameters.get('page') === 'translation' ? 'translation' : 'general';
document.documentElement.dataset.referencePage = page;
document.documentElement.dataset.interfaceTheme = theme;
document.documentElement.dataset.theme = appearance;
const general = {
  language: 'zh-CN', theme: appearance, interfaceTheme: theme, startOnBoot: true,
  proxyMode: 'manual', proxyUrl: 'http://127.0.0.1:7890', requestTimeoutMs: 25000, retryCount: 4,
  logLevel: 'debug', logRetentionDays: 20, performanceMonitoring: true, experimentalGpuAcceleration: true,
};
useSettingsConfigStore.setState({
  hydrated: true, general: general as never,
  history: { autoCleanupEnabled: false, retentionDays: 30, maximumRecords: 5000, maximumFavorites: 1000 },
});
if (['ocr','translation','screenshot'].includes(page)) initializeFeatureReference();
const scenario = parameters.get('scenario');
document.documentElement.dataset.scenario = scenario ?? 'baseline';
const permissionSnapshot = { status: { screenRecording: scenario !== 'recovery', accessibility: scenario !== 'recovery' }, error: null };
const updateSnapshot = scenario === 'update' ? { status: { currentVersion: '0.1.23', latestVersion: '0.1.24', available: true, downloadSize: 2_100_000, notes: '新版本更新说明\n• 修复截图窗口与滚动截图交互\n• 改善识别文本和翻译体验', publishedAt: null, platform: 'macos' }, phase: 'idle', error: null, downloadedPath: '/tmp/SnapLingo fixture.dmg' } : { status: null, phase: 'idle', error: null, downloadedPath: null };
const runtime = {
  window: { selectScreenshotDirectory:async()=>'/tmp/SnapLingo screenshots',version: async () => '0.1.23', openExternal: async () => {} },
  maintenance: { listAppLogs: async () => scenario === 'logs' ? Array.from({ length: 8 }, (_, index) => ({id:index, timestamp:'2026-11-07T00:00:00Z', level:'INFO', target:'native-settings', message:`Fixture log entry ${index}: native preferences state update`})) : [], clearAppLogs: async () => {} },
  permissions: {
    subscribe: (receive: (value: typeof permissionSnapshot) => void) => { receive(permissionSnapshot); return () => {}; },
    refresh: async () => permissionSnapshot,
    context: async () => ({ platform: 'macos', appPath: scenario === 'recovery' ? '/Applications/SnapLingo.app' : null, needsInstallation: false, canReset: true }),
  },
  updates: {
    subscribe: (receive: (value: unknown) => void) => { receive(updateSnapshot); return () => {}; },
    ensureChecked: async () => {},
  },
} as unknown as SettingsRuntime;
if(page==='favorites'||page==='history') Object.assign(runtime,initializeLibraryReference());
if(page==='services') initializeServicesReference(runtime);
createRoot(document.getElementById('root')!).render(
  page==='services' ? <SettingsWindow runtime={runtime} /> : <SettingsRuntimeProvider runtime={runtime}>
    {new URLSearchParams(location.search).has('notices') && <ThirdPartyNotices onClose={() => {}} />}
    <div className="settings-shell flex bg-canvas" style={{ width: '100vw', height: '100vh', containerType: 'inline-size' }}>
      <MainNav activeTab={page} onTabChange={() => {}} />
      <div className="settings-content flex-1 min-h-0 min-w-0 overflow-hidden flex">{page==='favorites'?<FavoritesLibraryPage />:page==='history'?<HistoryLibraryPage />:page==='screenshot'?<ScreenshotSettingsPage />:page === 'ocr' ? <OcrSettingsScrollPage /> : page === 'translation' ? <TranslationSettingsScrollPage /> : <GeneralPage />}</div>
    </div>
  </SettingsRuntimeProvider>,
);
