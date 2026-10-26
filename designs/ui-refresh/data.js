export const directions = [
  { id: 'native', letter: 'A', name: '澄明', english: 'Native', tone: '轻一点，专注一点。', description: '清透冷白 · 蓝色强调 · 熟悉的侧栏', detail: '留白有序，操作熟悉。为每天随手打开的桌面工具，保留恰到好处的轻盈。', layout: '经典侧栏', density: '舒适', font: '系统无衬线', accent: '#3975dd' },
  { id: 'studio', letter: 'B', name: '夜航', english: 'Studio', tone: '思绪不断，工作不停。', description: '石墨深色 · 琥珀强调 · 紧凑工作台', detail: '把常用工具放在手边，把注意力留给内容。层次分明，信息紧凑，适合沉浸工作。', layout: '工具栏 + 目录', density: '紧凑', font: '无衬线 + 等宽', accent: '#dda86b' },
  { id: 'paper', letter: 'C', name: '松间', english: 'Paper', tone: '让信息，慢慢清晰。', description: '暖白纸色 · 松绿强调 · 阅读式布局', detail: '像整理一本随身笔记。温润的色彩、松弛的节奏，让每一段文字都有呼吸的空间。', layout: '顶部导航', density: '宽松', font: '衬线 + 无衬线', accent: '#4c725a' },
];

export const navigation = [
  { id: 'general', label: '通用', icon: 'sliders', english: 'GENERAL', description: '让 SnapLingo 更贴合你的使用习惯。' },
  { id: 'screenshot', label: '截图', icon: 'scan', english: 'CAPTURE', description: '从一个像素，到一整个灵感。' },
  { id: 'translation', label: '翻译', icon: 'languages', english: 'TRANSLATION', description: '跨过语言的边界，保持思路的连贯。' },
  { id: 'ocr', label: '文字识别', icon: 'ocr', english: 'RECOGNITION', description: '图片里的文字，也可以随手取用。' },
  { id: 'services', label: '服务管理', icon: 'layers', english: 'SERVICES', description: '连接你信任的翻译、识别与语音服务。' },
  { id: 'favorites', label: '收藏夹', icon: 'bookmark', english: 'COLLECTION', description: '值得留下的内容，在这里慢慢积累。' },
  { id: 'history', label: '历史记录', icon: 'history', english: 'HISTORY', description: '找回刚才的文字，继续未完的思路。' },
];

export const scenes = [
  { id: 'workspace', label: '设置与资料库', icon: 'layout' },
  { id: 'translate', label: '翻译浮窗', icon: 'languages' },
  { id: 'recognize', label: 'OCR 浮窗', icon: 'ocr' },
  { id: 'capture', label: '截图编辑', icon: 'scan' },
  { id: 'pin', label: '贴图窗口', icon: 'pin' },
  { id: 'onboarding', label: '权限引导', icon: 'shield' },
];

const select = (id, label, value, options, description) => ({ id, label, value, options, description, type: 'select' });
const toggle = (id, label, value, description) => ({ id, label, value, description, type: 'toggle' });
const range = (id, label, value, min, max, unit, description) => ({ id, label, value, min, max, unit, description, type: 'range' });
const text = (id, label, value, description) => ({ id, label, value, description, type: 'text' });
const number = (id, label, value, min, max, unit) => ({ id, label, value, min, max, unit, type: 'number' });
const shortcut = (id, label, value, description) => ({ id, label, value, description, type: 'shortcut' });
const positionOptions = ['鼠标下方', '鼠标位置', '居中', '上次停留位置'];
const inputOptions = ['记住上次状态', '总是展开', '总是折叠'];
export const languages = ['自动检测', '简体中文', '繁體中文', 'English', '日本語', '한국어', 'Français', 'Deutsch', 'Español'];

export const settingsPages = {
  general: [
    { id: 'appearance', label: '外观与启动', icon: 'sun', description: '熟悉的界面，舒服的工作节奏。', type: 'appearance', rows: [select('language', '界面语言', '简体中文', ['简体中文']), toggle('startOnBoot', '登录时启动', true, '开机后即可使用截图和翻译快捷键。')] },
    { id: 'permissions', label: '系统权限', icon: 'shield', description: '按需授权，让每个功能顺畅工作。', type: 'permissions', rows: [] },
    { id: 'network', label: '网络连接', icon: 'globe', rows: [select('proxy', '代理设置', '跟随系统', ['跟随系统', '不使用代理', '手动配置']), text('proxyUrl', '代理地址', 'http://127.0.0.1:7890'), number('timeout', '请求超时', 10, 1, 120, '秒'), number('retryCount', '失败重试', 1, 0, 5, '次')] },
    { id: 'maintenance', label: '日志与维护', icon: 'terminal', rows: [select('logLevel', '日志级别', 'Info', ['Debug', 'Info', 'Warn', 'Error']), number('logRetention', '日志保留时间', 7, 1, 365, '天')], type: 'maintenance' },
    { id: 'experimental', label: '实验功能', icon: 'flask', rows: [toggle('gpu', 'GPU 合成加速', false, '尝试使用 GPU 加速画布合成。'), toggle('performance', '性能监控', false)] },
    { id: 'about', label: '关于应用', icon: 'info', type: 'about', rows: [toggle('autoUpdate', '自动检查更新', true)] },
  ],
  screenshot: [
    { id: 'hotkeys', label: '快捷键', icon: 'keyboard', description: '不打断当前工作，随时捕捉屏幕。', rows: [shortcut('captureKey', '截屏', '⌥ S', '框选区域，标注后保存或复制。'), shortcut('captureCopyKey', '截屏并自动复制', '⌥ ⇧ S'), shortcut('pinKey', '贴图', '⌥ P'), shortcut('pinToggleKey', '隐藏 / 显示所有贴图', '⌥ ⇧ P'), shortcut('pinGroupKey', '切换贴图组', '')] },
    { id: 'output', label: '保存与输出', icon: 'folder', rows: [text('savePath', '默认保存位置', '~/Pictures/SnapLingo'), select('format', '图片格式', 'PNG', ['PNG', 'JPG', 'WebP']), range('quality', '图片质量', 95, 50, 100, '%'), select('naming', '文件命名', '时间戳', ['时间戳', '日期', '计数器', '自定义']), text('filePrefix', '自定义文件名前缀', 'SnapLingo'), toggle('screenshotAutoCopy', '截图后自动复制', true)] },
    { id: 'interface', label: '选区与标注', icon: 'crop', type: 'captureInterface', rows: [range('borderWidth', '选区边框', 2, 1, 8, 'px'), { id: 'borderColor', label: '边框颜色', value: '#3975dd', type: 'color' }, { id: 'maskColor', label: '遮罩颜色', value: '#1d2531', type: 'color' }, range('maskOpacity', '遮罩不透明度', 38, 0, 90, '%'), toggle('selectionSize', '显示选区尺寸', true), toggle('magnifier', '显示像素放大镜', true), range('magnifierZoom', '放大镜倍率', 12, 4, 20, '×'), toggle('lastTool', '记住上次使用的工具', true)] },
    { id: 'pinSettings', label: '贴图', icon: 'pin', rows: [range('pinOpacity', '默认不透明度', 100, 20, 100, '%'), toggle('pinShadow', '显示贴图阴影', true)] },
  ],
  translation: [
    { id: 'hotkeys', label: '快捷键', icon: 'keyboard', description: '选中、按下，答案就在眼前。', rows: [shortcut('selectionKey', '划词翻译', '⌥ D', '选中文字后，快速查看翻译。'), shortcut('screenshotTranslateKey', '截图翻译', '⌥ A', '框选屏幕内容，自动识别并翻译。'), shortcut('showWindowKey', '显示翻译窗口', '⌥ F', '直接输入或粘贴需要翻译的文本。')] },
    { id: 'language', label: '语言与取词', icon: 'globe', description: '为你常用的语言，少做一次选择。', rows: [select('sourceLanguage', '源语言', '自动检测', languages, '自动识别原文所使用的语言。'), select('targetLanguage', '目标语言', '简体中文', languages.filter(x => x !== '自动检测')), select('selectionMode', '取词模式', '智能模式', ['智能模式', '效果优先', '速度优先'], '根据当前应用选择合适的取词方式。')] },
    { id: 'window', label: '窗口与输入', icon: 'layout', rows: [select('selectionPosition', '划词与截图翻译窗口', '鼠标下方', positionOptions), select('inputPosition', '手动翻译窗口', '居中', positionOptions), range('windowWidth', '窗口宽度', 660, 300, 1000, 'pt'), range('windowHeight', '最高屏幕占比', 70, 30, 90, '%'), select('selectionInput', '划词翻译输入框', '记住上次状态', inputOptions), select('captureInput', '截图翻译输入框', '记住上次状态', inputOptions)] },
    { id: 'behavior', label: '翻译行为', icon: 'spark', rows: [toggle('autoTranslate', '自动翻译', true, '输入停顿后自动开始翻译。'), toggle('incremental', '增量翻译', true, '追加文字时，保留已经完成的结果。'), toggle('translationAutoCopy', '翻译后自动复制', false), toggle('lineBreaks', '保留原文换行', true), toggle('alwaysOnTop', '结果窗口置顶', false), toggle('hideOnBlur', '失去焦点时隐藏', true)] },
  ],
  ocr: [
    { id: 'hotkeys', label: '快捷键', icon: 'keyboard', description: '让图片里的信息变成可编辑的文字。', rows: [shortcut('ocrKey', '截图识别', '⌥ O', '框选区域，提取图片中的文字。'), shortcut('silentOcrKey', '静默截图识别', '⌥ ⇧ O', '识别后直接复制，不打开结果窗口。'), shortcut('fileOcrKey', '从图片文件识别', '⌥ U')] },
    { id: 'recognition', label: '识别与文本', icon: 'text', rows: [select('ocrLanguage', '识别语言', '自动检测', languages.slice(0, 6)), toggle('preserveFormat', '保留文本格式', true), toggle('removeSpaces', '中文自动去除空格', true), toggle('showConfidence', '显示识别置信度', true)] },
    { id: 'window', label: '窗口与提示', icon: 'layout', rows: [select('ocrPosition', '结果窗口位置', '鼠标位置', positionOptions), toggle('hideOcrStatus', '隐藏静默识别状态提示', false)] },
  ],
};

export const defaultSettings = Object.fromEntries(Object.values(settingsPages).flatMap(sections => sections.flatMap(section => section.rows.map(row => [row.id, row.value]))));

export const initialProviders = [
  { id: 'google', name: 'Google 翻译', mark: 'G', color: '#4285f4', description: '覆盖多种语言，日常翻译随手可用。', group: 'translation', active: true, configured: true, builtin: true, tag: '内置服务' },
  { id: 'deeplx', name: 'DeepLX', mark: 'D', color: '#365669', description: '连接自己的 DeepLX 服务。', group: 'translation', active: true, configured: true, builtin: true, tag: '自部署', endpoint: 'http://localhost:1188/translate' },
  { id: 'baidu', name: '百度翻译', mark: '百', color: '#5565d8', description: '使用百度翻译开放平台的应用凭证。', group: 'translation', active: false, configured: false, builtin: true, tag: '需要密钥' },
  { id: 'llm', name: '我的 AI 翻译', mark: 'AI', color: '#64886c', description: '使用自定义提示词，获得更贴合语境的译文。', group: 'translation', active: false, configured: true, builtin: false, tag: '自定义', endpoint: 'https://api.example.com/v1' },
  { id: 'apple', name: 'Apple Vision', mark: 'A', color: '#707982', description: '系统原生文字识别，图片在设备上处理。', group: 'ocr', active: true, configured: true, builtin: true, tag: '本地处理' },
  { id: 'tesseract', name: 'Tesseract', mark: 'T', color: '#577bc5', description: '开源识别引擎，支持多语言离线识别。', group: 'ocr', active: false, configured: true, builtin: true, tag: '本地处理' },
  { id: 'baidu-ocr', name: '百度 OCR', mark: '百', color: '#5565d8', description: '连接百度智能云文字识别服务。', group: 'ocr', active: false, configured: false, builtin: true, tag: '云端识别' },
];

export const sourceText = 'Good design is as little design as possible. Less, but better — because it concentrates on the essential aspects, and the products are not burdened with non-essentials.';
export const translatedText = '好的设计是尽可能少的设计。更少，却更好——因为它专注于本质，让产品不被多余之物所累。';
export const alternateTranslation = '好的设计，是尽可能少的设计。少，但更好。专注于核心，让产品摆脱一切不必要的负担。';
export const sampleOcrText = 'Good design is as little design as possible.\nLess, but better.\n\nDieter Rams · Ten principles for good design';

export const initialRecords = [
  { id: 'r1', kind: 'translation', title: 'Less, but better.', source: sourceText, result: translatedText, provider: 'Google 翻译', time: '刚刚', day: '今天', favorite: true, tags: ['设计', '阅读'], note: '少一些干扰，多一些专注。' },
  { id: 'r2', kind: 'ocr', title: 'Ten principles for good design', source: sampleOcrText, result: sampleOcrText, provider: 'Apple Vision', time: '12 分钟前', day: '今天', favorite: false, tags: [], note: '' },
  { id: 'r3', kind: 'translation', title: 'Make time for what matters.', source: 'Make time for what matters.', result: '为真正重要的事情留出时间。', provider: 'DeepLX', time: '28 分钟前', day: '今天', favorite: true, tags: ['日常'], note: '' },
  { id: 'r4', kind: 'translation', title: 'The details are not the details.', source: 'The details are not the details. They make the design.', result: '细节不只是细节，它们成就了设计。', provider: 'Google 翻译', time: '1 小时前', day: '今天', favorite: false, tags: [], note: '' },
  { id: 'r5', kind: 'screenshot', title: '设计原则 · 阅读摘录', source: sampleOcrText, result: '960 × 600 · PNG', provider: '截图', time: '昨天 18:42', day: '昨天', favorite: true, tags: ['灵感'], note: '用在下一次产品评审中。' },
  { id: 'r6', kind: 'ocr', title: '把注意力放回内容本身', source: '把注意力放回内容本身。\n让工具成为思考的延伸。', result: '把注意力放回内容本身。\n让工具成为思考的延伸。', provider: 'Apple Vision', time: '昨天 16:20', day: '昨天', favorite: true, tags: ['笔记'], note: '' },
  { id: 'r7', kind: 'translation', title: 'Simplicity is the ultimate sophistication.', source: 'Simplicity is the ultimate sophistication.', result: '简约，是复杂的终极形式。', provider: 'DeepLX', time: '昨天 14:08', day: '昨天', favorite: false, tags: [], note: '' },
  { id: 'r8', kind: 'translation', title: 'Stay curious.', source: 'Stay curious. There is always more to discover.', result: '保持好奇，总有新的发现在等你。', provider: 'Google 翻译', time: '10 月 24 日', day: '更早', favorite: true, tags: ['日常'], note: '' },
];
