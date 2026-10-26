import React, { useEffect, useRef, useState } from 'react';
import { navigation, settingsPages, defaultSettings } from './data.js';
import { Badge, Brand, Button, Dialog, Icon, IconButton, Keys, SaveStatus, Select, Toggle, usePreview } from './ui.jsx';

export function SettingsPage({ page }) {
  const { setScene, section, setSection, notify, setDialog, setSettings, miniature } = usePreview();
  const pageRef = useRef(null);
  const info = navigation.find(item => item.id === page);
  const sections = settingsPages[page];
  useEffect(() => {
    if (miniature) return;
    const root = pageRef.current?.closest('.workspace-main');
    if (!root) return;
    root.scrollTo({ top: 0 });
    const trackSection = () => {
      const threshold = root.getBoundingClientRect().top + 115;
      const elements = [...pageRef.current.querySelectorAll('.settings-section')];
      const visible = elements.filter(element => element.getBoundingClientRect().top <= threshold);
      const active = visible[visible.length - 1] || elements[0];
      if (active) setSection(active.id.replace(`section-${page}-`, ''));
    };
    root.addEventListener('scroll', trackSection, { passive: true });
    trackSection();
    return () => root.removeEventListener('scroll', trackSection);
  }, [page, miniature]);
  const jumpTo = id => {
    setSection(id);
    document.getElementById(`section-${page}-${id}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  };
  return <div ref={pageRef} className={`settings-page page-${page}`} data-screen-label={`${info.label}设置`}>
    <header className="page-header"><div><div className="page-eyebrow"><span>偏好设置</span><span className="eyebrow-divider">/</span><span>{info.english}</span></div><h1>{info.label}<span className="paper-title-dot">.</span></h1><p>{info.description}</p></div><div className="page-header-action">{page !== 'general' && <Button icon={page === 'translation' ? 'languages' : page === 'ocr' ? 'ocr' : 'scan'} onClick={() => setScene({ translation: 'translate', screenshot: 'capture', ocr: 'recognize' }[page])}>{page === 'translation' ? '打开翻译窗口' : page === 'ocr' ? '试试文字识别' : '开始截图'}</Button>}</div></header>
    <div className="section-nav-wrap"><nav className="section-nav" aria-label="设置分组">{sections.map(item => <button key={item.id} type="button" className={section === item.id ? 'is-active' : ''} onClick={() => jumpTo(item.id)}>{item.label}</button>)}</nav><SaveStatus/></div>
    <div className="settings-sections">{sections.map((item, index) => <section className="settings-section" id={miniature ? undefined : `section-${page}-${item.id}`} key={item.id}>
      <header className="section-heading"><div className="section-title"><span className="section-icon"><Icon name={item.icon}/></span><span className="section-number">{String(index + 1).padStart(2, '0')}</span><div><h2>{item.label}</h2>{item.description && <p>{item.description}</p>}</div></div>{item.id === 'hotkeys' && <div className="section-actions"><Button small tone="quiet" onClick={() => setDialog({ type: 'confirm', title: '恢复这组快捷键？', description: '这组快捷键将恢复到预览的默认组合。', confirmText: '恢复默认值', onConfirm: () => { setSettings(current => ({ ...current, ...Object.fromEntries(item.rows.map(row => [row.id, defaultSettings[row.id]])) })); notify('这组快捷键已恢复默认值'); } })}>恢复默认</Button><Button small icon="shield" onClick={() => setDialog({ type: 'hotkey-check' })}>检测冲突</Button></div>}</header>
      <div className="section-body">{item.type === 'appearance' && <AppearancePicker/>}{item.type === 'permissions' && <Permissions/>}{item.type === 'about' && <About/>}{item.rows.map(row => <SettingRow key={row.id} row={row}/>)}{item.type === 'maintenance' && <Maintenance/>}{item.type === 'about' && <UpdateControls/>}</div>
    </section>)}</div>
    <footer className="settings-footer"><Icon name="lock" size={13}/><span>你的偏好设置，保存在本机。</span><span className="footer-wordmark">SnapLingo</span></footer>
  </div>;
}

export function RangeControl({ value, onChange, min, max, unit = '', label }) {
  return <div className="range-control"><input type="range" aria-label={label} min={min} max={max} step={max > 100 ? 10 : 1} value={value} onChange={event => onChange(Number(event.target.value))} style={{ '--progress': `${(value - min) / (max - min) * 100}%` }}/><output>{value}<span>{unit}</span></output></div>;
}

export function NumberControl({ value, onChange, min = 0, max = 100000, unit = '', label }) {
  return <div className="number-control"><input type="number" aria-label={label} value={value} min={min} max={max} onChange={event => { if (event.target.value !== '') onChange(Math.min(max, Math.max(min, Number(event.target.value)))); }}/><span>{unit}</span><div><button type="button" aria-label={`增加${label}`} disabled={value >= max} onClick={() => onChange(Math.min(max, value + 1))}><Icon name="plus" size={11}/></button><button type="button" aria-label={`减少${label}`} disabled={value <= min} onClick={() => onChange(Math.max(min, value - 1))}><Icon name="minus" size={11}/></button></div></div>;
}

function SettingRow({ row }) {
  const { settings, updateSetting, setDialog } = usePreview();
  const value = settings[row.id] ?? row.value;
  if (row.id === 'proxyUrl' && settings.proxy !== '手动配置') return null;
  if (row.id === 'filePrefix' && settings.naming !== '自定义') return null;
  let control;
  if (row.type === 'toggle') control = <Toggle checked={value} onChange={next => updateSetting(row.id, next)} label={row.label}/>;
  if (row.type === 'select') control = <Select value={value} onChange={next => updateSetting(row.id, next)} options={row.options} label={row.label}/>;
  if (row.type === 'text') control = <div className="text-control"><input aria-label={row.label} value={value} onChange={event => updateSetting(row.id, event.target.value)} spellCheck={false}/>{row.id === 'savePath' && <IconButton icon="folder" label="选择保存位置" onClick={() => setDialog({ type: 'directory' })}/>}</div>;
  if (row.type === 'number') control = <NumberControl {...row} value={value} onChange={next => updateSetting(row.id, next)}/>;
  if (row.type === 'range') control = <RangeControl {...row} value={value} onChange={next => updateSetting(row.id, next)}/>;
  if (row.type === 'shortcut') control = <HotkeyControl row={row} value={value}/>;
  if (row.type === 'color') control = <div className="color-choices">{['#3975dd', '#dd6760', '#4c8c66', '#dba746', '#ffffff', '#1d2531'].map(color => <button type="button" key={color} style={{ background: color }} className={color === value ? 'is-active' : ''} aria-label={`${row.label} ${color}`} title={color} onClick={() => updateSetting(row.id, color)}>{color === value && <Icon name="check" size={12}/>}</button>)}<label className="custom-color"><input type="color" value={value} onChange={event => updateSetting(row.id, event.target.value)} aria-label={`自定义${row.label}`}/><Icon name="plus" size={13}/></label></div>;
  return <div className={`setting-row row-${row.type}`}><div className="row-label"><h3>{row.label}</h3>{row.description && <p>{row.description}</p>}</div><div className="row-control">{control}</div></div>;
}

function AppearancePicker() {
  const { appearance, setAppearance } = usePreview();
  return <div className="appearance-picker"><div className="appearance-copy"><h3>喜欢的样子</h3><p>选择一种适合当下的外观。</p></div><div className="appearance-options">{[{ id: 'light', icon: 'sun', label: '浅色' }, { id: 'dark', icon: 'moon', label: '深色' }, { id: 'system', icon: 'monitor', label: '跟随系统' }].map(item => <button type="button" key={item.id} aria-pressed={appearance === item.id} className={`appearance-option ${appearance === item.id ? 'is-active' : ''}`} onClick={() => setAppearance(item.id)}><span className={`appearance-mini mini-${item.id}`}><span className="appearance-mini-sidebar"><i/><i/><i/></span><span className="appearance-mini-content"><i/><i/><i/></span>{appearance === item.id && <span className="appearance-selected"><Icon name="check" size={10}/></span>}</span><span><Icon name={item.icon} size={13}/>{item.label}</span></button>)}</div></div>;
}

function Permissions() {
  const { permissions, setDialog } = usePreview();
  return <>{[{ id: 'screen', icon: 'scan', title: '屏幕录制', description: '用于截图，以及识别和翻译屏幕上的内容。' }, { id: 'accessibility', icon: 'cursor', title: '辅助功能', description: '用于从其他应用中获取选中的文字。' }].map(item => <div className="setting-row permission-row" key={item.id}><span className="permission-icon"><Icon name={item.icon} size={20}/></span><div className="row-label"><h3>{item.title}</h3><p>{item.description}</p></div><div className="row-control">{permissions[item.id] ? <Badge tone="good" dot>已授权</Badge> : <Button small onClick={() => setDialog({ type: 'permission', permission: item.id, title: item.title })}>去授权<Icon name="arrowRight" size={12}/></Button>}</div></div>)}</>;
}

function About() {
  return <div className="about-app"><Brand/><Badge>v0.1.7</Badge><p>捕捉灵感，读懂世界。</p><span>一个轻巧的截图、文字识别与翻译工具。</span><small>开源许可 · MIT License</small></div>;
}

function UpdateControls() {
  const { notify } = usePreview();
  const [checking, setChecking] = useState(false);
  return <div className="setting-row"><div className="row-label"><h3>软件更新</h3><p>当前版本 v0.1.7</p></div><Button small icon="refresh" disabled={checking} onClick={() => { setChecking(true); setTimeout(() => { setChecking(false); notify('这是更新检查的交互预览，未连接更新服务器'); }, 900); }}>{checking ? '正在检查…' : '检查更新'}</Button></div>;
}

function Maintenance() {
  const { setDialog, notify, setRecords } = usePreview();
  const [logs, setLogs] = useState(true);
  return <><div className="setting-row"><div className="row-label"><h3>应用日志</h3><p>查看最近的运行信息。</p></div><div className="inline-actions"><Button small onClick={() => { setLogs(true); notify('已刷新预览日志'); }}>刷新</Button><Button small tone="danger-quiet" onClick={() => setDialog({ type: 'confirm', title: '清空日志？', description: '将清空本次预览显示的日志。', danger: true, onConfirm: () => { setLogs(false); notify('预览日志已清空'); } })}>清空</Button></div></div><div className="log-preview">{logs ? <><span><i>INFO</i> Settings loaded from local storage</span><span><i>INFO</i> Application ready</span></> : <span>暂无日志</span>}</div><div className="setting-row"><div className="row-label"><h3>历史记录</h3><p>清空历史记录后，独立收藏仍会保留。</p></div><Button small tone="danger-quiet" onClick={() => setDialog({ type: 'confirm', title: '清空全部历史记录？', description: '将清空预览历史，已收藏的内容会继续保留。', danger: true, onConfirm: () => { setRecords(current => current.map(item => ({ ...item, historyDeleted: true }))); notify('历史记录已清空'); } })}>清空历史记录</Button></div></>;
}

export function SettingsDialogs() {
  const context = usePreview();
  const { dialog, setDialog, settings, updateSetting, setSettings, notify, permissions, setPermissions } = context;
  const close = () => setDialog(null);
  if (!dialog) return null;
  if (dialog.type === 'directory') return <DirectoryDialog/>;
  if (dialog.type === 'capacity') return <CapacityDialog/>;
  if (dialog.type === 'permission') return <Dialog title={`开启${dialog.title}`} description="在 macOS「系统设置 → 隐私与安全性」中允许 SnapLingo。" onClose={close} footer={<><Button onClick={close}>稍后</Button><Button tone="primary" icon="check" onClick={() => { setPermissions({ ...permissions, [dialog.permission]: true }); notify(`已展示${dialog.title}授权完成状态`); close(); }}>模拟授权完成</Button></>}><div className="permission-instructions"><div><span>1</span><p>在列表中找到 <strong>SnapLingo</strong></p></div><div><span>2</span><p>打开对应的权限开关</p></div><div><span>3</span><p>返回应用，继续使用</p></div></div><div className="inline-note"><Icon name="info" size={15}/>这是设计预览，不会修改系统权限。</div></Dialog>;
  if (dialog.type === 'confirm') return <Dialog title={dialog.title} description={dialog.description} onClose={close} footer={<><Button onClick={close}>取消</Button><Button tone={dialog.danger ? 'danger' : 'primary'} onClick={() => { dialog.onConfirm?.(); close(); }}>{dialog.confirmText || '确定清空'}</Button></>}><div className="confirm-symbol"><Icon name={dialog.danger ? 'trash' : 'refresh'} size={25}/></div></Dialog>;
  if (dialog.type === 'hotkey-check') {
    const values = Object.values(settingsPages).flat().flatMap(section => section.rows).filter(row => row.type === 'shortcut').map(row => ({ label: row.label, value: settings[row.id] })).filter(row => row.value);
    const conflicts = values.filter((row, index) => values.some((other, otherIndex) => index !== otherIndex && row.value === other.value));
    return <Dialog title={conflicts.length ? '发现快捷键冲突' : '快捷键没有冲突'} description={conflicts.length ? '请为下面的操作设置不同的组合。' : '每个操作都有自己的组合，可以安心使用。'} onClose={close} footer={<Button tone="primary" onClick={close}>知道了</Button>}><div className={`conflict-result ${conflicts.length ? 'has-conflict' : ''}`}><Icon name={conflicts.length ? 'warning' : 'checkCircle'} size={38}/>{conflicts.map(row => <div key={row.label}>{row.label}<Keys value={row.value}/></div>)}</div></Dialog>;
  }
  return null;
}

function HotkeyControl({ row, value }) {
  const { updateSetting } = usePreview();
  const [recording, setRecording] = useState(false);
  const pending = useRef(null);
  const cancel = () => { pending.current = null; setRecording(false); };
  const isWindows = navigator.platform.startsWith('Win');
  const normalized = (value || '').replace(/\s/g, '').replace('Ctrl', '⌃');
  const hasValue = Boolean(value) && value !== '未设置';
  const isModified = value !== defaultSettings[row.id];
  const modifiers = [
    { symbol: '⇧', active: normalized.includes('⇧') },
    { symbol: '⌥', active: normalized.includes('⌥') },
    { symbol: isWindows ? 'Ctrl' : '⌘', active: isWindows ? /[⌘⌃]/.test(normalized) : normalized.includes('⌘') },
    ...(isWindows ? [] : [{ symbol: '⌃', active: normalized.includes('⌃') }]),
  ];

  useEffect(() => {
    if (!recording) return;
    window.addEventListener('blur', cancel);
    return () => window.removeEventListener('blur', cancel);
  }, [recording]);

  const recordKeyDown = event => {
    if (!recording) return;
    event.preventDefault(); event.stopPropagation();
    if (event.key === 'Escape') { cancel(); return; }
    if (event.repeat) return;
    const code = event.code || event.key;
    const key = code.startsWith('Key') ? code.slice(3) : code.startsWith('Digit') ? code.slice(5) : /^F\d+$/.test(code) ? code : '';
    if (!key) return;
    pending.current = {
      code,
      value: [event.shiftKey && '⇧', event.altKey && '⌥', (isWindows ? event.ctrlKey : event.metaKey) && '⌘', !isWindows && event.ctrlKey && '⌃', key].filter(Boolean).join(' '),
    };
  };
  const recordKeyUp = event => {
    if (!recording || !pending.current || (event.code || event.key) !== pending.current.code) return;
    event.preventDefault(); event.stopPropagation();
    updateSetting(row.id, pending.current.value);
    cancel();
  };

  return <div className="shortcut-controls">
    <button className={`shortcut-control ${recording ? 'is-recording' : !hasValue ? 'is-unset' : ''}`} type="button" aria-label={`修改${row.label}快捷键`} aria-pressed={recording} title={recording ? '松开按键后自动保存，Esc 取消' : '点击录制快捷键'} onClick={event => { event.currentTarget.focus(); pending.current = null; setRecording(current => !current); }} onBlur={cancel} onKeyDown={recordKeyDown} onKeyUp={recordKeyUp}>
      {modifiers.map(item => <span key={item.symbol} className={`shortcut-modifier ${hasValue && !recording && item.active ? 'is-set' : ''}`}>{item.symbol}</span>)}
      {recording || !hasValue ? <span className="shortcut-hint">{recording ? '按下快捷键...' : '未设置'}</span> : <span className="shortcut-letter">{normalized.replace(/[⇧⌥⌘⌃]/g, '')}</span>}
    </button>
    <div className="shortcut-actions">
      <IconButton icon="refresh" label={`恢复${row.label}默认快捷键`} title={isModified ? '恢复默认值' : '已是默认值'} disabled={!isModified} onClick={() => { cancel(); updateSetting(row.id, defaultSettings[row.id]); }}/>
      <IconButton icon="close" label={`清除${row.label}快捷键`} title={hasValue ? '清除快捷键' : '未设置快捷键'} className="shortcut-clear" disabled={!hasValue} onClick={() => { cancel(); updateSetting(row.id, ''); }}/>
    </div>
  </div>;
}

function DirectoryDialog() {
  const { settings, updateSetting, setDialog } = usePreview();
  const [path, setPath] = useState(settings.savePath);
  return <Dialog title="选择保存位置" description="为截图设置默认目录。" onClose={() => setDialog(null)} footer={<><Button onClick={() => setDialog(null)}>取消</Button><Button tone="primary" disabled={!path.trim()} onClick={() => { updateSetting('savePath', path.trim()); setDialog(null); }}>使用此位置</Button></>}><label className="form-field"><span>文件夹路径</span><input value={path} onChange={event => setPath(event.target.value)}/></label><div className="folder-presets">{['~/Pictures/SnapLingo', '~/Desktop', '~/Downloads'].map(item => <button type="button" key={item} onClick={() => setPath(item)}><Icon name="folder" size={18}/>{item}<Icon name="chevronRight" size={14}/></button>)}</div></Dialog>;
}

function CapacityDialog() {
  const { settings, setSettings, setDialog, notify, records } = usePreview();
  const [draft, setDraft] = useState({ autoCleanup: settings.autoCleanup, historyDays: settings.historyDays, historyCapacity: settings.historyCapacity, favoriteCapacity: settings.favoriteCapacity });
  const update = (id, value) => setDraft(current => ({ ...current, [id]: value }));
  return <Dialog title="资料库容量" description="管理历史记录与收藏，让资料库保持轻盈。" onClose={() => setDialog(null)} footer={<><Button onClick={() => setDialog(null)}>取消</Button><Button tone="primary" onClick={() => { setSettings(current => ({ ...current, ...draft })); setDialog(null); notify('资料库设置已保存'); }}>保存设置</Button></>}><div className="capacity-overview"><span><strong>{records.filter(item => item.kind !== 'screenshot' && !item.historyDeleted).length}</strong>条历史记录</span><span><strong>{records.filter(item => item.favorite).length}</strong>项收藏</span></div><div className="setting-row"><div className="row-label"><h3>自动清理历史记录</h3><p>保留近期内容，收藏不受影响。</p></div><Toggle label="自动清理历史记录" checked={draft.autoCleanup} onChange={value => update('autoCleanup', value)}/></div>{[{ id: 'historyDays', label: '历史保留天数', min: 1, max: 365, unit: '天' }, { id: 'historyCapacity', label: '历史记录上限', min: 100, max: 100000, unit: '条' }, { id: 'favoriteCapacity', label: '收藏上限', min: 100, max: 100000, unit: '项' }].map(row => <div className="setting-row" key={row.id}><div className="row-label"><h3>{row.label}</h3></div><NumberControl {...row} value={draft[row.id]} onChange={value => update(row.id, value)}/></div>)}<div className="inline-note"><Icon name="bookmark" size={15}/>收藏独立保存，不随历史记录自动清理。</div></Dialog>;
}
