import React, { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { defaultSettings, directions, initialProviders, initialRecords, navigation, sampleOcrText, scenes, settingsPages, sourceText } from './data.js';
import { Badge, Brand, Button, Icon, IconButton, PreviewContext, SaveStatus, Select, TrafficLights, usePreview } from './ui.jsx';
import { SettingsDialogs, SettingsPage } from './settings.jsx';
import { ServicesPage } from './services.jsx';
import { LibraryPage } from './library.jsx';
import { PermissionScene, ResultScene } from './results.jsx';
import { CaptureScene, PinnedScene, copyDemoImage, downloadDemoImage } from './capture.jsx';

const storageKey = 'snaplingo-ui-refresh-v1';
const parameters = new URLSearchParams(window.location.search);
function readSaved() {
  try { return JSON.parse(localStorage.getItem(storageKey) || '{}'); } catch { return {}; }
}
const saved = readSaved();
const initialSettings = { ...defaultSettings, historyDays: 30, historyCapacity: 5000, favoriteCapacity: 1000, autoCleanup: true, ...saved.settings };
const isOneOf = (value, list) => list.some(item => item.id === value);

function App() {
  const [direction, setDirection] = useState(isOneOf(parameters.get('scheme'), directions) ? parameters.get('scheme') : saved.direction || 'native');
  const [scene, changeScene] = useState(isOneOf(parameters.get('scene'), scenes) ? parameters.get('scene') : 'workspace');
  const [page, changePage] = useState(isOneOf(parameters.get('page'), navigation) ? parameters.get('page') : 'translation');
  const [comparison, setComparison] = useState(parameters.get('view') === 'compare');
  const [chosen, setChosen] = useState(saved.chosen || null);
  const [settings, setSettings] = useState(initialSettings);
  const [appearances, setAppearances] = useState(saved.appearances || { native: 'light', studio: 'dark', paper: 'light' });
  const [prefersDark, setPrefersDark] = useState(window.matchMedia('(prefers-color-scheme: dark)').matches);
  const [providers, setProviders] = useState(saved.providers || initialProviders);
  const [records, setRecords] = useState(saved.records || initialRecords);
  const [permissions, setPermissions] = useState(saved.permissions || { screen: false, accessibility: false });
  const [serviceTab, setServiceTab] = useState('translation');
  const [section, setSection] = useState('hotkeys');
  const [source, setSource] = useState(sourceText);
  const [pinnedAnnotations, setPinnedAnnotations] = useState([]);
  const [screenState, setScreenState] = useState(['ready', 'loading', 'empty', 'error'].includes(parameters.get('state')) ? parameters.get('state') : 'ready');
  const [dialog, setDialog] = useState(null);
  const [toast, setToast] = useState('');
  const [saving, setSaving] = useState(false);
  const toastTimer = useRef(null), saveTimer = useRef(null);
  const appearance = appearances[direction] || 'light';
  const effectiveAppearance = appearance === 'system' ? prefersDark ? 'dark' : 'light' : appearance;
  const notify = message => { setToast(message); clearTimeout(toastTimer.current); toastTimer.current = setTimeout(() => setToast(''), 3000); };
  const setPage = next => { changePage(next); setSection(settingsPages[next]?.[0]?.id || ''); setScreenState('ready'); };
  const setScene = next => { changeScene(next); setScreenState('ready'); setDialog(null); setComparison(false); };
  const updateSetting = (id, value) => { setSettings(current => ({ ...current, [id]: value })); setSaving(true); clearTimeout(saveTimer.current); saveTimer.current = setTimeout(() => setSaving(false), 550); };
  const setAppearance = value => { setAppearances(current => ({ ...current, [direction]: value })); notify('预览外观已更新'); };
  const copyText = async text => { try { await navigator.clipboard.writeText(text); notify('已复制到剪贴板'); } catch { notify('浏览器未允许复制，可选中文字手动复制'); } };
  const copyImage = async (annotations, rotation = 0) => { try { await copyDemoImage(annotations, rotation); notify('截图已复制到剪贴板'); } catch { notify('浏览器不支持复制图片，可以使用保存按钮导出'); } };
  const downloadImage = async (annotations, rotation = 0) => { await downloadDemoImage(annotations, rotation); notify('已导出演示截图'); };
  const openPin = annotations => { setPinnedAnnotations(annotations); setScene('pin'); };
  const speak = (text, language) => {
    if (!('speechSynthesis' in window)) { notify('当前浏览器未提供系统朗读'); return; }
    window.speechSynthesis.cancel();
    const utterance = new SpeechSynthesisUtterance(text);
    utterance.lang = language || (/[\u4e00-\u9fff]/.test(text) ? 'zh-CN' : 'en-US');
    utterance.rate = (settings.voiceRate || 180) / 180;
    utterance.onstart = () => notify('正在朗读'); utterance.onerror = () => notify('当前浏览器未能启动系统语音');
    window.speechSynthesis.speak(utterance);
  };
  const favoriteResult = ({ source: original, text, provider, kind = 'translation' }) => {
    setRecords(items => {
      const existing = items.find(item => item.source === original && item.provider === provider && item.kind === kind);
      if (existing) return items.map(item => item.id === existing.id ? { ...item, favorite: true } : item);
      return [{ id: `result-${Date.now()}`, kind, source: original, result: text, provider, title: original.split(/[.。\n]/)[0].slice(0, 65), time: '刚刚', day: '今天', favorite: true, tags: [], note: '' }, ...items];
    }); notify('已保存到收藏夹');
  };
  const favoriteScreenshot = annotations => { setRecords(items => [{ id: `capture-${Date.now()}`, kind: 'screenshot', title: '阅读摘录 · 截图标注', source: sampleOcrText, result: '960 × 600 · PNG', provider: '截图', time: '刚刚', day: '今天', favorite: true, tags: [], note: '', annotations }, ...items]); notify('截图已加入收藏夹'); };
  useEffect(() => {
    try { localStorage.setItem(storageKey, JSON.stringify({ direction, chosen, settings, appearances, providers, records, permissions })); } catch { /* The preview remains usable when browser storage is unavailable. */ }
  }, [direction, chosen, settings, appearances, providers, records, permissions]);
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const change = event => setPrefersDark(event.matches); media.addEventListener('change', change);
    return () => media.removeEventListener('change', change);
  }, []);
  useEffect(() => {
    const url = new URL(window.location.href);
    url.searchParams.set('scheme', direction); url.searchParams.set('scene', scene); url.searchParams.set('page', page);
    comparison ? url.searchParams.set('view', 'compare') : url.searchParams.delete('view');
    screenState === 'ready' ? url.searchParams.delete('state') : url.searchParams.set('state', screenState);
    window.history.replaceState({}, '', url);
  }, [direction, scene, page, comparison, screenState]);
  useEffect(() => {
    const onKey = event => { if (event.key === 'Escape' && !dialog && scene !== 'workspace') setScene('workspace'); };
    window.addEventListener('keydown', onKey); return () => window.removeEventListener('keydown', onKey);
  }, [dialog, scene]);
  const context = { direction, setDirection, scene, setScene, page, setPage, comparison, setComparison, chosen, setChosen, settings, setSettings, updateSetting, appearance, setAppearance, effectiveAppearance, providers, setProviders, records, setRecords, permissions, setPermissions, serviceTab, setServiceTab, section, setSection, source, setSource, pinnedAnnotations, setPinnedAnnotations, screenState, setScreenState, dialog, setDialog, saving, notify, copyText, copyImage, downloadImage, openPin, speak, favoriteResult, favoriteScreenshot };
  const info = directions.find(item => item.id === direction) || directions[0];
  const showStates = ['translate', 'recognize'].includes(scene) || scene === 'workspace' && ['history', 'favorites'].includes(page);
  return <PreviewContext.Provider value={context}><div className="preview-page"><header className="preview-header"><a className="preview-wordmark" href="./index.html" aria-label="SnapLingo 设计提案首页"><span className="preview-logo"><Icon name="scan" size={17}/></span><span><strong>SnapLingo</strong><small>UI DESIGN EXPLORATION</small></span></a><div className="direction-switcher" role="tablist" aria-label="设计方案">{directions.map(item => <button type="button" key={item.id} role="tab" aria-selected={direction === item.id && !comparison} className={direction === item.id && !comparison ? 'is-active' : ''} onClick={() => { setDirection(item.id); setComparison(false); }}><span className={`direction-swatch swatch-${item.id}`}/><span>{item.letter}<span className="direction-name"> · {item.name}</span></span>{chosen === item.id && <Icon name="check" size={11}/>}</button>)}</div><div className="preview-header-actions"><Button small icon="compare" tone={comparison ? 'primary' : 'quiet'} onClick={() => setComparison(value => !value)}>{comparison ? '返回预览' : '并排对比'}</Button><span className="preview-local"><i/>交互预览</span></div></header>
    {!comparison && <div className="preview-scene-bar"><nav aria-label="预览场景">{scenes.map(item => <button type="button" key={item.id} className={scene === item.id ? 'is-active' : ''} onClick={() => setScene(item.id)}><Icon name={item.icon} size={14}/>{item.label}</button>)}</nav><div className="preview-scene-options">{showStates && <Select label="展示状态" value={screenState} onChange={setScreenState} options={[{ value: 'ready', label: '正常状态' }, { value: 'loading', label: '加载中' }, { value: 'empty', label: '空白状态' }, { value: 'error', label: '失败状态' }]}/>}<button type="button" className={`choose-direction ${chosen === direction ? 'is-chosen' : ''}`} onClick={() => { setChosen(chosen === direction ? null : direction); notify(chosen === direction ? '已取消首选标记' : `已标记 ${info.letter} · ${info.name} 为首选`); }}><Icon name={chosen === direction ? 'checkCircle' : 'bookmark'} size={14}/>{chosen === direction ? '已标记首选' : '标记为首选'}</button></div></div>}
    <main className={comparison ? 'comparison-stage' : `design-stage stage-${direction} stage-scene-${scene}`}>{comparison ? <Comparison/> : <div className={`theme-surface theme-${direction}`} data-theme={direction} data-appearance={effectiveAppearance}>{scene === 'workspace' ? <Workspace/> : ['translate', 'recognize'].includes(scene) ? <ResultScene key={scene}/> : scene === 'capture' ? <CaptureScene/> : scene === 'pin' ? <PinnedScene/> : <PermissionScene/>}<SettingsDialogs/>{toast && <div className="toast" role="status"><Icon name="checkCircle" size={17}/>{toast}</div>}</div>}</main>
    <footer className="preview-footer"><span><strong>{comparison ? '三种新日常' : `${info.letter} / ${info.name}`}</strong><i/>{comparison ? '同一组功能，三种截然不同的体验。' : info.description}</span><span>演示数据 · 设置仅保存在本页{!comparison && <button type="button" className="reset-preview" onClick={() => setDialog({ type: 'confirm', title: '重置演示内容？', description: '恢复本页的示例设置、服务与资料库，保留当前设计方向。', confirmText: '重置演示', onConfirm: () => { setSettings({ ...defaultSettings, historyDays: 30, historyCapacity: 5000, favoriteCapacity: 1000, autoCleanup: true }); setProviders(initialProviders); setRecords(initialRecords); setSource(sourceText); setPermissions({ screen: false, accessibility: false }); setAppearances({ native: 'light', studio: 'dark', paper: 'light' }); setScreenState('ready'); setPinnedAnnotations([]); notify('演示内容已重置'); } })}>重置演示</button>}</span></footer></div></PreviewContext.Provider>;
}

function Workspace({ miniature = false, forcedPage }) {
  const context = usePreview();
  const page = forcedPage || context.page;
  const info = navigation.find(item => item.id === page);
  return <section className={`app-window ${miniature ? 'is-miniature' : ''}`} data-screen-label="SnapLingo 设置窗口"><header className="window-chrome"><TrafficLights/><span>SnapLingo <span className="chrome-separator">/</span> {info.label}</span><div className="spacer"/><span className="window-chrome-meta"><span className="online-dot"/>本机保存</span><Icon name="layout" size={14}/></header><div className="workspace-body"><MainNavigation page={page}/>{(settingsPages[page] || page === 'services') && <ContextNavigation page={page}/>}<main className={`workspace-main ${['favorites', 'history'].includes(page) ? 'workspace-library' : ''}`}>{page === 'services' ? <ServicesPage/> : ['favorites', 'history'].includes(page) ? <LibraryPage key={page} page={page}/> : <SettingsPage page={page}/>}</main></div></section>;
}

function MainNavigation({ page }) {
  const { setPage, setDialog, records, settings, direction } = usePreview();
  const historyCount = records.filter(item => item.kind !== 'screenshot' && !item.historyDeleted).length;
  const favoritesCount = records.filter(item => item.favorite).length;
  const group = (items, label) => <div className="nav-group"><div className="nav-group-label">{label}</div><div>{items.map(item => <button type="button" key={item.id} onClick={() => setPage(item.id)} className={`nav-item ${page === item.id ? 'is-active' : ''}`} aria-current={page === item.id ? 'page' : undefined} title={item.label}><span className="nav-item-icon"><Icon name={item.icon} size={19}/></span><span>{item.label}</span>{['favorites', 'history'].includes(item.id) && <small>{item.id === 'favorites' ? favoritesCount : historyCount}</small>}{page === item.id && <i className="nav-active-dot"/>}</button>)}</div></div>;
  return <aside className="main-sidebar"><Brand/><nav aria-label="主导航">{group(navigation.slice(0, 5), '工作偏好')}{group(navigation.slice(5), '我的资料库')}</nav><button type="button" className="sidebar-storage" onClick={() => setDialog({ type: 'capacity' })} aria-label="管理资料库容量"><span className="storage-heading"><Icon name="layers" size={15}/><strong>本地资料库</strong><Icon name="chevronRight" size={12}/></span><span className="storage-line"><span>历史记录</span><span>{historyCount} <small>/ {settings.historyCapacity.toLocaleString()}</small></span></span><span className="storage-meter"><i style={{ width: `${Math.max(2, Math.min(100, historyCount / settings.historyCapacity * 100))}%` }}/></span><span className="storage-line"><span>收藏</span><span>{favoritesCount} <small>/ {settings.favoriteCapacity.toLocaleString()}</small></span></span><span className="storage-meter"><i style={{ width: `${Math.max(2, Math.min(100, favoritesCount / settings.favoriteCapacity * 100))}%` }}/></span><span className="storage-retention">历史保留 {settings.historyDays} 天</span></button><footer className="sidebar-footer"><span><Icon name="shield" size={12}/>本机保存</span><span>v0.1.7</span></footer></aside>;
}

function ContextNavigation({ page }) {
  const { section, setSection, serviceTab, setServiceTab } = usePreview();
  const sections = settingsPages[page] || [{ id: 'translation', label: '翻译服务', icon: 'languages' }, { id: 'ocr', label: '文字识别', icon: 'ocr' }, { id: 'tts', label: '语音合成', icon: 'volume' }];
  return <aside className="context-sidebar"><div className="context-title">{navigation.find(item => item.id === page).label}<span>CONTENTS</span></div><nav aria-label="页面目录">{sections.map((item, index) => <button type="button" key={item.id} className={(page === 'services' ? serviceTab : section) === item.id ? 'is-active' : ''} onClick={() => { if (page === 'services') setServiceTab(item.id); else { setSection(item.id); document.getElementById(`section-${page}-${item.id}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' }); } }}><span>{String(index + 1).padStart(2, '0')}</span>{item.label}<Icon name="chevronRight" size={12}/></button>)}</nav><div className="spacer"/><SaveStatus/></aside>;
}

function Comparison() {
  const context = usePreview();
  const { setDirection, setComparison, setChosen, chosen } = context;
  return <div className="comparison-content"><header className="comparison-heading"><span>SNAPLINGO · THREE DIRECTIONS</span><h1>同样顺手，各有性格。</h1><p>三套完整的界面方向。打开任意方案，体验设置、翻译、截图与资料库。</p></header><div className="comparison-grid">{directions.map(item => <article key={item.id} className={`comparison-card comparison-${item.id}`}><div className="comparison-card-title"><span>{item.letter}</span><div><h2>{item.name}<small>{item.english}</small></h2><p>{item.tone}</p></div>{chosen === item.id && <Badge tone="good">首选</Badge>}</div><Miniature direction={item.id} context={context}/><div className="comparison-details"><p>{item.detail}</p><dl><div><dt>导航</dt><dd>{item.layout}</dd></div><div><dt>密度</dt><dd>{item.density}</dd></div><div><dt>字体</dt><dd>{item.font}</dd></div></dl><footer><Button icon="arrowRight" tone="primary" onClick={() => { setDirection(item.id); setComparison(false); }}>打开完整方案</Button><IconButton icon={chosen === item.id ? 'checkCircle' : 'bookmark'} label={`标记 ${item.letter} ${item.name} 为首选`} active={chosen === item.id} onClick={() => setChosen(chosen === item.id ? null : item.id)}/></footer></div></article>)}</div><div className="comparison-note"><Icon name="info" size={16}/><span>每套都包含完整的主要页面与交互，也可以混合喜欢的元素。最终方向由你选择。</span></div></div>;
}

function Miniature({ direction, context }) {
  const ref = useRef(null);
  const [scale, setScale] = useState(0.33);
  useEffect(() => {
    const resize = new ResizeObserver(entries => setScale(entries[0].contentRect.width / 1080));
    resize.observe(ref.current); return () => resize.disconnect();
  }, []);
  const appearance = direction === 'studio' ? 'dark' : 'light';
  return <div ref={ref} className="miniature-viewport"><div className="miniature" style={{ transform: `scale(${scale})` }} aria-hidden="true" inert=""><PreviewContext.Provider value={{ ...context, direction, appearance, effectiveAppearance: appearance, miniature: true }}><div className="theme-surface" data-theme={direction} data-appearance={appearance}><Workspace miniature forcedPage="translation"/></div></PreviewContext.Provider></div></div>;
}

createRoot(document.getElementById('root')).render(<App/>);
