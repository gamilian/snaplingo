const { useMemo, useRef, useState } = React;

function VariantBar({ active, onChange }) {
  return (
    <header className="variant-bar">
      <div className="variant-intro">
        <strong>取消二级栏 · 4 种方案</strong>
        <span>左侧一级导航保持不变，仅比较内容区组织方式</span>
      </div>
      <div className="variant-options" role="tablist" aria-label="设计方案">
        {variantOptions.map((option) => (
          <button
            type="button"
            key={option.key}
            className={`variant-button ${active === option.key ? "active" : ""}`}
            onClick={() => onChange(option.key)}
          >
            <span className="variant-letter">{option.letter}</span>
            <span className="variant-copy">
              <strong>{option.name}</strong>
              <span>{option.note}</span>
            </span>
          </button>
        ))}
      </div>
    </header>
  );
}

const settingsNav = [
  { key: "general", label: "通用", icon: "≡" },
  { key: "screenshot", label: "截图", icon: "□" },
  { key: "translation", label: "翻译", icon: "T" },
  { key: "ocr", label: "OCR", icon: "O" },
  { key: "services", label: "服务", icon: "S" },
  { key: "advanced", label: "高级", icon: "✦" },
];

const libraryNav = [
  { key: "favorites", label: "收藏夹", icon: "☆" },
  { key: "history", label: "历史记录", icon: "◷" },
];

function MainNav({ active, onChange }) {
  const renderGroup = (label, items) => (
    <div className="nav-group">
      <div className="nav-label">{label}</div>
      {items.map((item) => (
        <button
          type="button"
          key={item.key}
          className={`nav-item ${active === item.key ? "active" : ""}`}
          onClick={() => onChange(item.key)}
        >
          <span className="nav-icon">{item.icon}</span>
          <span className="nav-text">{item.label}</span>
        </button>
      ))}
    </div>
  );

  return (
    <aside className="main-nav">
      <div className="brand">
        <div className="brand-mark">S</div>
        <div className="brand-name">SnapLingo</div>
      </div>
      <div className="nav-scroll">
        {renderGroup("设置", settingsNav)}
        {renderGroup("资料库", libraryNav)}
      </div>
      <div className="capacity">
        <div className="capacity-head"><strong>容量</strong><button type="button">设置</button></div>
        <div className="capacity-row"><span>历史上限</span><span>5,000</span></div>
        <div className="capacity-row"><span>收藏上限</span><span>1,000</span></div>
      </div>
    </aside>
  );
}

function Toggle({ initial = true }) {
  const [active, setActive] = useState(initial);
  return <button type="button" className={`toggle ${active ? "active" : ""}`} onClick={() => setActive(!active)} aria-label="切换设置"></button>;
}

function SettingsRows({ domainKey, sectionKey, compact = false }) {
  if (sectionKey === "hotkeys") {
    const rows = domainKey === "screenshot"
      ? [["截屏", "选择区域并打开截图编辑器", "⇧⌘2"], ["截屏并自动复制", "完成选区后直接复制到剪贴板", "⇧⌘C"], ["贴图", "将剪贴板图片固定在桌面", "⇧⌘P"]]
      : domainKey === "translation"
        ? [["文本翻译", "打开文本翻译窗口", "⌥⌘T"], ["截图翻译", "选择屏幕区域并直接翻译", "⌥⌘R"], ["翻译剪贴板", "翻译当前剪贴板文本", "⌥⌘V"]]
        : [["截图 OCR", "选择屏幕区域并识别文字", "⌥⌘O"], ["图片 OCR", "识别剪贴板中的图片", "⌥⌘I"]];
    return (
      <div className="settings-body">
        {rows.slice(0, compact ? 2 : rows.length).map(([label, description, key]) => (
          <div className="setting-row" key={label}>
            <div className="setting-copy"><strong>{label}</strong><span>{description}</span></div>
            <div className="keycap">{key}</div>
          </div>
        ))}
      </div>
    );
  }

  if (sectionKey === "save") {
    return (
      <div className="settings-body">
        <SettingRow title="默认保存路径" description="截图文件将保存到此文件夹"><div className="select-value">~/Pictures/SnapLingo</div></SettingRow>
        <SettingRow title="图片格式" description="PNG 无损压缩，适合界面截图"><div className="select-value">PNG</div></SettingRow>
        {!compact && <SettingRow title="截图后自动复制" description="截图完成后自动复制到剪贴板"><Toggle /></SettingRow>}
      </div>
    );
  }

  if (sectionKey === "editor") {
    return (
      <div className="settings-body">
        <SettingRow title="标注颜色" description="编辑器工具栏默认使用的预设颜色">
          <div className="palette">
            <span className="swatch selected" style={{ background: "#ff4d4f" }}></span>
            <span className="swatch" style={{ background: "#27aa4b" }}></span>
            <span className="swatch" style={{ background: "#268df0" }}></span>
            <span className="swatch" style={{ background: "#ffd416" }}></span>
          </div>
        </SettingRow>
        <SettingRow title="默认线条粗细" description="直线、箭头和形状的默认粗细"><div className="range-control"><div className="range-track"></div><span className="range-value">1px</span></div></SettingRow>
        {!compact && <SettingRow title="默认字体大小" description="文字标注的默认字体大小"><div className="select-value">16px（中）</div></SettingRow>}
      </div>
    );
  }

  if (sectionKey === "translation") {
    return (
      <div className="settings-body">
        <SettingRow title="默认语言" description="自动识别源语言并翻译为中文简体"><div className="select-value">自动检测 → 中文简体</div></SettingRow>
        <SettingRow title="自动翻译" description="输入文本后自动开始翻译"><Toggle /></SettingRow>
        {!compact && <SettingRow title="失去焦点时隐藏" description="点击窗口外部时隐藏翻译结果"><Toggle /></SettingRow>}
      </div>
    );
  }

  return (
    <div className="settings-body">
      <SettingRow title="识别语言" description="用于截图 OCR 和上传图片 OCR"><div className="select-value">自动检测</div></SettingRow>
      <SettingRow title="识别后自动复制" description="完成识别后复制结果到剪贴板"><Toggle /></SettingRow>
      {!compact && <SettingRow title="保留文本格式" description="保留换行和段落结构"><Toggle /></SettingRow>}
    </div>
  );
}

function SettingRow({ title, description, children }) {
  return (
    <div className="setting-row">
      <div className="setting-copy"><strong>{title}</strong><span>{description}</span></div>
      {children}
    </div>
  );
}

function SectionCard({ domainKey, section, compact = false }) {
  return (
    <section className="content-card">
      <header className="card-head">
        <div><h2>{section.label}</h2><p>{section.description}</p></div>
        {section.key === "hotkeys" && <button type="button" className="card-action">检测冲突</button>}
      </header>
      <SettingsRows domainKey={domainKey} sectionKey={section.key} compact={compact} />
    </section>
  );
}

function PageHeader({ domain }) {
  return (
    <header className="page-header">
      <div className="page-heading"><h1>{domain.label}</h1><p>{domain.description}</p></div>
      <div className="context-badge"><i></i>设置自动保存</div>
    </header>
  );
}

function TabsScheme({ domainKey, domain }) {
  const defaultKey = domainKey === "screenshot" ? "editor" : domain.sections[0].key;
  const [active, setActive] = useState(defaultKey);
  const availableActive = domain.sections.some((section) => section.key === active) ? active : domain.sections[0].key;
  const section = domain.sections.find((item) => item.key === availableActive) || domain.sections[0];
  return (
    <div className="page-scroll" data-screen-label="方案 A · 顶部页签">
      <main className="page">
        <PageHeader domain={domain} />
        <nav className="section-tabs">
          {domain.sections.map((item) => <button type="button" key={item.key} className={`section-tab ${item.key === section.key ? "active" : ""}`} onClick={() => setActive(item.key)}>{item.label}</button>)}
        </nav>
        <SectionCard domainKey={domainKey} section={section} />
        <div className="scheme-note"><strong>适合：</strong><span>用户已经熟悉现有“快捷键 / 设置 / 编辑器”的划分，希望最小成本取消中间栏。页面数量不多时最直观。</span></div>
      </main>
    </div>
  );
}

function OverviewScheme({ domainKey, domain }) {
  const [detail, setDetail] = useState(null);
  const section = domain.sections.find((item) => item.key === detail);
  return (
    <div className="page-scroll" data-screen-label="方案 B · 总览后进入">
      <main className="page">
        {section ? (
          <>
            <button type="button" className="detail-back" onClick={() => setDetail(null)}>← 返回{domain.label}设置</button>
            <PageHeader domain={{ ...domain, label: section.label, description: section.description }} />
            <SectionCard domainKey={domainKey} section={section} />
          </>
        ) : (
          <>
            <PageHeader domain={domain} />
            <div className="overview-grid">
              {domain.sections.map((item) => (
                <button type="button" key={item.key} className="overview-card" onClick={() => setDetail(item.key)}>
                  <span className="overview-icon">{item.icon}</span>
                  <span className="overview-copy"><strong>{item.label}</strong><span>{item.description}</span></span>
                  <span className="overview-meta"><b>{item.short}</b><span>打开 →</span></span>
                </button>
              ))}
            </div>
            <div className="scheme-note"><strong>适合：</strong><span>后续设置项可能继续增加。入口页先展示每组的状态摘要，用户能快速确认当前配置，再进入完整页面。</span></div>
          </>
        )}
      </main>
    </div>
  );
}

function ScrollScheme({ domainKey, domain }) {
  const [active, setActive] = useState(domain.sections[0].key);
  const refs = useRef({});
  const jump = (key) => {
    setActive(key);
    refs.current[key]?.scrollIntoView({ behavior: "smooth", block: "start" });
  };
  return (
    <div className="page-scroll" data-screen-label="方案 C · 单页滚动">
      <main className="scroll-page">
        <header className="scroll-sticky">
          <div className="scroll-title"><h1>{domain.label}</h1><span>{domain.description}</span></div>
          <nav className="anchor-nav">
            {domain.sections.map((item) => <button type="button" key={item.key} className={`anchor-button ${active === item.key ? "active" : ""}`} onClick={() => jump(item.key)}>{item.label}</button>)}
          </nav>
        </header>
        <div className="scroll-stack">
          {domain.sections.map((section) => (
            <div key={section.key} className="scroll-section" ref={(node) => { refs.current[section.key] = node; }} onMouseEnter={() => setActive(section.key)}>
              <SectionCard domainKey={domainKey} section={section} compact />
            </div>
          ))}
        </div>
        <div className="scheme-note"><strong>适合：</strong><span>设置总量有限、用户经常连续调整多组配置。页面内锚点只负责定位，不形成新的视觉分栏。</span></div>
      </main>
    </div>
  );
}

function AccordionScheme({ domainKey, domain }) {
  const defaultOpen = domainKey === "screenshot" ? "editor" : domain.sections[0].key;
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="page-scroll" data-screen-label="方案 D · 折叠分组">
      <main className="page">
        <PageHeader domain={domain} />
        <div className="accordion">
          {domain.sections.map((section, index) => {
            const isOpen = open === section.key;
            return (
              <section key={section.key} className={`accordion-item ${isOpen ? "open" : ""}`}>
                <button type="button" className="accordion-toggle" onClick={() => setOpen(isOpen ? null : section.key)}>
                  <span className="accordion-index">0{index + 1}</span>
                  <span className="accordion-copy"><strong>{section.label}</strong><span>{isOpen ? section.description : section.short}</span></span>
                  <span className="accordion-caret">⌄</span>
                </button>
                {isOpen && <div className="accordion-content"><SettingsRows domainKey={domainKey} sectionKey={section.key} /></div>}
              </section>
            );
          })}
        </div>
        <div className="scheme-note"><strong>适合：</strong><span>希望保留“全部设置都在一页”的感知，同时降低长页面的视觉噪音。移动到小窗口时也最容易适配。</span></div>
      </main>
    </div>
  );
}

function Workspace({ variant, activeNav }) {
  if (!settingsDomains[activeNav]) return <div className="empty-page">本原型重点比较截图、翻译和 OCR 的内部导航。</div>;
  const domain = settingsDomains[activeNav];
  const key = `${variant}-${activeNav}`;
  if (variant === "overview") return <OverviewScheme key={key} domainKey={activeNav} domain={domain} />;
  if (variant === "scroll") return <ScrollScheme key={key} domainKey={activeNav} domain={domain} />;
  if (variant === "accordion") return <AccordionScheme key={key} domainKey={activeNav} domain={domain} />;
  return <TabsScheme key={key} domainKey={activeNav} domain={domain} />;
}

function App() {
  const [variant, setVariant] = useState("tabs");
  const [activeNav, setActiveNav] = useState("screenshot");
  const currentVariant = useMemo(() => variantOptions.find((item) => item.key === variant), [variant]);
  return (
    <div className="prototype">
      <VariantBar active={variant} onChange={setVariant} />
      <section className="window" aria-label={`SnapLingo ${currentVariant.name}方案`}>
        <div className="titlebar"><div className="traffic"><i></i><i></i><i></i></div><div className="window-title">SnapLingo</div></div>
        <div className="app">
          <MainNav active={activeNav} onChange={setActiveNav} />
          <div className="workspace"><Workspace variant={variant} activeNav={activeNav} /></div>
        </div>
      </section>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(<App />);
