const settingsDomains = {
  screenshot: {
    label: "截图",
    description: "截图快捷键、文件保存和编辑工具",
    sections: [
      { key: "hotkeys", label: "快捷键", short: "5 个快捷键", description: "截屏、复制与贴图的全局快捷键", icon: "⌘" },
      { key: "save", label: "保存设置", short: "PNG · 自动复制", description: "保存位置、图片格式和命名规则", icon: "↓" },
      { key: "editor", label: "编辑器", short: "红色 · 1px · 16px", description: "标注颜色、线条和文字的默认值", icon: "A" },
    ],
  },
  translation: {
    label: "翻译",
    description: "翻译快捷键与默认翻译行为",
    sections: [
      { key: "hotkeys", label: "快捷键", short: "3 个快捷键", description: "文本翻译与截图翻译快捷键", icon: "⌘" },
      { key: "translation", label: "翻译设置", short: "自动检测 → 中文简体", description: "默认语言、翻译行为和窗口行为", icon: "译" },
    ],
  },
  ocr: {
    label: "OCR",
    description: "OCR 快捷键与识别结果设置",
    sections: [
      { key: "hotkeys", label: "快捷键", short: "2 个快捷键", description: "截图 OCR 与图片 OCR 快捷键", icon: "⌘" },
      { key: "ocr", label: "OCR 设置", short: "自动检测 · 自动复制", description: "识别语言、格式保留和结果展示", icon: "O" },
    ],
  },
};

const variantOptions = [
  { key: "tabs", letter: "A", name: "顶部页签", note: "最稳妥，改动最小" },
  { key: "overview", letter: "B", name: "总览后进入", note: "层级清楚，适合继续扩展" },
  { key: "scroll", letter: "C", name: "单页滚动", note: "一次看全，查找更快" },
  { key: "accordion", letter: "D", name: "折叠分组", note: "结构紧凑，减少跳转" },
];

Object.assign(window, { settingsDomains, variantOptions });
