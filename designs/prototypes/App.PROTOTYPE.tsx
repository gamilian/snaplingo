/**
 * PROTOTYPE - THROWAWAY CODE
 *
 * 主窗口 UI 原型 - 4 种不同的设计方案
 *
 * Question: 主窗口应该是什么样的布局和风格？
 *
 * Variations:
 * 1. Classic Sidebar: 传统侧边栏 + 内容区（当前实现）
 * 2. Top Tabs: 顶部标签页 + 全屏内容区
 * 3. Compact Floating: 紧凑浮动工具栏 + 卡片式布局
 * 4. Dashboard: 仪表盘式，所有功能一屏展示
 *
 * DELETE OR ABSORB WHEN DONE
 */

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "./stores/appStore";
import ResultWindow from "./components/ResultWindow";

type TabType = "translate" | "ocr" | "settings" | "plugins";
type Variant = "sidebar" | "toptabs" | "floating" | "dashboard";

function AppPrototype() {
  const [variant, setVariant] = useState<Variant>("sidebar");
  const [activeTab, setActiveTab] = useState<TabType>("translate");
  const [config, setConfig] = useState<any>(null);
  const { setSourceText, showResultWindow } = useAppStore();

  useEffect(() => {
    invoke("get_config")
      .then((data) => setConfig(data))
      .catch((err) => console.error("Failed to load config:", err));

    const unlisten = listen<string>("input-translation", (event) => {
      setSourceText(event.payload);
      showResultWindow();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setSourceText, showResultWindow]);

  if (!config) {
    return (
      <div className="h-screen flex items-center justify-center bg-[#f5f5f7]">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <>
      <ResultWindow />

      {/* Variant Switcher - Top Right Corner */}
      <div className="fixed top-4 right-4 bg-gray-900 text-white rounded-full px-4 py-2 shadow-2xl flex items-center space-x-3 z-50 text-xs">
        <span className="opacity-75 font-medium">原型:</span>
        <button onClick={() => setVariant('sidebar')} className={`px-2 py-1 rounded-full transition-colors ${variant === 'sidebar' ? 'bg-blue-500' : 'hover:bg-gray-700'}`}>
          侧边栏
        </button>
        <button onClick={() => setVariant('toptabs')} className={`px-2 py-1 rounded-full transition-colors ${variant === 'toptabs' ? 'bg-blue-500' : 'hover:bg-gray-700'}`}>
          顶部标签
        </button>
        <button onClick={() => setVariant('floating')} className={`px-2 py-1 rounded-full transition-colors ${variant === 'floating' ? 'bg-blue-500' : 'hover:bg-gray-700'}`}>
          浮动式
        </button>
        <button onClick={() => setVariant('dashboard')} className={`px-2 py-1 rounded-full transition-colors ${variant === 'dashboard' ? 'bg-blue-500' : 'hover:bg-gray-700'}`}>
          仪表盘
        </button>
      </div>

      {variant === 'sidebar' && <SidebarVariant activeTab={activeTab} setActiveTab={setActiveTab} config={config} showResultWindow={showResultWindow} />}
      {variant === 'toptabs' && <TopTabsVariant activeTab={activeTab} setActiveTab={setActiveTab} config={config} showResultWindow={showResultWindow} />}
      {variant === 'floating' && <FloatingVariant activeTab={activeTab} setActiveTab={setActiveTab} config={config} showResultWindow={showResultWindow} />}
      {variant === 'dashboard' && <DashboardVariant config={config} showResultWindow={showResultWindow} />}
    </>
  );
}

// VARIATION 1: Classic Sidebar (当前实现)
function SidebarVariant({ activeTab, setActiveTab, config, showResultWindow }: any) {
  return (
    <div className="flex h-screen bg-[#f5f5f7]">
      {/* 侧边栏 */}
      <div className="w-24 bg-gradient-to-b from-gray-50 to-gray-100 border-r border-gray-200 flex flex-col items-center py-8 space-y-6">
        <div className="w-12 h-12 bg-gradient-to-br from-blue-500 to-blue-600 rounded-2xl flex items-center justify-center shadow-lg">
          <span className="text-2xl">🌐</span>
        </div>
        <div className="h-px w-12 bg-gray-300" />
        <nav className="flex-1 flex flex-col space-y-2">
          <NavButton icon="🌐" label="翻译" active={activeTab === "translate"} onClick={() => setActiveTab("translate")} />
          <NavButton icon="📷" label="OCR" active={activeTab === "ocr"} onClick={() => setActiveTab("ocr")} />
          <NavButton icon="⚙️" label="设置" active={activeTab === "settings"} onClick={() => setActiveTab("settings")} />
          <NavButton icon="🧩" label="插件" active={activeTab === "plugins"} onClick={() => setActiveTab("plugins")} />
        </nav>
      </div>

      {/* 内容区 */}
      <div className="flex-1 overflow-hidden">
        {activeTab === "translate" && <TranslateContent showResultWindow={showResultWindow} />}
        {activeTab === "ocr" && <OcrContent />}
        {activeTab === "settings" && <SettingsContent config={config} />}
        {activeTab === "plugins" && <PluginsContent />}
      </div>
    </div>
  );
}

// VARIATION 2: Top Tabs
function TopTabsVariant({ activeTab, setActiveTab, config, showResultWindow }: any) {
  return (
    <div className="flex flex-col h-screen bg-white">
      {/* 顶部栏 */}
      <div className="bg-white border-b border-gray-200 px-8 pt-6 pb-0">
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center space-x-3">
            <div className="w-10 h-10 bg-gradient-to-br from-blue-500 to-purple-500 rounded-xl flex items-center justify-center shadow-lg">
              <span className="text-xl">🌐</span>
            </div>
            <h1 className="text-2xl font-bold bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent">
              SnapLingo
            </h1>
          </div>
        </div>

        {/* 标签页 */}
        <div className="flex space-x-1">
          <TopTab icon="🌐" label="翻译" active={activeTab === "translate"} onClick={() => setActiveTab("translate")} />
          <TopTab icon="📷" label="OCR" active={activeTab === "ocr"} onClick={() => setActiveTab("ocr")} />
          <TopTab icon="⚙️" label="设置" active={activeTab === "settings"} onClick={() => setActiveTab("settings")} />
          <TopTab icon="🧩" label="插件" active={activeTab === "plugins"} onClick={() => setActiveTab("plugins")} />
        </div>
      </div>

      {/* 内容区 */}
      <div className="flex-1 overflow-y-auto bg-[#f5f5f7]">
        <div className="max-w-6xl mx-auto p-8">
          {activeTab === "translate" && <TranslateContent showResultWindow={showResultWindow} />}
          {activeTab === "ocr" && <OcrContent />}
          {activeTab === "settings" && <SettingsContent config={config} />}
          {activeTab === "plugins" && <PluginsContent />}
        </div>
      </div>
    </div>
  );
}

// VARIATION 3: Compact Floating
function FloatingVariant({ activeTab, setActiveTab, config, showResultWindow }: any) {
  return (
    <div className="h-screen bg-gradient-to-br from-blue-50 via-purple-50 to-pink-50 p-8">
      {/* 浮动导航栏 */}
      <div className="mb-6">
        <div className="bg-white/80 backdrop-blur-xl rounded-2xl shadow-xl border border-white/50 p-3 inline-flex items-center space-x-2">
          <div className="w-8 h-8 bg-gradient-to-br from-blue-500 to-purple-500 rounded-lg flex items-center justify-center">
            <span className="text-lg">🌐</span>
          </div>
          <div className="w-px h-6 bg-gray-300" />
          <FloatingTab icon="🌐" label="翻译" active={activeTab === "translate"} onClick={() => setActiveTab("translate")} />
          <FloatingTab icon="📷" label="OCR" active={activeTab === "ocr"} onClick={() => setActiveTab("ocr")} />
          <FloatingTab icon="⚙️" label="设置" active={activeTab === "settings"} onClick={() => setActiveTab("settings")} />
          <FloatingTab icon="🧩" label="插件" active={activeTab === "plugins"} onClick={() => setActiveTab("plugins")} />
        </div>
      </div>

      {/* 内容卡片 */}
      <div className="h-[calc(100vh-120px)] overflow-y-auto">
        {activeTab === "translate" && <TranslateContent showResultWindow={showResultWindow} />}
        {activeTab === "ocr" && <OcrContent />}
        {activeTab === "settings" && <SettingsContent config={config} />}
        {activeTab === "plugins" && <PluginsContent />}
      </div>
    </div>
  );
}

// VARIATION 4: Dashboard (所有功能一屏展示)
function DashboardVariant({ config, showResultWindow }: any) {
  return (
    <div className="h-screen bg-gradient-to-br from-gray-50 to-gray-100 overflow-y-auto">
      <div className="max-w-7xl mx-auto p-8 space-y-6">
        {/* 头部 */}
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className="w-14 h-14 bg-gradient-to-br from-blue-500 to-purple-500 rounded-2xl flex items-center justify-center shadow-lg">
              <span className="text-3xl">🌐</span>
            </div>
            <div>
              <h1 className="text-3xl font-bold bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent">
                SnapLingo
              </h1>
              <p className="text-sm text-gray-600">一站式截图、OCR 与翻译工具</p>
            </div>
          </div>
        </div>

        {/* 主操作区 */}
        <div className="grid grid-cols-3 gap-6">
          <DashboardCard
            icon="🌐"
            title="快速翻译"
            description="打开翻译窗口进行文本翻译"
            action={<button onClick={showResultWindow} className="w-full py-3 bg-gradient-to-r from-blue-600 to-blue-700 hover:from-blue-700 hover:to-blue-800 text-white font-medium rounded-xl transition-all">
              打开翻译窗口
            </button>}
          />
          <DashboardCard
            icon="📷"
            title="OCR 识别"
            description="从图片中提取文字"
            badge="开发中"
          />
          <DashboardCard
            icon="🧩"
            title="插件管理"
            description="扩展更多功能"
            badge="开发中"
          />
        </div>

        {/* 快捷操作 */}
        <div className="bg-white rounded-2xl shadow-lg border border-gray-200 p-6">
          <h2 className="text-lg font-semibold text-gray-800 mb-4">快捷操作</h2>
          <div className="grid grid-cols-4 gap-4">
            <QuickActionCard icon="📸" label="截图翻译" hotkey="⌥S" />
            <QuickActionCard icon="✏️" label="划词翻译" hotkey="⌥D" />
            <QuickActionCard icon="⌨️" label="输入翻译" hotkey="⌥W" onClick={showResultWindow} />
            <QuickActionCard icon="🔍" label="OCR" hotkey="⌥A" />
          </div>
        </div>

        {/* 设置预览 */}
        <div className="grid grid-cols-2 gap-6">
          <div className="bg-white rounded-2xl shadow-lg border border-gray-200 p-6">
            <h2 className="text-lg font-semibold text-gray-800 mb-4 flex items-center">
              <span className="text-xl mr-2">⚙️</span>
              通用设置
            </h2>
            <div className="space-y-3 text-sm">
              <SettingRow label="语言" value={config.general.language === "en" ? "English" : "中文"} />
              <SettingRow label="主题" value="跟随系统" />
              <SettingRow label="开机启动" value={config.general.start_on_boot ? "已启用" : "已禁用"} />
            </div>
          </div>

          <div className="bg-white rounded-2xl shadow-lg border border-gray-200 p-6">
            <h2 className="text-lg font-semibold text-gray-800 mb-4 flex items-center">
              <span className="text-xl mr-2">⌨️</span>
              全局快捷键
            </h2>
            <div className="space-y-2 text-sm">
              <HotkeyRow icon="📸" label="截图" hotkey="F1" />
              <HotkeyRow icon="🔍" label="OCR" hotkey="⌥A" />
              <HotkeyRow icon="🌐" label="OCR+翻译" hotkey="⌥S" />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// 共用组件
function NavButton({ icon, label, active, onClick }: any) {
  return (
    <button
      onClick={onClick}
      className={`flex flex-col items-center justify-center w-16 h-16 rounded-xl transition-all ${
        active ? "bg-white shadow-md text-blue-600" : "text-gray-500 hover:text-gray-700 hover:bg-white/50"
      }`}
    >
      <span className="text-2xl mb-1">{icon}</span>
      <span className="text-[10px] font-medium">{label}</span>
    </button>
  );
}

function TopTab({ icon, label, active, onClick }: any) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center space-x-2 px-6 py-3 rounded-t-lg transition-all ${
        active
          ? "bg-[#f5f5f7] text-blue-600 font-medium"
          : "text-gray-600 hover:text-gray-900 hover:bg-gray-50"
      }`}
    >
      <span className="text-lg">{icon}</span>
      <span className="text-sm">{label}</span>
    </button>
  );
}

function FloatingTab({ icon, label, active, onClick }: any) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center space-x-1.5 px-4 py-2 rounded-lg transition-all text-sm ${
        active
          ? "bg-gradient-to-r from-blue-500 to-purple-500 text-white shadow-lg"
          : "text-gray-700 hover:bg-gray-100"
      }`}
    >
      <span>{icon}</span>
      <span className="font-medium">{label}</span>
    </button>
  );
}

function DashboardCard({ icon, title, description, action, badge }: any) {
  return (
    <div className="bg-white rounded-2xl shadow-lg border border-gray-200 p-6 space-y-4">
      <div className="flex items-start justify-between">
        <div className="w-12 h-12 bg-gradient-to-br from-blue-100 to-purple-100 rounded-xl flex items-center justify-center">
          <span className="text-2xl">{icon}</span>
        </div>
        {badge && (
          <span className="px-2 py-1 bg-blue-100 text-blue-700 text-xs rounded-full font-medium">
            {badge}
          </span>
        )}
      </div>
      <div>
        <h3 className="font-semibold text-gray-800 mb-1">{title}</h3>
        <p className="text-sm text-gray-600">{description}</p>
      </div>
      {action && <div>{action}</div>}
    </div>
  );
}

function QuickActionCard({ icon, label, hotkey, onClick }: any) {
  return (
    <button
      onClick={onClick}
      className="flex flex-col items-center justify-center p-4 bg-gray-50 hover:bg-gray-100 rounded-xl transition-all group"
    >
      <span className="text-2xl mb-2 group-hover:scale-110 transition-transform">{icon}</span>
      <span className="text-sm font-medium text-gray-700 mb-1">{label}</span>
      <kbd className="text-xs text-gray-500 px-2 py-0.5 bg-white rounded border border-gray-200">
        {hotkey}
      </kbd>
    </button>
  );
}

function SettingRow({ label, value }: any) {
  return (
    <div className="flex justify-between items-center py-1.5">
      <span className="text-gray-700">{label}</span>
      <span className="text-gray-500">{value}</span>
    </div>
  );
}

function HotkeyRow({ icon, label, hotkey }: any) {
  return (
    <div className="flex items-center justify-between py-1.5">
      <div className="flex items-center space-x-2">
        <span>{icon}</span>
        <span className="text-gray-700">{label}</span>
      </div>
      <kbd className="px-2 py-1 bg-gray-100 border border-gray-300 rounded text-xs font-mono">
        {hotkey}
      </kbd>
    </div>
  );
}

// 内容组件
function TranslateContent({ showResultWindow }: any) {
  return (
    <div className="h-full flex items-center justify-center p-12">
      <div className="max-w-3xl w-full space-y-8">
        <div className="text-center space-y-3">
          <h1 className="text-5xl font-bold bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent">
            SnapLingo
          </h1>
          <p className="text-lg text-gray-600">一站式截图、OCR 与翻译工具</p>
        </div>

        <div className="bg-white rounded-2xl shadow-xl border border-gray-100 overflow-hidden">
          <div className="p-8 space-y-6">
            <div className="flex items-center space-x-3">
              <div className="w-10 h-10 bg-blue-100 rounded-lg flex items-center justify-center">
                <span className="text-xl">🌐</span>
              </div>
              <h2 className="text-2xl font-semibold text-gray-800">快速翻译</h2>
            </div>

            <button
              onClick={showResultWindow}
              className="w-full py-5 bg-gradient-to-r from-blue-600 to-blue-700 hover:from-blue-700 hover:to-blue-800 text-white text-lg font-medium rounded-xl transition-all transform hover:scale-[1.02] active:scale-[0.98] shadow-lg hover:shadow-xl"
            >
              打开翻译窗口
            </button>

            <div className="grid grid-cols-3 gap-3 pt-2">
              <QuickAction icon="📸" label="截图翻译" hotkey="⌥S" />
              <QuickAction icon="✏️" label="划词翻译" hotkey="⌥D" />
              <QuickAction icon="⌨️" label="输入翻译" hotkey="⌥W" onClick={showResultWindow} />
            </div>
          </div>

          <div className="bg-gradient-to-r from-blue-50 to-purple-50 px-8 py-6 border-t border-gray-100">
            <h3 className="text-sm font-semibold text-gray-700 mb-3">全局快捷键</h3>
            <div className="grid grid-cols-2 gap-3">
              <HotkeyItem icon="📸" label="截图" hotkey="F1" />
              <HotkeyItem icon="🔍" label="OCR" hotkey="⌥A" />
              <HotkeyItem icon="🌐" label="OCR+翻译" hotkey="⌥S" />
              <HotkeyItem icon="✨" label="划词翻译" hotkey="⌥D" />
            </div>
          </div>
        </div>

        <div className="grid grid-cols-3 gap-4">
          <FeatureCard icon="🚀" title="极速翻译" description="多引擎并行，结果秒出" />
          <FeatureCard icon="🎯" title="精准识别" description="支持多种 OCR 引擎" />
          <FeatureCard icon="🔒" title="隐私优先" description="本地处理，数据安全" />
        </div>
      </div>
    </div>
  );
}

function QuickAction({ icon, label, hotkey, onClick }: any) {
  return (
    <button
      onClick={onClick}
      className="flex flex-col items-center justify-center p-4 bg-gray-50 hover:bg-gray-100 rounded-xl transition-all group"
    >
      <span className="text-2xl mb-2 group-hover:scale-110 transition-transform">{icon}</span>
      <span className="text-sm font-medium text-gray-700">{label}</span>
      <kbd className="text-xs text-gray-500 mt-1 px-2 py-0.5 bg-white rounded border border-gray-200">
        {hotkey}
      </kbd>
    </button>
  );
}

function HotkeyItem({ icon, label, hotkey }: any) {
  return (
    <div className="flex items-center justify-between text-sm">
      <div className="flex items-center space-x-2">
        <span>{icon}</span>
        <span className="text-gray-700">{label}</span>
      </div>
      <kbd className="px-2 py-1 bg-white rounded text-xs font-mono border border-gray-200">
        {hotkey}
      </kbd>
    </div>
  );
}

function FeatureCard({ icon, title, description }: any) {
  return (
    <div className="bg-white rounded-xl p-5 border border-gray-100 hover:shadow-lg transition-shadow">
      <div className="text-3xl mb-3">{icon}</div>
      <h3 className="font-semibold text-gray-800 mb-1">{title}</h3>
      <p className="text-sm text-gray-600">{description}</p>
    </div>
  );
}

function OcrContent() {
  return (
    <div className="h-full flex items-center justify-center">
      <div className="text-center space-y-6 max-w-md">
        <div className="w-24 h-24 mx-auto bg-gradient-to-br from-purple-100 to-blue-100 rounded-3xl flex items-center justify-center">
          <span className="text-4xl">📷</span>
        </div>
        <div className="space-y-3">
          <h2 className="text-3xl font-bold text-gray-800">OCR 识别</h2>
          <p className="text-base text-gray-600 leading-relaxed">
            OCR 功能正在开发中
            <br />
            即将支持 Tesseract、PaddleOCR 等引擎
          </p>
        </div>
      </div>
    </div>
  );
}

function SettingsContent({ config }: any) {
  return (
    <div className="h-full overflow-y-auto p-12">
      <div className="max-w-4xl mx-auto space-y-8">
        <div>
          <h2 className="text-3xl font-bold text-gray-900 mb-2">通用设置</h2>
          <p className="text-base text-gray-600">应用的基础配置选项</p>
        </div>

        <div className="bg-white rounded-lg border border-gray-200 p-6">
          <h3 className="font-semibold text-gray-800 mb-4">界面</h3>
          <div className="space-y-3">
            <SettingRow label="语言" value={config.general.language === "en" ? "English" : "中文"} />
            <SettingRow label="主题" value="跟随系统" />
            <SettingRow label="开机启动" value={config.general.start_on_boot ? "已启用" : "已禁用"} />
          </div>
        </div>

        <div className="bg-white rounded-lg border border-gray-200 p-6">
          <h3 className="font-semibold text-gray-800 mb-4">截图</h3>
          <div className="space-y-3">
            <SettingRow label="默认保存路径" value={config.screenshot.default_save_path} />
            <SettingRow label="图片格式" value={config.screenshot.format.toUpperCase()} />
            <SettingRow label="图片质量" value={`${config.screenshot.quality}%`} />
          </div>
        </div>
      </div>
    </div>
  );
}

function PluginsContent() {
  return (
    <div className="h-full flex items-center justify-center">
      <div className="text-center space-y-6 max-w-md">
        <div className="w-24 h-24 mx-auto bg-gradient-to-br from-green-100 to-blue-100 rounded-3xl flex items-center justify-center">
          <span className="text-4xl">🧩</span>
        </div>
        <div className="space-y-3">
          <h2 className="text-3xl font-bold text-gray-800">插件</h2>
          <p className="text-base text-gray-600">插件功能正在开发中...</p>
        </div>
      </div>
    </div>
  );
}

export default AppPrototype;
