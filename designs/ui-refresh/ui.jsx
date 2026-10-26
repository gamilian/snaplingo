import React, { createContext, useContext, useEffect, useRef } from 'react';

export const PreviewContext = createContext(null);
export const usePreview = () => useContext(PreviewContext);

const paths = {
  sliders: <><path d="M4 6h16M4 12h16M4 18h16"/><circle cx="8" cy="6" r="2"/><circle cx="16" cy="12" r="2"/><circle cx="10" cy="18" r="2"/></>,
  scan: <><path d="M8 3H5a2 2 0 0 0-2 2v3m13-5h3a2 2 0 0 1 2 2v3M3 16v3a2 2 0 0 0 2 2h3m8 0h3a2 2 0 0 0 2-2v-3"/><rect x="7" y="7" width="10" height="10" rx="1.5"/></>,
  languages: <><path d="M3 5h12M9 3v2m4 0c-1 7-4 10-9 12m1-8c2 4 5 6 8 7m1 5 4-10 4 10m-6-4h4"/></>,
  ocr: <><path d="M7 3H5a2 2 0 0 0-2 2v2m14-4h2a2 2 0 0 1 2 2v2M3 17v2a2 2 0 0 0 2 2h2m10 0h2a2 2 0 0 0 2-2v-2M7 8h10M12 8v9m-3 0h6"/></>,
  layers: <><path d="m12 3 10 5-10 5L2 8l10-5Zm-9 9 9 5 9-5M3 16l9 5 9-5"/></>,
  bookmark: <path d="M6 4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v17l-6-4-6 4V4Z"/>,
  history: <><path d="M3 10a9 9 0 1 1 2 8M3 4v6h6M12 7v5l3 2"/></>,
  globe: <><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a17 17 0 0 1 0 18 17 17 0 0 1 0-18Z"/></>,
  search: <><circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/></>,
  arrowRight: <path d="M4 12h16m-6-6 6 6-6 6"/>,
  arrowLeft: <path d="M20 12H4m6-6-6 6 6 6"/>,
  arrowUp: <path d="M12 20V4m-6 6 6-6 6 6"/>,
  arrowDown: <path d="M12 4v16m-6-6 6 6 6-6"/>,
  chevron: <path d="m6 9 6 6 6-6"/>,
  chevronRight: <path d="m9 5 7 7-7 7"/>,
  check: <path d="m5 12 4 4L19 6"/>,
  close: <path d="m6 6 12 12M6 18 18 6"/>,
  plus: <path d="M12 5v14M5 12h14"/>,
  minus: <path d="M5 12h14"/>,
  sun: <><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"/></>,
  moon: <path d="M20.5 13.3A8.5 8.5 0 0 1 10.7 3a9 9 0 1 0 9.8 10.3Z"/>,
  monitor: <><rect x="3" y="3" width="18" height="13" rx="2"/><path d="M8 21h8m-4-5v5"/></>,
  keyboard: <><rect x="2" y="5" width="20" height="14" rx="2"/><path d="M6 9h.01M10 9h.01M14 9h.01M18 9h.01M6 12h.01M10 12h.01M14 12h.01M18 12h.01M7 15h10"/></>,
  shield: <><path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6l8-3Z"/><path d="m8 12 3 3 5-6"/></>,
  folder: <path d="M3 7V5a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7h18"/>,
  terminal: <><rect x="3" y="4" width="18" height="16" rx="2"/><path d="m7 8 4 4-4 4m7 0h3"/></>,
  flask: <><path d="M9 3h6m-5 0v6l-6 9a2 2 0 0 0 2 3h12a2 2 0 0 0 2-3l-6-9V3M8 14h8"/></>,
  info: <><circle cx="12" cy="12" r="9"/><path d="M12 11v6m0-10h.01"/></>,
  spark: <><path d="m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5L12 3ZM20 2v4m-2-2h4"/></>,
  layout: <><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M8 3v18m0-13h13"/></>,
  crop: <><path d="M6 2v16h16M2 6h16v16"/></>,
  pin: <><path d="m8 3 8 0-1 6 4 4v2H5v-2l4-4-1-6Zm4 12v7"/></>,
  text: <path d="M5 5h14M12 5v14M8 19h8M5 5v3m14-3v3"/>,
  volume: <><path d="m11 4-6 5H2v6h3l6 5V4Zm4 4a6 6 0 0 1 0 8m3-11a10 10 0 0 1 0 14"/></>,
  copy: <><rect x="8" y="8" width="12" height="13" rx="2"/><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3"/></>,
  refresh: <><path d="M20 7a9 9 0 0 0-16 1M4 3v5h5m-5 9a9 9 0 0 0 16-1m0 5v-5h-5"/></>,
  edit: <><path d="m16 3 5 5-12 12-6 1 1-6L16 3Zm-3 3 5 5"/></>,
  trash: <><path d="M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7m4-7v7"/></>,
  eye: <><path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z"/><circle cx="12" cy="12" r="3"/></>,
  eyeOff: <><path d="m3 3 18 18M10 5c6-1 12 7 12 7s-1 2-3 4M6 6a21 21 0 0 0-4 6s4 7 10 7a11 11 0 0 0 5-1M9 9a4 4 0 0 0 6 6"/></>,
  lock: <><rect x="5" y="10" width="14" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3m-4 4v3"/></>,
  key: <><circle cx="8" cy="8" r="5"/><path d="m12 12 9 9m-4-4 3-3m-6 0 3-3"/></>,
  grip: <><circle cx="9" cy="5" r=".7"/><circle cx="15" cy="5" r=".7"/><circle cx="9" cy="12" r=".7"/><circle cx="15" cy="12" r=".7"/><circle cx="9" cy="19" r=".7"/><circle cx="15" cy="19" r=".7"/></>,
  swap: <><path d="M3 8h17m-4-4 4 4-4 4M21 16H4m4-4-4 4 4 4"/></>,
  upload: <><path d="M12 16V3m-5 5 5-5 5 5M3 15v6h18v-6"/></>,
  download: <><path d="M12 3v13m-5-5 5 5 5-5M3 17v4h18v-4"/></>,
  rectangle: <rect x="4" y="5" width="16" height="14" rx="1"/>,
  ellipse: <ellipse cx="12" cy="12" rx="9" ry="8"/>,
  arrow: <path d="M4 20 20 4M8 4h12v12"/>,
  pen: <><path d="M3 17c7-15 11-15 7-4s4 7 11-9M4 21h16"/></>,
  highlighter: <><path d="m14 3 7 7-9 9-7-7 9-9ZM5 12l-2 6 3 3 6-2M2 22h9"/></>,
  mosaic: <><rect x="3" y="3" width="18" height="18" rx="1"/><path d="M9 3v18m6-18v18M3 9h18M3 15h18"/><path d="M3 3h6v6H3Zm12 6h6v6h-6ZM9 15h6v6H9Z" fill="currentColor" stroke="none"/></>,
  number: <><circle cx="12" cy="12" r="9"/><path d="m9 9 3-2v10m-3 0h6M12 7v10"/></>,
  cursor: <path d="m4 3 6 18 3-8 8-3L4 3Z"/>,
  undo: <><path d="m8 4-5 5 5 5M3 9h11a7 7 0 0 1 0 14"/></>,
  redo: <><path d="m16 4 5 5-5 5m5-5H10a7 7 0 0 0 0 14"/></>,
  expand: <path d="M8 3H3v5m13-5h5v5M3 16v5h5m8 0h5v-5"/>,
  compare: <><rect x="2" y="4" width="8" height="16" rx="1.5"/><rect x="14" y="4" width="8" height="16" rx="1.5"/></>,
  circle: <circle cx="12" cy="12" r="8"/>,
  mail: <><rect x="3" y="5" width="18" height="14" rx="2"/><path d="m3 6 9 7 9-7"/></>,
  link: <><path d="m10 14 4-4m-5 6-2 2a4 4 0 0 1-6-6l5-5a4 4 0 0 1 6 0m0 10a4 4 0 0 0 6 0l5-5a4 4 0 0 0-6-6l-2 2"/></>,
  tag: <><path d="M3 3h8l10 10-8 8L3 11V3Z"/><circle cx="7" cy="7" r="1"/></>,
  warning: <><path d="m12 3 10 18H2L12 3Z"/><path d="M12 9v5m0 3h.01"/></>,
  checkCircle: <><circle cx="12" cy="12" r="9"/><path d="m7 12 3 3 7-7"/></>,
  book: <><path d="M12 5C8 2 3 3 3 3v17s5-1 9 2c4-3 9-2 9-2V3s-5-1-9 2Zm0 0v17"/></>,
  bolt: <path d="m13 2-9 12h7l-1 8 10-13h-7V2Z"/>,
  image: <><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8" cy="8" r="1.5"/><path d="m3 17 6-6 4 4 3-3 5 5"/></>,
};

export function Icon({ name, size = 18, className = '', ...props }) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" className={`icon ${className}`} {...props}>{paths[name] || paths.circle}</svg>;
}

export function Button({ children, icon, tone = 'default', small = false, className = '', ...props }) {
  return <button type="button" className={`button button-${tone} ${small ? 'button-small' : ''} ${className}`} {...props}>{icon && <Icon name={icon} size={15}/>}<span>{children}</span></button>;
}

export function IconButton({ icon, label, active = false, className = '', ...props }) {
  return <button type="button" aria-label={label} title={label} className={`icon-button ${active ? 'is-active' : ''} ${className}`} {...props}><Icon name={icon}/></button>;
}

export function Toggle({ checked, onChange, label, disabled = false }) {
  return <button type="button" role="switch" aria-checked={checked} aria-label={label} disabled={disabled} className={`toggle ${checked ? 'is-on' : ''}`} onClick={() => onChange(!checked)}><span><Icon name="check" size={10}/></span></button>;
}

export function Select({ value, options, onChange, label, className = '', ...props }) {
  return <div className={`select ${className}`}><select aria-label={label} value={value} onChange={event => onChange(event.target.value)} {...props}>{options.map(option => <option key={typeof option === 'string' ? option : option.value} value={typeof option === 'string' ? option : option.value}>{typeof option === 'string' ? option : option.label}</option>)}</select><Icon name="chevron" size={13}/></div>;
}

export function Tabs({ items, value, onChange, label, className = '' }) {
  return <div className={`tabs ${className}`} role="tablist" aria-label={label}>{items.map(item => {
    const id = typeof item === 'string' ? item : item.id;
    return <button type="button" key={id} role="tab" aria-selected={id === value} onClick={() => onChange(id)} className={id === value ? 'is-active' : ''}>{item.icon && <Icon name={item.icon} size={15}/>}<span>{typeof item === 'string' ? item : item.label}</span>{item.count !== undefined && <small>{item.count}</small>}</button>;
  })}</div>;
}

export function Keys({ value }) {
  return <span className="keys">{value ? value.split(' ').map((part, i) => <kbd key={i}>{part}</kbd>) : <span className="key-unset">点击设置</span>}</span>;
}

export function Badge({ children, tone = 'muted', dot = false, className = '' }) {
  return <span className={`badge badge-${tone} ${className}`}>{dot && <i/>}{children}</span>;
}

export function TrafficLights() {
  return <div className="traffic-lights" aria-hidden="true"><i/><i/><i/></div>;
}

export function Brand({ compact = false }) {
  return <div className={`brand ${compact ? 'compact' : ''}`}><img src="./app-icon.svg" alt="" width="32" height="32"/><div><strong>SnapLingo</strong><span>截图 · 识别 · 翻译</span></div></div>;
}

export function SaveStatus() {
  const { saving } = usePreview();
  return <span className={`save-status ${saving ? 'is-saving' : ''}`} role="status"><Icon name={saving ? 'refresh' : 'checkCircle'} size={14}/>{saving ? '正在保存' : '自动保存'}</span>;
}

export function Dialog({ title, description, children, footer, onClose, wide = false }) {
  const ref = useRef(null);
  useEffect(() => {
    const previous = document.activeElement;
    const focusable = ref.current?.querySelector('input, select, textarea, button');
    (focusable || ref.current)?.focus();
    const handler = event => {
      if (event.key === 'Escape') { event.preventDefault(); onClose(); }
      if (event.key !== 'Tab') return;
      const elements = [...(ref.current?.querySelectorAll('button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea, [tabindex="0"]') || [])];
      const first = elements[0], last = elements[elements.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
      if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    };
    document.addEventListener('keydown', handler);
    return () => { document.removeEventListener('keydown', handler); previous?.isConnected && previous.focus(); };
  }, []);
  return <div className="dialog-backdrop" onMouseDown={event => { if (event.target === event.currentTarget) onClose(); }}><section ref={ref} className={`dialog ${wide ? 'dialog-wide' : ''}`} role="dialog" aria-modal="true" aria-labelledby="dialog-title" tabIndex={-1}><header><div><h2 id="dialog-title">{title}</h2>{description && <p>{description}</p>}</div><IconButton icon="close" label="关闭弹窗" onClick={onClose}/></header><div className="dialog-body">{children}</div>{footer && <footer>{footer}</footer>}</section></div>;
}

export function DocumentSample({ compact = false, className = '' }) {
  return <article className={`document-sample ${compact ? 'document-compact' : ''} ${className}`}><div className="document-overline"><span>NOTES ON DESIGN</span><span>№ 10</span></div><div className="document-main"><span className="document-rule"/><h2>Less,<br/><em>but better.</em></h2><p>Good design is as little design as possible.</p><p className="document-body">It concentrates on the essential aspects,<br/>and the products are not burdened<br/>with non-essentials.</p></div><footer><span>DIETER RAMS</span><span>Ten principles for good design</span></footer></article>;
}

export function EmptyState({ icon = 'search', title, description, action }) {
  return <div className="empty-state"><div className="empty-icon"><Icon name={icon} size={27}/></div><h3>{title}</h3>{description && <p>{description}</p>}{action}</div>;
}

export function Skeleton({ lines = 3 }) {
  return <div className="skeleton" aria-label="加载中" role="status">{Array.from({ length: lines }, (_, index) => <i key={index} style={{ width: `${[94, 85, 63, 78][index % 4]}%` }}/>)}</div>;
}
