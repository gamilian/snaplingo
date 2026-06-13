/**
 * UI PROTOTYPE - 主窗口结构原型
 *
 * Question: 6标签页（截图/翻译/OCR/服务/通用/高级）+ 二级导航的结构是否合适？
 *
 * 如何运行：替换 src/main.tsx 中的 import App，或直接访问这个组件
 *
 * THROWAWAY CODE - 验证完成后删除或合并到 App.tsx
 */

import { useState } from "react";
import React from "react";

type MainTab = "screenshot" | "translation" | "ocr" | "services" | "general" | "advanced";
type ScreenshotSubTab = "hotkeys" | "save-settings" | "editor" | "favorites";
type TranslationSubTab = "hotkeys" | "settings" | "history" | "favorites";
type OcrSubTab = "hotkeys" | "settings" | "history" | "favorites";
type ServicesTab = "ocr" | "translation" | "tts";
type GeneralSubTab = "interface" | "app-hotkeys" | "about";
type AdvancedSubTab = "network" | "logs" | "data";

// ============ SVG 图标组件（增强识别度版本）============
// 保持 Anthropic 风格，但增加特征元素让图标更有个性

// ============ SVG 图标组件 ============

function ScreenshotIcon() {
  // 截图：相机
  return (
    <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z" />
      <circle cx="12" cy="13" r="4" />
    </svg>
  );
}

function TranslationIcon() {
  // 翻译：地球仪
  return (
    <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
    </svg>
  );
}

function OcrIcon() {
  // OCR：扫描框 + 文本
  return (
    <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 7V5a2 2 0 0 1 2-2h2M17 3h2a2 2 0 0 1 2 2v2M21 17v2a2 2 0 0 1-2 2h-2M7 21H5a2 2 0 0 1-2-2v-2" />
      <path d="M8 12h8M8 16h5" />
    </svg>
  );
}

function ServicesIcon() {
  // 服务：堆叠卡片
  return (
    <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="8" y="2" width="8" height="4" rx="1" />
      <rect x="6" y="8" width="12" height="8" rx="1" />
      <rect x="4" y="18" width="16" height="4" rx="1" />
    </svg>
  );
}

function SettingsIcon() {
  // 通用：滑动开关
  return (
    <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 12h16M4 6h16M4 18h16" />
      <circle cx="8" cy="6" r="2" />
      <circle cx="16" cy="12" r="2" />
      <circle cx="12" cy="18" r="2" />
    </svg>
  );
}

function AdvancedIcon() {
  // 高级：放射控制
  return (
    <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="3" />
      <path d="M12 2v4m0 12v4M4.93 4.93l2.83 2.83m8.48 8.48l2.83 2.83M2 12h4m12 0h4M4.93 19.07l2.83-2.83m8.48-8.48l2.83-2.83" />
    </svg>
  );
}

function AppPrototypeUI() {
  const [activeMainTab, setActiveMainTab] = useState<MainTab>("screenshot");
  const [screenshotSubTab, setScreenshotSubTab] = useState<ScreenshotSubTab>("hotkeys");
  const [translationSubTab, setTranslationSubTab] = useState<TranslationSubTab>("hotkeys");
  const [ocrSubTab, setOcrSubTab] = useState<OcrSubTab>("hotkeys");
  const [servicesTab, setServicesTab] = useState<ServicesTab>("ocr");
  const [generalSubTab, setGeneralSubTab] = useState<GeneralSubTab>("interface");
  const [advancedSubTab, setAdvancedSubTab] = useState<AdvancedSubTab>("network");

  return (
    <div className="flex h-screen bg-[#f5f5f7]">
      {/* 左侧主导航 */}
      <div className="w-24 bg-gradient-to-b from-gray-50 to-gray-100 border-r border-gray-200 flex flex-col items-center py-8 space-y-6">
        {/* Logo */}
        <div className="w-12 h-12 bg-gradient-to-br from-blue-500 to-blue-600 rounded-2xl flex items-center justify-center shadow-lg">
          <svg className="w-7 h-7 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129" />
          </svg>
        </div>

        <div className="h-px w-12 bg-gray-300" />

        {/* 主导航按钮 */}
        <nav className="flex-1 flex flex-col space-y-2">
          <MainNavButton
            icon={<ScreenshotIcon />}
            label="截图"
            active={activeMainTab === "screenshot"}
            onClick={() => setActiveMainTab("screenshot")}
          />
          <MainNavButton
            icon={<TranslationIcon />}
            label="翻译"
            active={activeMainTab === "translation"}
            onClick={() => setActiveMainTab("translation")}
          />
          <MainNavButton
            icon={<OcrIcon />}
            label="OCR"
            active={activeMainTab === "ocr"}
            onClick={() => setActiveMainTab("ocr")}
          />
          <MainNavButton
            icon={<ServicesIcon />}
            label="服务"
            active={activeMainTab === "services"}
            onClick={() => setActiveMainTab("services")}
          />
          <MainNavButton
            icon={<SettingsIcon />}
            label="通用"
            active={activeMainTab === "general"}
            onClick={() => setActiveMainTab("general")}
          />
          <MainNavButton
            icon={<AdvancedIcon />}
            label="高级"
            active={activeMainTab === "advanced"}
            onClick={() => setActiveMainTab("advanced")}
          />
        </nav>
      </div>

      {/* 右侧内容区 */}
      <div className="flex-1 overflow-hidden">
        {activeMainTab === "screenshot" && <ScreenshotTab activeSubTab={screenshotSubTab} setActiveSubTab={setScreenshotSubTab} />}
        {activeMainTab === "translation" && <TranslationTab activeSubTab={translationSubTab} setActiveSubTab={setTranslationSubTab} />}
        {activeMainTab === "ocr" && <OcrTab activeSubTab={ocrSubTab} setActiveSubTab={setOcrSubTab} />}
        {activeMainTab === "services" && <ServicesTab activeTab={servicesTab} setActiveTab={setServicesTab} />}
        {activeMainTab === "general" && <GeneralTab activeSubTab={generalSubTab} setActiveSubTab={setGeneralSubTab} />}
        {activeMainTab === "advanced" && <AdvancedTab activeSubTab={advancedSubTab} setActiveSubTab={setAdvancedSubTab} />}
      </div>
    </div>
  );
}

function MainNavButton({ icon, label, active, onClick }: { icon: React.ReactNode; label: string; active: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className={`flex flex-col items-center justify-center w-16 h-16 rounded-xl transition-all ${
        active ? "bg-white shadow-md text-blue-600" : "text-gray-500 hover:text-gray-700 hover:bg-white/50"
      }`}
    >
      <div className="mb-1">{icon}</div>
      <span className="text-[10px] font-medium">{label}</span>
    </button>
  );
}

// ============ 截图 Tab ============
function ScreenshotTab({ activeSubTab, setActiveSubTab }: { activeSubTab: ScreenshotSubTab; setActiveSubTab: (tab: ScreenshotSubTab) => void }) {
  return (
    <div className="h-full flex bg-white">
      {/* 左侧二级导航 */}
      <div className="w-56 bg-gray-50 border-r border-gray-200 p-6">
        <h2 className="text-lg font-bold text-gray-800 mb-6 px-2">截图</h2>
        <nav className="space-y-1">
          <SubNavButton label="快捷键" active={activeSubTab === "hotkeys"} onClick={() => setActiveSubTab("hotkeys")} />
          <SubNavButton label="保存设置" active={activeSubTab === "save-settings"} onClick={() => setActiveSubTab("save-settings")} />
          <SubNavButton label="编辑器" active={activeSubTab === "editor"} onClick={() => setActiveSubTab("editor")} />
          <SubNavButton label="收藏夹" active={activeSubTab === "favorites"} onClick={() => setActiveSubTab("favorites")} />
        </nav>
      </div>

      {/* 右侧内容 */}
      <div className="flex-1 overflow-y-auto p-12">
        {activeSubTab === "hotkeys" && <ScreenshotHotkeysContent />}
        {activeSubTab === "save-settings" && <PlaceholderContent title="保存设置" description="配置截图保存路径、格式、质量等" />}
        {activeSubTab === "editor" && <PlaceholderContent title="编辑器" description="配置默认颜色、粗细、字体等" />}
        {activeSubTab === "favorites" && <PlaceholderContent title="收藏夹" description="管理收藏的截图" />}
      </div>
    </div>
  );
}

function ScreenshotHotkeysContent() {
  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">快捷键</h2>
        <p className="text-gray-600">配置截图相关的全局快捷键。点击快捷键框进行录制。</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 divide-y divide-gray-100">
        <HotkeyRow label="截屏" value="F1" />
        <HotkeyRow label="截屏并自动复制" value="⌘F1" />
        <HotkeyRow label="自定义截屏" value="⇧F1" />
        <HotkeyRow label="贴图" value="F3" />
        <HotkeyRow label="隐藏/显示所有贴图" value="⇧F3" />
        <HotkeyRow label="切换到另一贴图组" value="⌘F3" />
      </div>

      <div className="flex items-center justify-between pt-4">
        <button className="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded-lg transition-colors">
          恢复所有默认值
        </button>
        <button className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors">
          检测冲突
        </button>
      </div>
    </div>
  );
}

// ============ 翻译 Tab ============
function TranslationTab({ activeSubTab, setActiveSubTab }: { activeSubTab: TranslationSubTab; setActiveSubTab: (tab: TranslationSubTab) => void }) {
  return (
    <div className="h-full flex bg-white">
      <div className="w-56 bg-gray-50 border-r border-gray-200 p-6">
        <h2 className="text-lg font-bold text-gray-800 mb-6 px-2">翻译</h2>
        <nav className="space-y-1">
          <SubNavButton label="快捷键" active={activeSubTab === "hotkeys"} onClick={() => setActiveSubTab("hotkeys")} />
          <SubNavButton label="翻译设置" active={activeSubTab === "settings"} onClick={() => setActiveSubTab("settings")} />
          <SubNavButton label="历史记录" active={activeSubTab === "history"} onClick={() => setActiveSubTab("history")} />
          <SubNavButton label="收藏夹" active={activeSubTab === "favorites"} onClick={() => setActiveSubTab("favorites")} />
        </nav>
      </div>

      <div className="flex-1 overflow-y-auto p-12">
        {activeSubTab === "hotkeys" && <TranslationHotkeysContent />}
        {activeSubTab === "settings" && <PlaceholderContent title="翻译设置" description="配置源语言、目标语言、多服务显示顺序等" />}
        {activeSubTab === "history" && <PlaceholderContent title="历史记录" description="查看翻译历史，支持过滤和搜索" />}
        {activeSubTab === "favorites" && <PlaceholderContent title="收藏夹" description="管理收藏的翻译，支持标签和笔记" />}
      </div>
    </div>
  );
}

function TranslationHotkeysContent() {
  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">快捷键</h2>
        <p className="text-gray-600">配置翻译相关的全局快捷键。点击快捷键框进行录制。</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 divide-y divide-gray-100">
        <HotkeyRow label="划词翻译" value="⌥D" description="选中文字后触发翻译" />
        <HotkeyRow label="截图翻译" value="⌥S" description="截图区域 → OCR → 自动翻译" />
        <HotkeyRow label="输入翻译" value="⌥A" description="清空翻译窗口并显示，用于手动输入" />
        <HotkeyRow label="显示翻译窗口" value="未设置" description="直接显示翻译窗口，查看之前的结果" />
      </div>

      <div className="flex items-center justify-between pt-4">
        <button className="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded-lg transition-colors">
          恢复所有默认值
        </button>
        <button className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors">
          检测冲突
        </button>
      </div>
    </div>
  );
}

// ============ OCR Tab ============
function OcrTab({ activeSubTab, setActiveSubTab }: { activeSubTab: OcrSubTab; setActiveSubTab: (tab: OcrSubTab) => void }) {
  return (
    <div className="h-full flex bg-white">
      <div className="w-56 bg-gray-50 border-r border-gray-200 p-6">
        <h2 className="text-lg font-bold text-gray-800 mb-6 px-2">OCR</h2>
        <nav className="space-y-1">
          <SubNavButton label="快捷键" active={activeSubTab === "hotkeys"} onClick={() => setActiveSubTab("hotkeys")} />
          <SubNavButton label="OCR 设置" active={activeSubTab === "settings"} onClick={() => setActiveSubTab("settings")} />
          <SubNavButton label="历史记录" active={activeSubTab === "history"} onClick={() => setActiveSubTab("history")} />
          <SubNavButton label="收藏夹" active={activeSubTab === "favorites"} onClick={() => setActiveSubTab("favorites")} />
        </nav>
      </div>

      <div className="flex-1 overflow-y-auto p-12">
        {activeSubTab === "hotkeys" && <OcrHotkeysContent />}
        {activeSubTab === "settings" && <PlaceholderContent title="OCR 设置" description="配置识别语言、结果窗口位置等" />}
        {activeSubTab === "history" && <PlaceholderContent title="历史记录" description="查看 OCR 历史，带原图缩略图" />}
        {activeSubTab === "favorites" && <PlaceholderContent title="收藏夹" description="管理收藏的 OCR 结果" />}
      </div>
    </div>
  );
}

function OcrHotkeysContent() {
  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">快捷键</h2>
        <p className="text-gray-600">配置 OCR 相关的全局快捷键。点击快捷键框进行录制。</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 divide-y divide-gray-100">
        <HotkeyRow label="截图 OCR" value="⇧⌥S" description="截图区域 → 自动 OCR → 显示识别结果" />
        <HotkeyRow label="静默截图 OCR" value="未设置" description="后台识别，自动将结果拷贝到剪切板" />
        <HotkeyRow label="访问选图 OCR" value="未设置" description="通过文件选择器选择图片进行 OCR" />
        <HotkeyRow label="显示 OCR 窗口" value="未设置" description="直接显示 OCR 窗口" />
      </div>

      <div className="flex items-center justify-between pt-4">
        <button className="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded-lg transition-colors">
          恢复所有默认值
        </button>
        <button className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors">
          检测冲突
        </button>
      </div>
    </div>
  );
}

// ============ 服务 Tab ============
function ServicesTab({ activeTab, setActiveTab }: { activeTab: ServicesTab; setActiveTab: (tab: ServicesTab) => void }) {
  return (
    <div className="h-full flex flex-col bg-white">
      {/* 顶部标签 */}
      <div className="border-b border-gray-200 px-12 pt-8">
        <div className="flex space-x-8">
          <TopTabButton label="OCR 服务" active={activeTab === "ocr"} onClick={() => setActiveTab("ocr")} />
          <TopTabButton label="翻译服务" active={activeTab === "translation"} onClick={() => setActiveTab("translation")} />
          <TopTabButton label="语音合成" active={activeTab === "tts"} onClick={() => setActiveTab("tts")} />
        </div>
      </div>

      {/* 内容区 */}
      <div className="flex-1 overflow-y-auto p-12">
        {activeTab === "ocr" && <OcrServicesContent />}
        {activeTab === "translation" && <TranslationServicesContent />}
        {activeTab === "tts" && <TtsServicesContent />}
      </div>
    </div>
  );
}

function OcrServicesContent() {
  return (
    <div className="max-w-4xl space-y-6">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">OCR 服务</h2>
        <p className="text-gray-600 mb-4">当前激活：<span className="font-semibold text-blue-600">● Tesseract</span></p>
      </div>

      <ProviderCard name="Tesseract" status="已激活" description="免费本地 OCR 引擎" icon="T" active />
      <ProviderCard name="PaddleOCR" status="未激活" description="中文优化，免费本地引擎" icon="P" />
      <ProviderCard name="百度 OCR" status="未配置" description="需要 API Key" icon="百" />

      <button className="px-4 py-2 text-blue-600 hover:bg-blue-50 rounded-lg transition-colors">
        + 添加自定义服务
      </button>
    </div>
  );
}

function TranslationServicesContent() {
  return (
    <div className="max-w-4xl space-y-6">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">翻译服务</h2>
        <p className="text-gray-600 mb-4">已激活：<span className="font-semibold text-green-600">Google ✓ DeepL ✓</span></p>
        <p className="text-sm text-gray-500 mb-4">提示：可同时激活多个翻译服务，结果会并排显示。优先级从上到下，拖动卡片调整顺序。</p>
      </div>

      <ProviderCard name="Google 翻译" status="已激活" description="免费服务，无需 API Key" icon="G" active draggable />
      <ProviderCard name="DeepL" status="已激活" description="需要 API Key" icon="D" active draggable />
      <ProviderCard name="百度翻译" status="未配置" description="需要 API Key" icon="百" />

      <button className="px-4 py-2 text-blue-600 hover:bg-blue-50 rounded-lg transition-colors">
        + 添加自定义服务
      </button>
    </div>
  );
}

function TtsServicesContent() {
  return (
    <div className="max-w-4xl space-y-6">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">语音合成</h2>
        <p className="text-gray-600 mb-4">当前激活：<span className="font-semibold text-blue-600">● 系统 TTS</span></p>
      </div>

      <ProviderCard name="系统 TTS" status="已激活" description="macOS say / Windows SAPI" icon="🔊" active />
      <ProviderCard name="Azure TTS" status="未配置" description="需要 API Key，支持多语言自然语音" icon="A" />

      <button className="px-4 py-2 text-blue-600 hover:bg-blue-50 rounded-lg transition-colors">
        + 添加自定义服务
      </button>
    </div>
  );
}

// ============ 通用 Tab ============
function GeneralTab({ activeSubTab, setActiveSubTab }: { activeSubTab: GeneralSubTab; setActiveSubTab: (tab: GeneralSubTab) => void }) {
  return (
    <div className="h-full flex bg-white">
      <div className="w-56 bg-gray-50 border-r border-gray-200 p-6">
        <h2 className="text-lg font-bold text-gray-800 mb-6 px-2">通用</h2>
        <nav className="space-y-1">
          <SubNavButton label="界面" active={activeSubTab === "interface"} onClick={() => setActiveSubTab("interface")} />
          <SubNavButton label="应用快捷键" active={activeSubTab === "app-hotkeys"} onClick={() => setActiveSubTab("app-hotkeys")} />
          <SubNavButton label="关于" active={activeSubTab === "about"} onClick={() => setActiveSubTab("about")} />
        </nav>
      </div>

      <div className="flex-1 overflow-y-auto p-12">
        {activeSubTab === "interface" && <PlaceholderContent title="界面" description="配置语言、主题、开机自启等" />}
        {activeSubTab === "app-hotkeys" && <PlaceholderContent title="应用快捷键" description="配置显示主窗口、退出应用的快捷键" />}
        {activeSubTab === "about" && <PlaceholderContent title="关于" description="版本信息、更新检查、开源协议等" />}
      </div>
    </div>
  );
}

// ============ 高级 Tab ============
function AdvancedTab({ activeSubTab, setActiveSubTab }: { activeSubTab: AdvancedSubTab; setActiveSubTab: (tab: AdvancedSubTab) => void }) {
  return (
    <div className="h-full flex bg-white">
      <div className="w-56 bg-gray-50 border-r border-gray-200 p-6">
        <h2 className="text-lg font-bold text-gray-800 mb-6 px-2">高级</h2>
        <nav className="space-y-1">
          <SubNavButton label="网络" active={activeSubTab === "network"} onClick={() => setActiveSubTab("network")} />
          <SubNavButton label="日志" active={activeSubTab === "logs"} onClick={() => setActiveSubTab("logs")} />
          <SubNavButton label="数据管理" active={activeSubTab === "data"} onClick={() => setActiveSubTab("data")} />
        </nav>
      </div>

      <div className="flex-1 overflow-y-auto p-12">
        {activeSubTab === "network" && <PlaceholderContent title="网络" description="配置代理设置、超时时间等" />}
        {activeSubTab === "logs" && <PlaceholderContent title="日志" description="配置日志级别、查看日志文件等" />}
        {activeSubTab === "data" && <PlaceholderContent title="数据管理" description="导出/导入配置、清空历史记录、清除所有数据" />}
      </div>
    </div>
  );
}

// ============ 通用组件 ============
function SubNavButton({ label, active, onClick }: { label: string; active: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className={`w-full text-left px-4 py-2 rounded-lg transition-colors text-sm ${
        active ? "bg-blue-500 text-white shadow-md font-medium" : "text-gray-700 hover:bg-gray-100"
      }`}
    >
      {label}
    </button>
  );
}

function TopTabButton({ label, active, onClick }: { label: string; active: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className={`pb-3 border-b-2 transition-colors ${
        active ? "border-blue-500 text-blue-600 font-medium" : "border-transparent text-gray-600 hover:text-gray-900"
      }`}
    >
      {label}
    </button>
  );
}

function HotkeyRow({ label, value, description }: { label: string; value: string; description?: string }) {
  return (
    <div className="flex items-center justify-between p-4 hover:bg-gray-50 transition-colors">
      <div className="flex-1">
        <div className="text-gray-700 font-medium">{label}</div>
        {description && <div className="text-xs text-gray-500 mt-1">{description}</div>}
      </div>
      <div className="flex items-center space-x-3">
        <button className="cursor-pointer" title="点击录制快捷键">
          <HotkeyDisplay value={value} />
        </button>
        {value !== "未设置" && (
          <button
            className="w-6 h-6 flex items-center justify-center text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded transition-colors"
            title="清除快捷键"
          >
            ✕
          </button>
        )}
      </div>
    </div>
  );
}

function HotkeyDisplay({ value }: { value: string }) {
  // 解析快捷键
  const hasShift = value.includes('⇧');
  const hasOption = value.includes('⌥');
  const hasCommand = value.includes('⌘');
  const hasControl = value.includes('⌃');

  // 提取字母键（最后一个非修饰键字符）
  const letterKey = value.replace(/[⇧⌥⌘⌃]/g, '').trim();

  const isUnset = value === "未设置";

  if (isUnset) {
    return (
      <div className="flex items-center space-x-1 px-4 py-2 bg-white border-2 border-dashed border-gray-300 rounded-lg text-gray-400 text-sm min-w-[280px] justify-center">
        按下快捷键
      </div>
    );
  }

  return (
    <div className="flex items-center space-x-1 px-4 py-2 bg-white border border-gray-200 rounded-lg min-w-[280px] justify-center">
      {/* Shift */}
      <span className={`text-2xl ${hasShift ? 'text-gray-700' : 'text-gray-300'}`}>⇧</span>

      {/* Option */}
      <span className={`text-2xl ${hasOption ? 'text-gray-700' : 'text-gray-300'}`}>⌥</span>

      {/* Command */}
      <span className={`text-2xl ${hasCommand ? 'text-gray-700' : 'text-gray-300'}`}>⌘</span>

      {/* Control */}
      <span className={`text-2xl ${hasControl ? 'text-gray-700' : 'text-gray-300'}`}>⌃</span>

      {/* 字母键 */}
      {letterKey && (
        <span className="text-2xl font-medium text-blue-500 ml-2">
          {letterKey}
        </span>
      )}
    </div>
  );
}

function ProviderCard({ name, status, description, icon, active = false, draggable = false }: { name: string; status: string; description: string; icon: string; active?: boolean; draggable?: boolean }) {
  const statusColor = status === "已激活" ? "bg-green-100 text-green-700" : status === "未配置" ? "bg-gray-100 text-gray-700" : "bg-blue-100 text-blue-700";

  return (
    <div className="flex items-start space-x-4 p-4 bg-gray-50 rounded-lg border border-gray-200">
      <div className="w-10 h-10 bg-blue-100 rounded-lg flex items-center justify-center flex-shrink-0">
        <span className="text-blue-600 font-semibold">{icon}</span>
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center space-x-2 mb-1">
          <h4 className="font-medium text-gray-800">{name}</h4>
          <span className={`px-2 py-0.5 text-xs rounded ${statusColor}`}>{status}</span>
          {draggable && <span className="text-gray-400 cursor-move">⋮⋮</span>}
        </div>
        <p className="text-sm text-gray-600">{description}</p>
        <div className="flex space-x-2 mt-3">
          <button className="px-3 py-1 text-sm bg-white hover:bg-gray-100 border border-gray-300 rounded transition-colors">配置</button>
          <button className="px-3 py-1 text-sm bg-white hover:bg-gray-100 border border-gray-300 rounded transition-colors">测试</button>
          {active ? (
            <button className="px-3 py-1 text-sm text-red-600 hover:bg-red-50 rounded transition-colors">停用</button>
          ) : (
            <button className="px-3 py-1 text-sm text-blue-600 hover:bg-blue-50 rounded transition-colors">激活</button>
          )}
        </div>
      </div>
    </div>
  );
}

function PlaceholderContent({ title, description }: { title: string; description: string }) {
  return (
    <div className="max-w-4xl">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">{title}</h2>
        <p className="text-gray-600">{description}</p>
      </div>
      <div className="mt-12 p-8 bg-gray-50 rounded-lg border-2 border-dashed border-gray-300 text-center">
        <p className="text-gray-500">此页面内容待实现</p>
      </div>
    </div>
  );
}

export default AppPrototypeUI;
