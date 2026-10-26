import React, { useEffect, useRef, useState } from 'react';
import { alternateTranslation, initialRecords, languages, sampleOcrText, sourceText, translatedText } from './data.js';
import { Badge, Button, DocumentSample, EmptyState, Icon, IconButton, Select, Skeleton, Tabs, TrafficLights, usePreview } from './ui.jsx';

export function ResultScene() {
  const { scene, setScene, source, setSource, screenState, setScreenState } = usePreview();
  const [pinned, setPinned] = useState(false);
  const isOcr = scene === 'recognize';
  return <div className="result-stage" data-screen-label={isOcr ? '文字识别浮窗' : '翻译浮窗'}><div className="scene-caption"><span>{isOcr ? '从图片，到文字' : '从一种语言，到另一种理解'}</span><p>{isOcr ? '提取、编辑、复制，一气呵成。' : '轻轻唤起，用完即走。'}</p></div><section className={`result-window ${isOcr ? 'result-ocr' : ''} ${pinned ? 'window-pinned' : ''}`}><header className="result-chrome"><TrafficLights/><span className="result-wordmark"><img src="./app-icon.svg" alt=""/>SnapLingo</span><div className="spacer"/><IconButton icon="pin" label={pinned ? '取消置顶' : '窗口置顶'} active={pinned} onClick={() => setPinned(value => !value)}/><IconButton icon="close" label="关闭结果窗口" onClick={() => setScene('workspace')}/></header><div className="result-modebar"><Tabs label="结果类型" value={isOcr ? 'recognize' : 'translate'} items={[{ id: 'translate', label: '翻译', icon: 'languages' }, { id: 'recognize', label: '文字识别', icon: 'ocr' }]} onChange={value => { setScene(value); setScreenState('ready'); }}/><span className="result-mode-hint">{pinned ? <><Icon name="pin" size={12}/>已置顶</> : <><span className="online-dot"/>随时待命</>}</span></div>{isOcr ? <OcrContent/> : <TranslationContent/>}<footer className="result-statusbar"><span><Icon name={isOcr ? 'shield' : 'bolt'} size={12}/>{isOcr ? 'Apple Vision · 本地识别' : '多引擎翻译'}</span><span>Esc 关闭<span className="status-separator">·</span>⌘ ↵ {isOcr ? '识别' : '翻译'}</span></footer></section><div className="scene-shortcut"><Icon name="keyboard" size={14}/><span>{isOcr ? '⌥ O' : '⌥ D'}</span><i/>{isOcr ? '唤起文字识别' : '唤起划词翻译'}</div></div>;
}

function TranslationContent() {
  const { source, setSource, providers, setPage, setScene, copyText, speak, screenState, setScreenState, settings } = usePreview();
  const [sourceLang, setSourceLang] = useState(settings.sourceLanguage || '自动检测');
  const [targetLang, setTargetLang] = useState(settings.targetLanguage || '简体中文');
  const [collapsed, setCollapsed] = useState(false);
  const [lastTranslated, setLastTranslated] = useState(source);
  const [result, setResult] = useState(initialRecords.find(item => item.kind === 'translation' && item.source === source)?.result || '这是交互预览，尚未连接翻译服务。你输入的原文已保留，接入服务后译文会显示在这里。');
  const [lastTarget, setLastTarget] = useState('简体中文');
  const engines = providers.filter(item => item.group === 'translation' && item.active);
  const translate = () => {
    if (!source.trim()) { setScreenState('empty'); return; }
    setScreenState('loading');
    setTimeout(() => {
      const known = initialRecords.find(item => item.kind === 'translation' && item.source.trim() === source.trim());
      const example = targetLang === '简体中文' ? known?.result : targetLang === 'English' && known ? known.source : undefined;
      setResult(example || '这是交互预览，尚未连接翻译服务。你输入的原文已保留，接入服务后译文会显示在这里。');
      setLastTranslated(source); setLastTarget(targetLang);
      setScreenState('ready');
    }, 900);
  };
  const isSample = ['简体中文', 'English'].includes(lastTarget) && initialRecords.some(item => item.kind === 'translation' && item.source.trim() === lastTranslated.trim());
  return <><div className="language-bar"><Select label="原文语言" value={sourceLang} options={languages} onChange={setSourceLang}/><IconButton icon="swap" label="交换翻译语言" onClick={() => { setSourceLang(targetLang); setTargetLang(sourceLang === '自动检测' ? 'English' : sourceLang); }}/><Select label="译文语言" value={targetLang} options={languages.filter(value => value !== '自动检测')} onChange={setTargetLang}/></div><div className="translation-columns"><section className="source-editor"><header><span><Icon name="text" size={14}/>原文{sourceLang === '自动检测' && source.trim() && <small>{/[\u4e00-\u9fff]/.test(source) ? '检测到中文' : '检测到英语'}</small>}</span><IconButton icon={collapsed ? 'plus' : 'minus'} label={collapsed ? '展开原文' : '折叠原文'} onClick={() => setCollapsed(value => !value)}/></header>{collapsed ? <button type="button" className="collapsed-source" onClick={() => setCollapsed(false)}>{source || '点击输入原文'}<Icon name="chevron" size={14}/></button> : <textarea aria-label="需要翻译的原文" placeholder="输入或粘贴需要翻译的文本…" value={source} onChange={event => setSource(event.target.value)} onKeyDown={event => { if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) { event.preventDefault(); translate(); } }} spellCheck={false}/>}
    <footer><span className="character-count">{source.length} 字符</span><div className="inline-actions"><IconButton icon="volume" label="朗读原文" disabled={!source.trim()} onClick={() => speak(source, 'en-US')}/><IconButton icon="copy" label="复制原文" disabled={!source.trim()} onClick={() => copyText(source)}/><IconButton icon="trash" label="清空原文" disabled={!source} onClick={() => { setSource(''); setScreenState('empty'); }}/><Button small tone="primary" icon="arrowRight" disabled={!source.trim() || screenState === 'loading'} onClick={translate}>{screenState === 'loading' ? '翻译中' : '翻译'}</Button></div></footer></section>
    <div className="translation-results"><div className="translation-results-heading"><span>翻译结果</span><span>{engines.length} 个服务</span></div>{screenState === 'empty' || !source.trim() ? <EmptyState icon="languages" title="下一段理解，从这里开始" description="输入一段文字，或试试我们的示例。" action={<Button small onClick={() => { setSource(sourceText); setLastTranslated(sourceText); setLastTarget('简体中文'); setTargetLang('简体中文'); setResult(translatedText); setScreenState('ready'); }}>填入示例</Button>}/> : engines.length === 0 ? <EmptyState icon="layers" title="还没有启用翻译服务" description="在服务管理中选择一个喜欢的翻译引擎。" action={<Button small onClick={() => { setPage('services'); setScene('workspace'); }}>前往服务管理</Button>}/> : engines.map((provider, index) => <TranslationResultCard key={provider.id} provider={provider} text={index === 1 && lastTarget === '简体中文' && lastTranslated === sourceText ? alternateTranslation : result} original={lastTranslated} sample={isSample} language={lastTarget} state={screenState} onRetry={translate}/>)}</div></div></>;
}

function TranslationResultCard({ provider, text, original, state, onRetry, sample, language }) {
  const { copyText, speak, records, favoriteResult } = usePreview();
  const [expanded, setExpanded] = useState(true);
  const saved = records.some(item => item.favorite && item.source === original && item.provider === provider.name);
  return <article className={`translation-result ${state === 'error' ? 'has-error' : ''}`}><header><button type="button" className="result-provider" onClick={() => setExpanded(value => !value)} aria-expanded={expanded}><span className="result-provider-mark" style={{ '--provider-color': provider.color }}>{provider.mark}</span><strong>{provider.name}</strong><Icon name="chevron" size={12} className={expanded ? '' : 'is-rotated'}/></button>{state === 'loading' ? <Badge>翻译中</Badge> : state === 'error' ? <Badge tone="warn">未完成</Badge> : <span className="result-language">{sample ? language : '预览提示'}</span>}</header>{expanded && (state === 'loading' ? <Skeleton/> : state === 'error' ? <div className="result-error"><Icon name="warning" size={18}/><div><strong>暂时无法连接此服务</strong><p>原文已保留，可以稍后重试。</p></div><Button small icon="refresh" onClick={onRetry}>重试</Button></div> : <><p className="translated-copy">{text}</p><footer><span className="result-ready"><Icon name="check" size={11}/>已完成</span><div className="inline-actions"><IconButton icon="refresh" label={`重新使用 ${provider.name} 翻译`} onClick={onRetry}/><IconButton icon="volume" label={`朗读 ${provider.name} 译文`} onClick={() => speak(text, 'zh-CN')}/><IconButton icon="bookmark" label={saved ? `${provider.name} 译文已收藏` : `收藏 ${provider.name} 译文`} active={saved} onClick={() => favoriteResult({ source: original, text, provider: provider.name })}/><IconButton icon="copy" label={`复制 ${provider.name} 译文`} onClick={() => copyText(text)}/></div></footer></>)}</article>;
}

function OcrContent() {
  const { setScene, setSource, screenState, setScreenState, copyText, speak, favoriteResult, notify } = usePreview();
  const [recognized, setRecognized] = useState(sampleOcrText);
  const [image, setImage] = useState(null);
  const [imageName, setImageName] = useState('阅读摘录.png');
  const input = useRef(null);
  useEffect(() => () => { if (image) URL.revokeObjectURL(image); }, [image]);
  const recognize = () => {
    setScreenState('loading');
    setTimeout(() => { setRecognized(image ? '图片已载入。这是界面预览，未连接实际文字识别引擎。' : sampleOcrText); setScreenState('ready'); }, 850);
  };
  const upload = event => {
    const file = event.target.files?.[0];
    if (!file) return;
    if (!file.type.startsWith('image/')) { notify('请选择图片文件'); return; }
    if (image) URL.revokeObjectURL(image);
    setImage(URL.createObjectURL(file)); setImageName(file.name); setRecognized(''); setScreenState('empty');
  };
  return <div className="ocr-content"><section className="ocr-source"><header><span>源图片</span><span>PNG / JPG / WebP</span></header><div className="ocr-image-preview">{image ? <img src={image} alt={imageName}/> : <DocumentSample compact/>}</div><footer><span title={imageName}>{imageName}</span><Button small tone="quiet" icon="upload" onClick={() => input.current?.click()}>更换图片</Button><input ref={input} type="file" accept="image/png,image/jpeg,image/webp" onChange={upload} hidden/></footer><Button tone="primary" icon="ocr" disabled={screenState === 'loading'} onClick={recognize}>{screenState === 'loading' ? '正在识别…' : '识别文字'}</Button></section><section className="ocr-output"><header><h3>识别文本</h3><Badge tone="good">{image ? '文件已载入' : '清晰文本'}</Badge></header>{screenState === 'loading' ? <Skeleton lines={5}/> : screenState === 'error' ? <EmptyState icon="warning" title="这次识别没有完成" description="可以换一张图片，或再次尝试。" action={<Button small onClick={recognize}>重新识别</Button>}/> : <textarea aria-label="识别出的文本" value={screenState === 'empty' ? '' : recognized} onChange={event => { setRecognized(event.target.value); setScreenState('ready'); }} placeholder="识别出的文字会出现在这里，也可以直接编辑。" onKeyDown={event => { if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) { event.preventDefault(); recognize(); } }}/>}<footer><span>{screenState === 'empty' ? 0 : recognized.length} 字符 · 可编辑</span><div className="inline-actions"><IconButton icon="volume" label="朗读识别文本" disabled={!recognized || screenState !== 'ready'} onClick={() => speak(recognized)}/><IconButton icon="bookmark" label="收藏识别文本" disabled={!recognized || screenState !== 'ready'} onClick={() => favoriteResult({ source: recognized, text: recognized, provider: 'Apple Vision', kind: 'ocr' })}/><IconButton icon="copy" label="复制识别文本" disabled={!recognized || screenState !== 'ready'} onClick={() => copyText(recognized)}/></div></footer><Button icon="languages" disabled={!recognized || screenState !== 'ready'} onClick={() => { setSource(recognized); setScene('translate'); setScreenState('ready'); }}>继续翻译<Icon name="arrowRight" size={14}/></Button></section></div>;
}

export function PermissionScene() {
  const { permissions, setDialog, setScene, setPage } = usePreview();
  return <div className="permission-stage" data-screen-label="首次权限引导"><section className="permission-window"><header><TrafficLights/><span>欢迎使用 SnapLingo</span></header><div className="permission-welcome"><img src="./app-icon.svg" alt="SnapLingo"/><span className="welcome-eyebrow">READY WHEN YOU ARE</span><h1>你的灵感，<br/>随时捕捉。</h1><p>开始之前，给 SnapLingo 一点帮助。<br/>只需几步，就能截图、取词和翻译。</p></div><div className="onboarding-permissions">{[{ id: 'screen', icon: 'scan', title: '屏幕录制', badge: '必需', text: '捕捉屏幕上的内容，用于截图和识别。' }, { id: 'accessibility', icon: 'cursor', title: '辅助功能', badge: '可选', text: '更准确地获取其他应用中选中的文字。' }].map(item => <div key={item.id}><span className="permission-icon"><Icon name={item.icon} size={21}/></span><div><h3>{item.title}<small>{item.badge}</small></h3><p>{item.text}</p></div>{permissions[item.id] ? <Badge tone="good" dot>已就绪</Badge> : <Button small onClick={() => setDialog({ type: 'permission', permission: item.id, title: item.title })}>开启<Icon name="arrowRight" size={13}/></Button>}</div>)}</div><footer><Button tone="quiet" onClick={() => { setPage('general'); setScene('workspace'); }}>稍后设置</Button><Button tone="primary" icon="arrowRight" disabled={!permissions.screen} onClick={() => { setPage('translation'); setScene('workspace'); }}>开始使用</Button></footer><div className="permission-privacy"><Icon name="shield" size={13}/><span>仅在你主动操作时，读取对应的屏幕内容。</span></div></section></div>;
}
