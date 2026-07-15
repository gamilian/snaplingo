const { useEffect, useMemo, useRef, useState } = React;

function VariantBar({ active, onChange }) {
  return (
    <header className="variant-bar">
      <div className="variant-intro">
        <strong>设置项重组 V2 · 4 种方案</strong>
        <span>已同步截图、翻译与 OCR 的删改项</span>
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
];

const libraryNav = [
  { key: "favorites", label: "收藏夹", icon: "☆" },
  { key: "history", label: "历史记录", icon: "◷" },
];

function MainNav({ active, onChange }) {
  const [capacityOpen, setCapacityOpen] = useState(false);
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
    <>
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
          <div className="capacity-head"><strong>历史与收藏</strong><button type="button" onClick={() => setCapacityOpen(true)}>设置</button></div>
          <div className="capacity-row"><span>保留天数</span><span>30 天</span></div>
          <div className="capacity-row"><span>历史上限</span><span>5,000</span></div>
          <div className="capacity-row"><span>收藏上限</span><span>1,000</span></div>
        </div>
      </aside>
      {capacityOpen && <CapacityModal onClose={() => setCapacityOpen(false)} />}
    </>
  );
}

function CapacityNumberField({ label, description, value, unit }) {
  const [current, setCurrent] = useState(value);
  return (
    <label className="capacity-field">
      <span><strong>{label}</strong><small>{description}</small></span>
      <span className="number-field">
        <input type="number" value={current} onChange={(event) => setCurrent(event.target.value)} />
        {unit && <b>{unit}</b>}
      </span>
    </label>
  );
}

function CapacityModal({ onClose }) {
  return (
    <div className="modal-layer" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section className="capacity-modal" role="dialog" aria-modal="true" aria-label="历史与收藏设置">
        <header className="modal-head">
          <div><h2>历史与收藏</h2><p>统一管理自动清理和资料库容量</p></div>
          <button type="button" onClick={onClose} aria-label="关闭">×</button>
        </header>
        <div className="modal-body">
          <div className="modal-toggle-row">
            <span><strong>自动清理历史记录</strong><small>删除超过保留天数的非收藏记录</small></span>
            <Toggle initial />
          </div>
          <CapacityNumberField label="历史记录保留天数" description="超过此天数的历史记录将被自动删除" value={30} unit="天" />
          <CapacityNumberField label="最多保留记录数" description="收藏记录不计入自动清理上限" value={5000} />
          <CapacityNumberField label="收藏夹容量" description="达到上限后停止新增，不会自动删除收藏" value={1000} />
        </div>
        <footer className="modal-footer">
          <button type="button" className="secondary-action" onClick={onClose}>取消</button>
          <button type="button" className="primary-action" onClick={onClose}>保存设置</button>
        </footer>
      </section>
    </div>
  );
}

function Toggle({ initial = true }) {
  const [active, setActive] = useState(initial);
  return <button type="button" className={`toggle ${active ? "active" : ""}`} onClick={() => setActive(!active)} aria-label="切换设置"></button>;
}

function HotkeyControl({ value }) {
  const [recording, setRecording] = useState(false);
  return (
    <button type="button" className={`keycap ${recording ? "recording" : ""}`} onClick={() => setRecording(!recording)}>
      {recording ? "请按快捷键…" : value}
    </button>
  );
}

function SelectControl({ value, options }) {
  const [selected, setSelected] = useState(value);
  const cycle = () => {
    const index = options.indexOf(selected);
    setSelected(options[(index + 1) % options.length]);
  };
  return <button type="button" className="select-value" onClick={cycle}>{selected}</button>;
}

function RangeControl({ value, min, max, unit }) {
  const [current, setCurrent] = useState(value);
  return (
    <div className="range-control">
      <input
        className="range-input"
        type="range"
        min={min}
        max={max}
        value={current}
        onChange={(event) => setCurrent(Number(event.target.value))}
      />
      <span className="range-value">{current}{unit}</span>
    </div>
  );
}

function PaletteControl({ value, colors }) {
  const [selected, setSelected] = useState(value);
  return (
    <div className="palette" aria-label="颜色选择">
      {colors.map((color) => (
        <button
          type="button"
          key={color}
          className={`swatch ${selected === color ? "selected" : ""}`}
          style={{ background: color }}
          onClick={() => setSelected(color)}
          aria-label={`选择颜色 ${color}`}
        ></button>
      ))}
    </div>
  );
}

function SettingControl({ row }) {
  if (row.type === "key") return <HotkeyControl value={row.value} />;
  if (row.type === "select") return <SelectControl value={row.value} options={row.options} />;
  if (row.type === "range") return <RangeControl value={row.value} min={row.min} max={row.max} unit={row.unit} />;
  if (row.type === "palette") return <PaletteControl value={row.value} colors={row.colors} />;
  if (row.type === "toggle") return <Toggle initial={row.value} />;
  if (row.type === "action" || row.type === "danger") {
    return <button type="button" className={`row-action ${row.type === "danger" ? "danger" : ""}`}>{row.value}</button>;
  }
  return <div className="text-value">{row.value}</div>;
}

function SettingsRows({ section }) {
  return (
    <div className="settings-groups">
      {section.groups.map((group) => (
        <section className="settings-group" key={group.title}>
          <div className="group-head">{group.title}</div>
          <div className="settings-body">
            {group.rows.map((row) => (
              <div className={`setting-row ${row.type === "key" ? "key-row" : ""}`} key={row.label}>
                <div className="setting-copy">
                  <strong>{row.label}</strong>
                  {row.description && <span>{row.description}</span>}
                </div>
                <SettingControl row={row} />
              </div>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

function SectionCard({ section, index }) {
  return (
    <section className="content-card">
      <header className="card-head">
        <div className="card-title-block">
          {Number.isInteger(index) && <span className="section-number">{String(index + 1).padStart(2, "0")}</span>}
          <div><h2>{section.label}</h2><p>{section.description}</p></div>
        </div>
        {section.key === "hotkeys" && <button type="button" className="card-action">检测冲突</button>}
      </header>
      <SettingsRows section={section} />
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
  const defaultKey = domain.defaultSection ?? domain.sections[0].key;
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
        <SectionCard section={section} />
        <div className="scheme-note"><strong>适合：</strong><span>每个功能保持 3–4 个稳定分组，用横向页签快速切换；设置数量增加后仍然容易定位。</span></div>
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
            <SectionCard section={section} />
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
  const scrollRef = useRef(null);

  useEffect(() => {
    const container = scrollRef.current;
    if (!container) return undefined;

    const updateActiveSection = () => {
      const threshold = container.getBoundingClientRect().top + 112;
      let nextActive = domain.sections[0].key;
      domain.sections.forEach((section) => {
        if (refs.current[section.key]?.getBoundingClientRect().top <= threshold) {
          nextActive = section.key;
        }
      });
      setActive((current) => current === nextActive ? current : nextActive);
    };

    container.addEventListener("scroll", updateActiveSection, { passive: true });
    updateActiveSection();
    return () => container.removeEventListener("scroll", updateActiveSection);
  }, [domain]);

  const jump = (key) => {
    setActive(key);
    refs.current[key]?.scrollIntoView({ behavior: "smooth", block: "start" });
  };
  return (
    <div ref={scrollRef} className="page-scroll" data-screen-label="单页滚动设置">
      <main className="scroll-page">
        <header className="scroll-sticky">
          <div className="scroll-title">
            <div className="scroll-title-row">
              <h1>{domain.label}</h1>
              <span className="autosave-status"><i></i>自动保存</span>
            </div>
            <span>{domain.description}</span>
          </div>
          <nav className="anchor-nav">
            {domain.sections.map((item) => <button type="button" key={item.key} aria-current={active === item.key ? "page" : undefined} className={`anchor-button ${active === item.key ? "active" : ""}`} onClick={() => jump(item.key)}>{item.label}</button>)}
          </nav>
        </header>
        <div className="scroll-stack">
          {domain.sections.map((section, index) => (
            <div key={section.key} className="scroll-section" ref={(node) => { refs.current[section.key] = node; }}>
              <SectionCard section={section} index={index} />
            </div>
          ))}
        </div>
      </main>
    </div>
  );
}

function AccordionScheme({ domainKey, domain }) {
  const defaultOpen = domain.defaultSection ?? domain.sections[0].key;
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
                {isOpen && <div className="accordion-content"><SettingsRows section={section} /></div>}
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
  const [variant] = useState("scroll");
  const [activeNav, setActiveNav] = useState("screenshot");
  const currentVariant = useMemo(() => variantOptions.find((item) => item.key === variant), [variant]);
  return (
    <div className="prototype selected-prototype">
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
