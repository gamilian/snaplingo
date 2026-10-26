import React, { useState } from 'react';
import { Badge, Button, Icon, IconButton, SaveStatus, Select, Tabs, Toggle, usePreview } from './ui.jsx';
import { RangeControl } from './settings.jsx';

export function ServicesPage() {
  const { providers, setProviders, serviceTab, setServiceTab, setDialog, notify } = usePreview();
  const [editing, setEditing] = useState(null);
  const [dragging, setDragging] = useState(null);
  const [testing, setTesting] = useState(null);
  const current = providers.filter(item => item.group === serviceTab);
  const active = current.filter(item => item.active);
  const ordered = [...active, ...current.filter(item => !item.active)];
  const activate = provider => {
    if (!provider.configured) { setEditing(provider); return; }
    setProviders(items => items.map(item => item.group === 'ocr' && serviceTab === 'ocr' ? { ...item, active: item.id === provider.id } : item.id === provider.id ? { ...item, active: !item.active } : item));
    notify(serviceTab === 'ocr' ? `已选择 ${provider.name}` : `${provider.name} 已${provider.active ? '停用' : '启用'}`);
  };
  const reorder = (from, to) => {
    const next = [...providers];
    const fromIndex = next.findIndex(item => item.id === from);
    const toIndex = next.findIndex(item => item.id === to);
    if (fromIndex < 0 || toIndex < 0 || fromIndex === toIndex) return;
    next.splice(toIndex, 0, next.splice(fromIndex, 1)[0]); setProviders(next);
  };
  const test = provider => {
    setTesting(provider.id);
    setTimeout(() => { setTesting(null); notify(`已展示 ${provider.name} 的连接成功状态，未请求真实 API`); }, 850);
  };
  return <div className="services-page" data-screen-label="服务管理">
    <header className="page-header"><div><div className="page-eyebrow"><span>偏好设置</span><span>/</span><span>SERVICES</span></div><h1>服务管理<span className="paper-title-dot">.</span></h1><p>连接你信任的翻译、识别与语音服务。</p></div>{serviceTab === 'translation' && !editing && <Button tone="primary" icon="plus" onClick={() => setEditing({ id: '', name: '', mark: 'AI', color: '#64886c', group: 'translation', builtin: false, active: false, configured: false, tag: '自定义' })}>添加服务</Button>}</header>
    <div className="section-nav-wrap"><Tabs label="服务类型" items={[{ id: 'translation', label: '翻译服务', icon: 'languages' }, { id: 'ocr', label: '文字识别', icon: 'ocr' }, { id: 'tts', label: '语音合成', icon: 'volume' }]} value={serviceTab} onChange={value => { setServiceTab(value); setEditing(null); }}/><SaveStatus/></div>
    {editing ? <ProviderEditor key={editing.id || 'new'} provider={editing} onClose={() => setEditing(null)}/> : serviceTab === 'tts' ? <VoiceSettings/> : <>
      <div className="service-summary"><div><span className="service-summary-icon"><Icon name={serviceTab === 'ocr' ? 'ocr' : 'layers'} size={21}/></span><div><h2>{serviceTab === 'ocr' ? '选择默认识别引擎' : '让译文多一个视角'}</h2><p>{serviceTab === 'ocr' ? '一次使用一个引擎，优先选择适合内容的服务。' : '启用多个服务，在结果窗口中并排比较。'}</p></div></div><Badge tone="accent" dot>{active.length} 个已启用</Badge></div>
      <div className="provider-list">{ordered.map((provider, index) => <article className={`provider-card ${provider.active ? 'provider-enabled' : ''} ${dragging === provider.id ? 'is-dragging' : ''}`} key={provider.id} draggable={serviceTab === 'translation' && provider.active} onDragStart={event => { setDragging(provider.id); event.dataTransfer.setData('text/plain', provider.id); }} onDragOver={event => { if (provider.active) event.preventDefault(); }} onDrop={event => { event.preventDefault(); if (provider.active && dragging) reorder(dragging, provider.id); setDragging(null); }} onDragEnd={() => setDragging(null)}>
        <div className="provider-main"><span className="provider-mark" style={{ '--provider-color': provider.color }}>{provider.mark}</span><div className="provider-name"><h3>{provider.name}</h3><span>{provider.tag}</span></div>{provider.active && <Badge tone="good" dot>已启用</Badge>}{!provider.configured && <Badge tone="warn">待配置</Badge>}</div>
        <p>{provider.description || '通过自己的 API 连接翻译模型。'}</p>{provider.endpoint && <span className="provider-endpoint">{provider.endpoint}</span>}
        <footer><div className="provider-config"><Button small tone="quiet" icon="sliders" onClick={() => setEditing(provider)}>配置</Button>{provider.configured && <IconButton icon="bolt" label={`测试 ${provider.name} 连接`} disabled={testing === provider.id} className={testing === provider.id ? 'is-spinning' : ''} onClick={() => test(provider)}/>}</div><div className="provider-state-actions">{serviceTab === 'translation' && provider.active && <div className="provider-order"><IconButton icon="arrowUp" label={`上移 ${provider.name}`} disabled={index === 0} onClick={() => reorder(provider.id, ordered[index - 1].id)}/><IconButton icon="arrowDown" label={`下移 ${provider.name}`} disabled={index >= active.length - 1} onClick={() => reorder(provider.id, ordered[index + 1].id)}/></div>}{!provider.builtin && <IconButton icon="trash" label={`删除 ${provider.name}`} onClick={() => setDialog({ type: 'confirm', title: `删除 ${provider.name}？`, description: '将从预览中移除这个自定义服务。', confirmText: '删除服务', danger: true, onConfirm: () => { setProviders(items => items.filter(item => item.id !== provider.id)); notify('自定义服务已删除'); } })}/>}<Toggle checked={provider.active} label={`启用 ${provider.name}`} onChange={() => activate(provider)} disabled={serviceTab === 'ocr' && provider.active}/></div></footer>
      </article>)}</div>
      <div className="service-footnote"><Icon name={serviceTab === 'ocr' ? 'shield' : 'grip'} size={15}/><span>{serviceTab === 'ocr' ? '标记为「本地处理」的引擎直接在设备上识别图片。' : '拖动已启用的服务，或使用箭头，调整译文的显示顺序。'}</span></div>
    </>}
  </div>;
}

function ProviderEditor({ provider, onClose }) {
  const { setProviders, notify } = usePreview();
  const [draft, setDraft] = useState({ name: provider.name, endpoint: provider.endpoint || '', apiKey: '', appId: '', model: provider.model || '', protocol: provider.protocol || 'OpenAI', mode: provider.mode || 'Chat Completions', prompt: provider.prompt || '请将以下文本翻译为 {target_language}。保留原文格式，准确传达含义，只输出译文。\n\n{text}', strategy: '通用翻译', reasoning: '默认' });
  const [tab, setTab] = useState('connection');
  const [showKey, setShowKey] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testStatus, setTestStatus] = useState('');
  const [error, setError] = useState('');
  const update = (id, value) => setDraft(current => ({ ...current, [id]: value }));
  const isCustom = !provider.builtin;
  const needsEndpoint = isCustom || provider.id === 'deeplx';
  const needsKey = isCustom || provider.id.includes('baidu');
  const save = event => {
    event.preventDefault();
    if (!draft.name.trim()) { setError('请填写服务名称。'); return; }
    if (needsEndpoint && !/^https?:\/\//i.test(draft.endpoint)) { setError('请输入以 http:// 或 https:// 开头的服务地址。'); setTab('connection'); return; }
    if (needsKey && !provider.configured && !draft.apiKey.trim()) { setError('请填写一个演示密钥，用于查看配置完成状态。'); setTab('connection'); return; }
    const next = { ...provider, id: provider.id || `custom-${Date.now()}`, name: draft.name.trim(), endpoint: draft.endpoint, model: draft.model, protocol: draft.protocol, mode: draft.mode, prompt: draft.prompt, configured: true, active: true, description: provider.description || '使用自定义提示词，获得更贴合语境的译文。' };
    setProviders(items => {
      const list = provider.id ? items.map(item => item.id === provider.id ? next : item) : [...items, next];
      return provider.group === 'ocr' ? list.map(item => item.group === 'ocr' ? { ...item, active: item.id === next.id } : item) : list;
    });
    notify('服务配置已保存到预览'); onClose();
  };
  const field = (id, label, placeholder, type = 'text', hint) => <label className="form-field"><span>{label}</span><input type={type} value={draft[id]} onChange={event => update(id, event.target.value)} placeholder={placeholder} spellCheck={false} autoComplete="off"/>{hint && <small>{hint}</small>}</label>;
  return <form className="provider-editor" onSubmit={save}><div className="editor-heading"><IconButton icon="arrowLeft" label="返回服务列表" onClick={onClose}/><div><h2>{provider.id ? `配置 ${provider.name}` : '添加自定义翻译服务'}</h2><p>{provider.id ? '管理连接信息与翻译偏好。' : '用你选择的模型，构建更合适的翻译体验。'}</p></div></div>
    {isCustom && <Tabs label="服务配置分组" items={[{ id: 'connection', label: '连接设置', icon: 'link' }, { id: 'prompt', label: '翻译提示词', icon: 'text' }]} value={tab} onChange={setTab}/>}
    <div className="editor-fields">{tab === 'connection' ? <>{field('name', '服务名称', '例如：我的 AI 翻译')}{isCustom && <><div className="form-field"><span>接口协议</span><Tabs label="接口协议" items={['OpenAI', 'Anthropic', 'Gemini']} value={draft.protocol} onChange={value => update('protocol', value)}/></div>{draft.protocol === 'OpenAI' && <div className="form-field"><span>OpenAI 模式</span><Select value={draft.mode} onChange={value => update('mode', value)} options={['Chat Completions', 'Responses']} label="OpenAI 模式"/></div>}</>}{needsEndpoint && field('endpoint', '服务地址', provider.id === 'deeplx' ? 'http://localhost:1188/translate' : 'https://api.example.com/v1')}{provider.id.includes('baidu') && field('appId', '应用 ID', '填写应用 ID')}{needsKey && <label className="form-field"><span>API 密钥</span><div className="password-field"><input type={showKey ? 'text' : 'password'} value={draft.apiKey} onChange={event => update('apiKey', event.target.value)} placeholder={provider.configured ? '已有配置 · 留空保持不变' : '使用演示值即可'} autoComplete="off"/><IconButton icon={showKey ? 'eyeOff' : 'eye'} label={showKey ? '隐藏密钥' : '显示密钥'} onClick={() => setShowKey(value => !value)}/></div><small>设计预览不会发送或持久化这个字段。</small></label>}{isCustom && <><div className="form-field"><span>模型</span><div className="input-with-button"><input value={draft.model} onChange={event => update('model', event.target.value)} placeholder="输入模型名称"/><Button small onClick={() => { update('model', 'demo-translation-model'); notify('已填入演示模型，未连接服务'); }}>获取模型</Button></div></div><div className="form-field"><span>推理强度</span><Select value={draft.reasoning} options={['默认', '低', '中', '高']} onChange={value => update('reasoning', value)} label="推理强度"/></div></>}{!needsKey && !needsEndpoint && <div className="inline-note"><Icon name="checkCircle" size={18}/><p>{provider.group === 'ocr' ? '此引擎直接在本地运行，无需 API 密钥。' : '这是应用内置的服务，无需填写额外配置。'}</p></div>}</> : <><div className="form-field"><span>提示词策略</span><Select value={draft.strategy} options={['通用翻译', '自然表达', '技术文档', '自定义']} onChange={value => { update('strategy', value); if (value === '自然表达') update('prompt', '将下面的内容自然地翻译为 {target_language}，保留原文语气，避免逐字直译。\n\n{text}'); if (value === '技术文档') update('prompt', '将技术内容翻译为 {target_language}。保留代码、术语与 Markdown 格式。\n\n{text}'); }} label="提示词策略"/></div><label className="form-field"><span>翻译提示词</span><textarea value={draft.prompt} onChange={event => update('prompt', event.target.value)} rows={8}/><small>可使用 {'{text}'} 和 {'{target_language}'} 插入原文与目标语言。</small></label></>}{error && <p role="alert" className="form-error"><Icon name="warning" size={14}/>{error}</p>}{testStatus && <div role="status" className="inline-note note-success"><Icon name="checkCircle" size={16}/>{testStatus}</div>}</div>
    <footer className="editor-footer"><Button icon="bolt" disabled={testing} onClick={() => { setTesting(true); setTestStatus(''); setTimeout(() => { setTesting(false); setTestStatus('连接成功状态预览 · 未发出真实请求'); }, 900); }}>{testing ? '测试中…' : '测试连接'}</Button><div className="spacer"/><Button onClick={onClose}>取消</Button><button type="submit" className="button button-primary"><Icon name="check" size={15}/>保存并启用</button></footer>
  </form>;
}

function VoiceSettings() {
  const { settings, updateSetting, speak } = usePreview();
  const [sample, setSample] = useState('你好，欢迎使用 SnapLingo。让工具成为思考的延伸。');
  return <div className="voice-settings"><div className="voice-hero"><span><Icon name="volume" size={32}/></span><div><h2>让文字，也有声音</h2><p>使用系统语音，听一听另一种语言。</p></div><Badge tone="good" dot>系统语音</Badge></div><section className="settings-section"><div className="section-body"><div className="setting-row"><div className="row-label"><h3>语音</h3><p>使用当前系统已安装的声音。</p></div><Select label="系统语音" value={settings.voice || '系统默认'} onChange={value => updateSetting('voice', value)} options={['系统默认', 'Tingting · 简体中文', 'Samantha · English', 'Kyoko · 日本語']}/></div><div className="setting-row"><div className="row-label"><h3>朗读速度</h3><p>每分钟朗读字数。</p></div><RangeControl value={settings.voiceRate || 180} min={80} max={400} unit="字/分" label="朗读速度" onChange={value => updateSetting('voiceRate', value)}/></div></div></section><div className="voice-sample"><label htmlFor="voice-sample-input">试着听一段</label><textarea id="voice-sample-input" value={sample} onChange={event => setSample(event.target.value)} rows={3}/><footer><span>选一段文字，感受合适的语速。</span><Button tone="primary" icon="volume" disabled={!sample.trim()} onClick={() => speak(sample, 'zh-CN')}>试听</Button></footer></div></div>;
}
