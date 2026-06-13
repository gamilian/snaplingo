/**
 * PROTOTYPE - THROWAWAY CODE
 *
 * 严格按照 UI_DESIGN.md 实现的主窗口设计
 *
 * 这个原型完全遵循文档中的：
 * - 精确的尺寸规格（90px 侧边栏）
 * - 配色方案（蓝色渐变 #3B82F6 → #2563EB）
 * - 组件样式（圆角、阴影、间距）
 * - 设置页面的双栏布局
 *
 * DELETE OR ABSORB WHEN DONE
 */

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "./stores/appStore";
import ResultWindow from "./components/ResultWindow";

type TabType = "translate" | "ocr" | "settings" | "plugins";
type SettingsCategory = "general" | "translation" | "ocr" | "hotkeys" | "history";

function AppUIDoc() {
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
      <div className="h-screen flex items-center justify-center bg-[#F5F5F7]">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-[#3B82F6]" />
      </div>
    );
  }

  return (
    <>
      <ResultWindow />
      <div className="flex h-screen bg-[#F5F5F7]">
        {/* 左侧导航栏 - 精确按照文档：90px 宽 */}
        <div className="w-[90px] bg-gradient-to-b from-gray-50 to-gray-100 border-r border-gray-200 flex flex-col items-center py-8 space-y-6 flex-shrink-0">
          {/* Logo 区域 - 48x48，蓝色渐变圆角矩形 */}
          <div className="w-12 h-12 bg-gradient-to-br from-[#3B82F6] to-[#2563EB] rounded-xl flex items-center justify-center shadow-lg">
            <svg className="w-7 h-7 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129" />
            </svg>
          </div>

          <div className="h-px w-12 bg-gray-300" />

          {/* 导航按钮 - 64x64，text-2xl emoji，text-[10px] 标签 */}
          <nav className="flex-1 flex flex-col space-y-2">
            <NavButton icon="🌐" label="翻译" active={activeTab === "translate"} onClick={() => setActiveTab("translate")} />
            <NavButton icon="📷" label="OCR" active={activeTab === "ocr"} onClick={() => setActiveTab("ocr")} />
            <NavButton icon="⚙️" label="设置" active={activeTab === "settings"} onClick={() => setActiveTab("settings")} />
            <NavButton icon="🧩" label="插件" active={activeTab === "plugins"} onClick={() => setActiveTab("plugins")} />
          </nav>
        </div>

        {/* 右侧内容区 */}
        <div className="flex-1 overflow-hidden">
          {activeTab === "translate" && <TranslatePage showResultWindow={showResultWindow} />}
          {activeTab === "ocr" && <OcrPage />}
          {activeTab === "settings" && <SettingsPage config={config} />}
          {activeTab === "plugins" && <PluginsPage />}
        </div>
      </div>
    </>
  );
}

// 导航按钮 - 按文档规格
function NavButton({ icon, label, active, onClick }: any) {
  return (
    <button
      onClick={onClick}
      className={`group relative flex flex-col items-center justify-center w-16 h-16 rounded-xl transition-all ${
        active
          ? "bg-white shadow-md text-[#3B82F6]"
          : "text-gray-500 hover:text-gray-700 hover:bg-white/50"
      }`}
    >
      <span className="text-2xl mb-1">{icon}</span>
      <span className="text-[10px] font-medium">{label}</span>
    </button>
  );
}

// 翻译页面 - 完全按照文档 3.1 节
function TranslatePage({ showResultWindow }: any) {
  return (
    <div className="h-full overflow-y-auto p-12">
      <div className="max-w-3xl mx-auto space-y-8">
        {/* 标题区域 */}
        <div className="text-center space-y-3">
          <h1 className="text-5xl font-bold bg-gradient-to-r from-[#3B82F6] to-[#8B5CF6] bg-clip-text text-transparent">
            SnapLingo
          </h1>
          <p className="text-lg text-[#6B7280]">一站式截图、OCR 与翻译工具</p>
        </div>

        {/* 主操作卡片 */}
        <div className="bg-white rounded-xl shadow-xl border border-gray-100 overflow-hidden">
          <div className="p-8 space-y-6">
            {/* 图标 + 标题 */}
            <div className="flex items-center space-x-3">
              <div className="w-10 h-10 bg-[#DBEAFE] rounded-lg flex items-center justify-center">
                <span className="text-xl">🌐</span>
              </div>
              <h2 className="text-2xl font-semibold text-[#1F2937]">快速翻译</h2>
            </div>

            {/* 打开翻译窗口按钮 */}
            <button
              onClick={showResultWindow}
              className="w-full py-5 bg-gradient-to-r from-[#3B82F6] to-[#2563EB] hover:from-[#2563EB] hover:to-[#1D4ED8] text-white text-lg font-medium rounded-xl transition-all transform hover:scale-[1.02] active:scale-[0.98] shadow-lg"
            >
              打开翻译窗口
            </button>

            {/* 3 个快速操作按钮网格 */}
            <div className="grid grid-cols-3 gap-3 pt-2">
              <QuickActionButton icon="📸" label="截图翻译" hotkey="⌥S" />
              <QuickActionButton icon="✏️" label="划词翻译" hotkey="⌥D" />
              <QuickActionButton icon="⌨️" label="输入翻译" hotkey="⌥W" onClick={showResultWindow} />
            </div>
          </div>

          {/* 底部区域 - 蓝紫渐变背景 */}
          <div className="bg-gradient-to-r from-[#DBEAFE] to-[#EDE9FE] px-8 py-6 border-t border-gray-100">
            <h3 className="text-sm font-semibold text-[#374151] mb-3">全局快捷键</h3>
            {/* 2 列快捷键展示 */}
            <div className="grid grid-cols-2 gap-3">
              <HotkeyDisplay icon="📸" label="截图" hotkey="F1" />
              <HotkeyDisplay icon="🔍" label="OCR" hotkey="⌥A" />
              <HotkeyDisplay icon="🌐" label="OCR+翻译" hotkey="⌥S" />
              <HotkeyDisplay icon="✨" label="划词翻译" hotkey="⌥D" />
            </div>
          </div>
        </div>

        {/* 功能特性卡片 - 3 列网格 */}
        <div className="grid grid-cols-3 gap-4">
          <FeatureCard icon="🚀" title="极速翻译" description="多引擎并行，结果秒出" />
          <FeatureCard icon="🎯" title="精准识别" description="支持多种 OCR 引擎" />
          <FeatureCard icon="🔒" title="隐私优先" description="本地处理，数据安全" />
        </div>
      </div>
    </div>
  );
}

// 快速操作按钮
function QuickActionButton({ icon, label, hotkey, onClick }: any) {
  return (
    <button
      onClick={onClick}
      className="flex flex-col items-center justify-center p-4 bg-gray-50 hover:bg-gray-100 rounded-xl transition-all group"
    >
      <span className="text-2xl mb-2 group-hover:scale-110 transition-transform">{icon}</span>
      <span className="text-sm font-medium text-[#374151]">{label}</span>
      <kbd className="text-xs text-[#6B7280] mt-1 px-2 py-0.5 bg-white rounded border border-gray-200">
        {hotkey}
      </kbd>
    </button>
  );
}

// 快捷键显示
function HotkeyDisplay({ icon, label, hotkey }: any) {
  return (
    <div className="flex items-center justify-between text-sm">
      <div className="flex items-center space-x-2">
        <span>{icon}</span>
        <span className="text-[#374151]">{label}</span>
      </div>
      <kbd className="px-2 py-1 bg-white rounded text-xs font-mono border border-gray-200">
        {hotkey}
      </kbd>
    </div>
  );
}

// 功能特性卡片
function FeatureCard({ icon, title, description }: any) {
  return (
    <div className="bg-white rounded-xl p-5 border border-gray-100 hover:shadow-lg transition-shadow">
      <div className="text-3xl mb-3">{icon}</div>
      <h3 className="font-semibold text-[#1F2937] mb-1 text-base">{title}</h3>
      <p className="text-sm text-[#6B7280]">{description}</p>
    </div>
  );
}

// OCR 页面 - 按文档 3.2 节
function OcrPage() {
  return (
    <div className="h-full flex items-center justify-center p-12">
      <div className="text-center space-y-6 max-w-md">
        <div className="w-24 h-24 mx-auto bg-gradient-to-br from-purple-100 to-blue-100 rounded-3xl flex items-center justify-center">
          <span className="text-4xl">📷</span>
        </div>
        <div className="space-y-3">
          <h2 className="text-3xl font-bold text-[#1F2937]">OCR 识别</h2>
          <p className="text-base text-[#6B7280] leading-relaxed">
            OCR 功能正在开发中
            <br />
            即将支持 Tesseract、PaddleOCR 等引擎
          </p>
        </div>
      </div>
    </div>
  );
}

// 设置页面 - 按文档 3.3 节：双栏布局
function SettingsPage({ config }: any) {
  const [selectedCategory, setSelectedCategory] = useState<SettingsCategory>("general");

  return (
    <div className="h-full flex bg-white">
      {/* 左侧二级导航 - 224px 宽 */}
      <div className="w-56 bg-gray-50 border-r border-gray-200 p-6 flex-shrink-0">
        <h2 className="text-lg font-bold text-[#1F2937] mb-6 px-2">设置</h2>
        <nav className="space-y-1">
          <CategoryButton label="通用设置" active={selectedCategory === "general"} onClick={() => setSelectedCategory("general")} />
          <CategoryButton label="翻译服务" active={selectedCategory === "translation"} onClick={() => setSelectedCategory("translation")} />
          <CategoryButton label="OCR 服务" active={selectedCategory === "ocr"} onClick={() => setSelectedCategory("ocr")} />
          <CategoryButton label="快捷键" active={selectedCategory === "hotkeys"} onClick={() => setSelectedCategory("hotkeys")} />
          <CategoryButton label="历史记录" active={selectedCategory === "history"} onClick={() => setSelectedCategory("history")} />
        </nav>
      </div>

      {/* 右侧内容区 */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-4xl mx-auto p-12">
          {selectedCategory === "general" && <GeneralSettings config={config} />}
          {selectedCategory === "translation" && <TranslationSettings config={config} />}
          {selectedCategory === "ocr" && <OcrSettings config={config} />}
          {selectedCategory === "hotkeys" && <HotkeySettings config={config} />}
          {selectedCategory === "history" && <HistorySettings config={config} />}
        </div>
      </div>
    </div>
  );
}

// 分类按钮
function CategoryButton({ label, active, onClick }: any) {
  return (
    <button
      onClick={onClick}
      className={`w-full text-left px-4 py-2 rounded-lg transition-colors text-sm ${
        active
          ? "bg-[#3B82F6] text-white shadow-md font-medium"
          : "text-[#374151] hover:bg-gray-100"
      }`}
    >
      {label}
    </button>
  );
}

// 通用设置
function GeneralSettings({ config }: any) {
  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-[#111827] mb-2">通用设置</h2>
        <p className="text-base text-[#6B7280]">应用的基础配置选项</p>
      </div>

      <SettingGroup title="界面">
        <SettingRow label="语言" value={config.general.language === "en" ? "English" : "中文"} />
        <SettingRow label="主题" value={config.general.theme === "system" ? "跟随系统" : config.general.theme} />
        <SettingRow label="开机启动" value={config.general.start_on_boot ? "已启用" : "已禁用"} />
      </SettingGroup>

      <SettingGroup title="截图">
        <SettingRow label="默认保存路径" value={config.screenshot.default_save_path} />
        <SettingRow label="图片格式" value={config.screenshot.format.toUpperCase()} />
        <SettingRow label="图片质量" value={`${config.screenshot.quality}%`} />
      </SettingGroup>
    </div>
  );
}

// 翻译服务设置
function TranslationSettings({ config }: any) {
  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-[#111827] mb-2">翻译服务</h2>
        <p className="text-base text-[#6B7280]">配置翻译服务提供商</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-4">
        <h3 className="font-semibold text-[#1F2937]">已启用的服务</h3>
        {config.translation.active_providers.map((provider: string) => (
          <ProviderCard
            key={provider}
            name={provider === "google-translate" ? "Google 翻译" : provider}
            status="已启用"
            description="免费服务，无需 API 密钥"
            icon="G"
          />
        ))}
        <div className="pt-4 border-t border-gray-100">
          <button className="px-4 py-2 text-sm text-[#3B82F6] hover:bg-blue-50 rounded-lg transition-colors">
            + 添加翻译服务
          </button>
        </div>
      </div>
    </div>
  );
}

// OCR 服务设置
function OcrSettings({ config }: any) {
  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-[#111827] mb-2">OCR 服务</h2>
        <p className="text-base text-[#6B7280]">配置 OCR 识别引擎</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <h3 className="font-semibold text-[#1F2937] mb-4">当前引擎</h3>
        <ProviderCard
          name={config.ocr.active_provider}
          status="已选择"
          description="本地 OCR 引擎"
          icon="T"
        />
      </div>
    </div>
  );
}

// 快捷键设置
function HotkeySettings({ config }: any) {
  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-[#111827] mb-2">快捷键设置</h2>
        <p className="text-base text-[#6B7280]">自定义全局快捷键</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 divide-y divide-gray-100">
        <HotkeyRow label="截图" value={config.hotkeys.screenshot} />
        <HotkeyRow label="OCR" value={config.hotkeys.ocr} />
        <HotkeyRow label="OCR + 翻译" value={config.hotkeys.ocr_translate} />
        <HotkeyRow label="划词翻译" value={config.hotkeys.selection_translate} />
        <HotkeyRow label="输入翻译" value={config.hotkeys.input_translate} />
      </div>
    </div>
  );
}

// 历史记录设置
function HistorySettings({ config }: any) {
  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-[#111827] mb-2">历史记录</h2>
        <p className="text-base text-[#6B7280]">管理历史记录和自动清理</p>
      </div>

      <SettingGroup title="记录选项">
        <SettingRow label="记录截图" value={config.history.record_screenshot ? "已启用" : "已禁用"} />
        <SettingRow label="记录 OCR" value={config.history.record_ocr ? "已启用" : "已禁用"} />
        <SettingRow label="记录翻译" value={config.history.record_translation ? "已启用" : "已禁用"} />
      </SettingGroup>

      <SettingGroup title="自动清理">
        <SettingRow label="自动清理" value={config.history.auto_cleanup_enabled ? "已启用" : "已禁用"} />
        <SettingRow label="最大保存天数" value={`${config.history.max_age_days} 天`} />
        <SettingRow label="最大条目数" value={`${config.history.max_entries} 条`} />
      </SettingGroup>
    </div>
  );
}

// 插件页面 - 按文档 3.4 节
function PluginsPage() {
  return (
    <div className="h-full flex items-center justify-center p-12">
      <div className="text-center space-y-6 max-w-md">
        <div className="w-24 h-24 mx-auto bg-gradient-to-br from-green-100 to-blue-100 rounded-3xl flex items-center justify-center">
          <span className="text-4xl">🧩</span>
        </div>
        <div className="space-y-3">
          <h2 className="text-3xl font-bold text-[#1F2937]">插件</h2>
          <p className="text-base text-[#6B7280]">插件功能正在开发中...</p>
        </div>
      </div>
    </div>
  );
}

// 设置组
function SettingGroup({ title, children }: any) {
  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6">
      <h3 className="font-semibold text-[#1F2937] mb-4">{title}</h3>
      <div className="space-y-3">{children}</div>
    </div>
  );
}

// 设置行
function SettingRow({ label, value }: any) {
  return (
    <div className="flex justify-between items-center py-2">
      <span className="text-[#374151] text-base">{label}</span>
      <span className="text-[#6B7280] text-sm">{value}</span>
    </div>
  );
}

// 服务提供商卡片
function ProviderCard({ name, status, description, icon }: any) {
  return (
    <div className="flex items-start space-x-4 p-4 bg-gray-50 rounded-lg">
      <div className="w-10 h-10 bg-[#DBEAFE] rounded-lg flex items-center justify-center flex-shrink-0">
        <span className="text-[#3B82F6] font-semibold">{icon}</span>
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center space-x-2">
          <h4 className="font-medium text-[#1F2937] text-base">{name}</h4>
          <span className="px-2 py-0.5 text-xs bg-[#D1FAE5] text-[#065F46] rounded">
            {status}
          </span>
        </div>
        <p className="text-sm text-[#6B7280] mt-1">{description}</p>
      </div>
    </div>
  );
}

// 快捷键行
function HotkeyRow({ label, value }: any) {
  return (
    <div className="flex justify-between items-center p-4">
      <span className="text-[#374151] text-base">{label}</span>
      <kbd className="px-3 py-1.5 bg-gray-100 border border-gray-300 rounded text-sm font-mono">
        {value}
      </kbd>
    </div>
  );
}

export default AppUIDoc;
