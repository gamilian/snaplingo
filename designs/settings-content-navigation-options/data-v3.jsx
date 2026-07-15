const settingsDomains = {
  general: {
    label: "通用",
    description: "应用界面、网络、日志与维护",
    defaultSection: "interface",
    sections: [
      {
        key: "interface",
        label: "界面与启动",
        short: "中文简体 · 跟随系统 · 开机自启",
        description: "应用外观与系统启动行为",
        icon: "≡",
        groups: [
          {
            title: "界面",
            rows: [
              { type: "select", label: "界面语言", value: "中文简体", options: ["中文简体", "中文繁體", "English", "日本語"] },
              { type: "select", label: "主题", value: "跟随系统", options: ["跟随系统", "浅色", "深色"] },
              { type: "toggle", label: "开机自启", description: "系统启动时自动运行 SnapLingo", value: true },
            ],
          },
        ],
      },
      {
        key: "network",
        label: "网络",
        short: "系统代理 · 10 秒 · 重试 3 次",
        description: "代理、超时与失败重试策略",
        icon: "N",
        groups: [
          {
            title: "连接",
            rows: [
              { type: "select", label: "代理设置", value: "使用系统代理", options: ["不使用代理", "使用系统代理", "自定义代理"] },
              { type: "range", label: "请求超时时间", value: 10, min: 5, max: 60, unit: " 秒" },
              { type: "select", label: "失败重试次数", value: "3 次（推荐）", options: ["不重试", "1 次", "2 次", "3 次（推荐）", "5 次"] },
            ],
          },
        ],
      },
      {
        key: "maintenance",
        label: "日志与维护",
        short: "错误日志 · 清理数据 · 重置设置",
        description: "日志记录和本地数据维护工具",
        icon: "L",
        groups: [
          {
            title: "日志",
            rows: [
              { type: "select", label: "日志级别", value: "错误（Error）", options: ["错误（Error）", "警告（Warn）", "信息（Info）", "调试（Debug）"] },
              { type: "toggle", label: "保存日志到文件", description: "将日志输出到文件，便于排查问题", value: false },
              { type: "action", label: "日志目录", value: "打开日志目录" },
            ],
          },
          {
            title: "数据维护",
            rows: [
              { type: "action", label: "历史记录", value: "清除所有历史记录" },
              { type: "action", label: "本地缓存", value: "清除所有缓存" },
              { type: "danger", label: "应用设置", description: "恢复为首次安装时的默认设置", value: "重置所有设置" },
            ],
          },
        ],
      },
      {
        key: "experimental",
        label: "实验性功能",
        short: "GPU 加速 · 性能监控",
        description: "尚在验证中的性能功能",
        icon: "β",
        groups: [
          {
            title: "性能",
            rows: [
              { type: "toggle", label: "GPU 加速", description: "使用 GPU 加速 OCR 识别", value: false },
              { type: "toggle", label: "性能监控", description: "显示 CPU、内存占用等性能指标", value: false },
            ],
          },
        ],
      },
      {
        key: "about",
        label: "关于",
        short: "SnapLingo 0.1.0 · MIT License",
        description: "版本、开源协议与更新",
        icon: "i",
        groups: [
          {
            title: "SnapLingo",
            rows: [
              { type: "text", label: "当前版本", value: "0.1.0" },
              { type: "text", label: "开源协议", value: "MIT License" },
              { type: "action", label: "软件更新", value: "检查更新" },
            ],
          },
        ],
      },
    ],
  },
  screenshot: {
    label: "截图",
    description: "截图快捷键、文件输出与选区界面",
    defaultSection: "capture",
    sections: [
      {
        key: "hotkeys",
        label: "快捷键",
        short: "5 个快捷键",
        description: "截图与贴图的全局快捷键",
        icon: "⌘",
        groups: [
          {
            title: "截图与贴图",
            rows: [
              { type: "key", label: "截屏", value: "⇧⌘2" },
              { type: "key", label: "截屏并自动复制", value: "⇧⌘C" },
              { type: "key", label: "贴图", value: "⇧⌘P" },
              { type: "key", label: "隐藏 / 显示所有贴图", value: "⇧⌘H" },
              { type: "key", label: "切换到另一贴图组", value: "⇧⌘G" },
            ],
          },
        ],
      },
      {
        key: "output",
        label: "保存与输出",
        short: "PNG · 90% · 自动复制",
        description: "保存位置、图片格式和完成后的动作",
        icon: "↓",
        groups: [
          {
            title: "文件",
            rows: [
              { type: "text", label: "默认保存路径", description: "截图文件保存到此文件夹", value: "~/Pictures/SnapLingo" },
              { type: "select", label: "图片格式", value: "PNG", options: ["PNG", "JPG", "WebP"] },
              { type: "range", label: "图片质量", description: "仅对 JPG 和 WebP 格式生效", value: 90, min: 50, max: 100, unit: "%" },
              { type: "select", label: "文件命名规则", value: "时间戳", options: ["时间戳", "日期", "计数器", "自定义"] },
            ],
          },
          {
            title: "完成后",
            rows: [
              { type: "toggle", label: "截图后自动复制", description: "截图完成后复制到剪贴板", value: true },
            ],
          },
        ],
      },
      {
        key: "capture",
        label: "截图界面",
        short: "2px · 深色遮罩 · 尺寸与放大镜",
        description: "配置选区外观、辅助信息与贴图显示",
        icon: "□",
        groups: [
          {
            title: "选区外观",
            rows: [
              { type: "range", label: "边框宽度", description: "截图选区边框的显示宽度", value: 2, min: 1, max: 8, unit: "px" },
              { type: "palette", label: "遮罩颜色", description: "选区外区域使用的遮罩颜色", value: "#20242c", colors: ["#20242c", "#3e4654", "#697386", "#ffffff"] },
            ],
          },
          {
            title: "选区辅助",
            rows: [
              { type: "toggle", label: "显示选区尺寸", description: "选取时显示宽度和高度", value: true },
              { type: "toggle", label: "显示放大镜", description: "靠近选区边缘时显示像素放大镜", value: true },
            ],
          },
          {
            title: "编辑器与贴图",
            rows: [
              { type: "toggle", label: "记住上次使用的工具", description: "下次截图时恢复上次使用的标注工具", value: true },
              { type: "range", label: "贴图默认透明度", value: 92, min: 20, max: 100, unit: "%" },
              { type: "toggle", label: "贴图显示阴影", value: true },
            ],
          },
        ],
      },
    ],
  },
  translation: {
    label: "翻译",
    description: "快捷键、语言规则、窗口与翻译行为",
    defaultSection: "window",
    sections: [
      {
        key: "hotkeys",
        label: "快捷键",
        short: "3 个快捷键",
        description: "常用翻译入口的全局快捷键",
        icon: "⌘",
        groups: [
          {
            title: "翻译入口",
            rows: [
              { type: "key", label: "划词翻译", value: "⌥⌘T" },
              { type: "key", label: "截图翻译", value: "⌥⌘R" },
              { type: "key", label: "显示翻译窗口", value: "⌥⌘V" },
            ],
          },
        ],
      },
      {
        key: "language",
        label: "语言与取词",
        short: "自动检测 → 自动 · 智能模式",
        description: "默认语言与划词翻译的取词策略",
        icon: "译",
        groups: [
          {
            title: "默认语言",
            rows: [
              { type: "select", label: "源语言", value: "自动检测", options: ["自动检测", "中文简体", "中文繁體", "English", "日本語", "한국어"] },
              {
                type: "select",
                label: "目标语言",
                description: "自动：源语言为中文时翻译成英语，其他语言翻译成中文",
                value: "自动",
                options: ["自动", "中文简体", "中文繁體", "English", "日本語", "한국어"],
              },
            ],
          },
          {
            title: "划词翻译",
            rows: [
              { type: "select", label: "取词模式", description: "智能模式会根据应用和文本类型选择取词方式", value: "智能模式", options: ["智能模式", "效果优先", "速度优先"] },
            ],
          },
        ],
      },
      {
        key: "window",
        label: "窗口与输入框",
        short: "鼠标下方 · 560pt · 上次状态",
        description: "翻译窗口的位置、尺寸与输入框初始状态",
        icon: "▣",
        groups: [
          {
            title: "窗口位置",
            rows: [
              { type: "select", label: "划词翻译与截图翻译", value: "鼠标下方", options: ["居中", "鼠标下方", "鼠标位置"] },
              { type: "select", label: "输入翻译窗口", value: "居中", options: ["居中", "鼠标下方", "鼠标位置"] },
            ],
          },
          {
            title: "窗口尺寸",
            rows: [
              { type: "range", label: "最高屏幕占比", value: 70, min: 30, max: 90, unit: "%" },
              { type: "range", label: "窗口宽度", value: 560, min: 300, max: 1000, unit: "pt" },
            ],
          },
          {
            title: "输入框初始状态",
            rows: [
              { type: "select", label: "划词翻译", value: "上次状态", options: ["上次状态", "总是折叠", "总是展开"] },
              { type: "select", label: "截图翻译", value: "上次状态", options: ["上次状态", "总是折叠", "总是展开"] },
            ],
          },
        ],
      },
      {
        key: "behavior",
        label: "翻译行为",
        short: "自动翻译 · 保留换行 · 失焦隐藏",
        description: "输入、结果与窗口的通用行为",
        icon: "↯",
        groups: [
          {
            title: "输入与结果",
            rows: [
              { type: "toggle", label: "自动翻译", description: "输入文本后自动开始翻译", value: true },
              { type: "toggle", label: "增量翻译", description: "输入时实时更新翻译结果", value: true },
              { type: "toggle", label: "翻译后自动复制", value: false },
              { type: "toggle", label: "保留原文换行", value: true },
            ],
          },
          {
            title: "窗口行为",
            rows: [
              { type: "toggle", label: "结果窗口置顶", value: true },
              { type: "toggle", label: "失去焦点时隐藏", value: true },
            ],
          },
        ],
      },
    ],
  },
  ocr: {
    label: "OCR",
    description: "快捷键、识别文本与结果窗口",
    defaultSection: "window",
    sections: [
      {
        key: "hotkeys",
        label: "快捷键",
        short: "3 个快捷键",
        description: "常用 OCR 入口的全局快捷键",
        icon: "⌘",
        groups: [
          {
            title: "识别入口",
            rows: [
              { type: "key", label: "截图 OCR", value: "⌥⌘O" },
              { type: "key", label: "静默截图 OCR", value: "⌥⇧⌘O" },
              { type: "key", label: "上传图片 OCR", value: "⌥⌘I" },
            ],
          },
        ],
      },
      {
        key: "recognition",
        label: "识别与文本",
        short: "自动检测 · 保留格式 · 中文去空格",
        description: "识别语言与结果文本的处理方式",
        icon: "文",
        groups: [
          {
            title: "识别",
            rows: [
              { type: "select", label: "识别语言", value: "自动检测", options: ["自动检测", "中文简体", "中文繁體", "English", "日本語", "한국어"] },
              { type: "toggle", label: "保留文本格式", description: "保留换行和段落结构", value: true },
              { type: "toggle", label: "中文自动去除空格", value: true },
              { type: "toggle", label: "显示识别置信度", value: false },
            ],
          },
        ],
      },
      {
        key: "window",
        label: "窗口与提示",
        short: "鼠标位置 · 显示状态提示",
        description: "OCR 结果窗口位置与静默识别提示",
        icon: "▣",
        groups: [
          {
            title: "结果窗口",
            rows: [
              { type: "select", label: "OCR 窗口位置", value: "鼠标位置", options: ["居中", "鼠标下方", "鼠标位置"] },
            ],
          },
          {
            title: "静默 OCR",
            rows: [
              {
                type: "toggle",
                label: "隐藏识别状态提示",
                description: "开启后，后台识别时不再在鼠标位置显示加载和成功提示",
                value: false,
              },
            ],
          },
        ],
      },
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
