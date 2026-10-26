import React, { useEffect, useRef, useState } from 'react';
import { Badge, Button, Icon, IconButton, Toggle, usePreview } from './ui.jsx';
import { RangeControl } from './settings.jsx';

const imageWidth = 960;
const imageHeight = 600;

export function paintDemoImage(canvas, annotations = []) {
  canvas.width = imageWidth;
  canvas.height = imageHeight;
  const ctx = canvas.getContext('2d');
  ctx.fillStyle = '#f4f1e7'; ctx.fillRect(0, 0, imageWidth, imageHeight);
  ctx.fillStyle = '#707865'; ctx.font = '12px -apple-system, sans-serif';
  ctx.fillText('N O T E S  O N  D E S I G N', 75, 66);
  ctx.fillText('№ 10', 845, 66);
  ctx.fillStyle = '#d1d2c3'; ctx.fillRect(75, 94, 810, 1);
  ctx.fillStyle = '#4a6150'; ctx.fillRect(78, 139, 30, 3);
  ctx.font = '84px Georgia, serif'; ctx.fillStyle = '#354337';
  ctx.fillText('Less,', 74, 236);
  ctx.font = 'italic 84px Georgia, serif'; ctx.fillText('but better.', 70, 330);
  ctx.fillStyle = '#525e51'; ctx.font = '21px Georgia, serif';
  ctx.fillText('Good design is as little design as possible.', 78, 386);
  ctx.font = '17px Georgia, serif'; ctx.fillStyle = '#818475';
  ctx.fillText('It concentrates on the essential aspects,', 78, 429);
  ctx.fillText('and the products are not burdened with non-essentials.', 78, 456);
  ctx.fillStyle = '#d1d2c3'; ctx.fillRect(75, 520, 810, 1);
  ctx.font = '11px -apple-system, sans-serif'; ctx.fillStyle = '#6d796c';
  ctx.fillText('D I E T E R  R A M S', 78, 550);
  ctx.fillText('Ten principles for good design', 707, 550);
  for (const item of annotations) drawAnnotation(ctx, item);
  return canvas;
}

function drawAnnotation(ctx, item) {
  ctx.save(); ctx.strokeStyle = item.color; ctx.fillStyle = item.color; ctx.lineWidth = item.width || 3; ctx.lineJoin = 'round'; ctx.lineCap = 'round';
  const x = Math.min(item.x, item.x2 ?? item.x), y = Math.min(item.y, item.y2 ?? item.y);
  const w = Math.abs((item.x2 ?? item.x) - item.x), h = Math.abs((item.y2 ?? item.y) - item.y);
  if (item.tool === 'rectangle') { ctx.beginPath(); ctx.rect(x, y, w, h); item.fill ? (ctx.globalAlpha = 0.18, ctx.fill(), ctx.globalAlpha = 1, ctx.stroke()) : ctx.stroke(); }
  if (item.tool === 'ellipse') { ctx.beginPath(); ctx.ellipse(x + w / 2, y + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2); if (item.fill) { ctx.globalAlpha = 0.18; ctx.fill(); ctx.globalAlpha = 1; } ctx.stroke(); }
  if (item.tool === 'arrow') {
    const angle = Math.atan2(item.y2 - item.y, item.x2 - item.x); const head = 13 + ctx.lineWidth * 2;
    ctx.beginPath(); ctx.moveTo(item.x, item.y); ctx.lineTo(item.x2, item.y2); ctx.moveTo(item.x2 - head * Math.cos(angle - Math.PI / 6), item.y2 - head * Math.sin(angle - Math.PI / 6)); ctx.lineTo(item.x2, item.y2); ctx.lineTo(item.x2 - head * Math.cos(angle + Math.PI / 6), item.y2 - head * Math.sin(angle + Math.PI / 6)); ctx.stroke();
  }
  if (item.tool === 'pen' || item.tool === 'highlighter') {
    if (item.tool === 'highlighter') { ctx.globalAlpha = 0.3; ctx.lineWidth = item.width * 7; }
    ctx.beginPath(); item.points.forEach((point, index) => index === 0 ? ctx.moveTo(point.x, point.y) : ctx.lineTo(point.x, point.y)); ctx.stroke();
  }
  if (item.tool === 'mosaic' && w > 1 && h > 1) {
    const buffer = document.createElement('canvas'); buffer.width = Math.max(1, Math.ceil(w / 14)); buffer.height = Math.max(1, Math.ceil(h / 14));
    buffer.getContext('2d').drawImage(ctx.canvas, x, y, w, h, 0, 0, buffer.width, buffer.height);
    ctx.imageSmoothingEnabled = false; ctx.drawImage(buffer, 0, 0, buffer.width, buffer.height, x, y, w, h);
  }
  if (item.tool === 'text') { ctx.font = `600 ${18 + item.width * 3}px -apple-system, "PingFang SC", sans-serif`; ctx.fillText(item.text, item.x, item.y); }
  if (item.tool === 'number') { ctx.beginPath(); ctx.arc(item.x, item.y, 17, 0, Math.PI * 2); ctx.fill(); ctx.fillStyle = '#ffffff'; ctx.font = '600 17px -apple-system, sans-serif'; ctx.textAlign = 'center'; ctx.textBaseline = 'middle'; ctx.fillText(String(item.number), item.x, item.y + 1); }
  ctx.restore();
}

function imageForExport(annotations, rotation) {
  const source = paintDemoImage(document.createElement('canvas'), annotations);
  if (!rotation) return source;
  const canvas = document.createElement('canvas');
  canvas.width = rotation % 2 ? imageHeight : imageWidth;
  canvas.height = rotation % 2 ? imageWidth : imageHeight;
  const ctx = canvas.getContext('2d');
  ctx.translate(canvas.width / 2, canvas.height / 2);
  ctx.rotate(rotation * Math.PI / 2);
  ctx.drawImage(source, -imageWidth / 2, -imageHeight / 2);
  return canvas;
}

export async function downloadDemoImage(annotations = [], rotation = 0) {
  const canvas = imageForExport(annotations, rotation);
  const blob = await new Promise(resolve => canvas.toBlob(resolve, 'image/png'));
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a'); link.href = url; link.download = 'SnapLingo-design-capture.png'; link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export async function copyDemoImage(annotations = [], rotation = 0) {
  const canvas = imageForExport(annotations, rotation);
  const blob = await new Promise(resolve => canvas.toBlob(resolve, 'image/png'));
  await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]);
}

export function DemoCanvas({ annotations = [], className = '', style }) {
  const ref = useRef(null);
  useEffect(() => { paintDemoImage(ref.current, annotations); }, [annotations]);
  return <canvas ref={ref} className={className} style={style} width={imageWidth} height={imageHeight} aria-label="设计原则阅读摘录" role="img"/>;
}

const tools = [
  { id: 'select', icon: 'cursor', label: '选择与移动', key: 'V' },
  { id: 'rectangle', icon: 'rectangle', label: '矩形', key: 'R' },
  { id: 'ellipse', icon: 'ellipse', label: '椭圆', key: 'O' },
  { id: 'arrow', icon: 'arrow', label: '箭头', key: 'A' },
  { id: 'pen', icon: 'pen', label: '画笔', key: 'P' },
  { id: 'highlighter', icon: 'highlighter', label: '高亮', key: 'H' },
  { id: 'text', icon: 'text', label: '文字', key: 'T' },
  { id: 'mosaic', icon: 'mosaic', label: '马赛克', key: 'M' },
  { id: 'number', icon: 'number', label: '序号', key: 'N' },
];

export function CaptureScene() {
  const { setScene, notify, downloadImage, copyImage, openPin, favoriteScreenshot } = usePreview();
  const [tool, setTool] = useState('rectangle');
  const [color, setColor] = useState('#e57162');
  const [width, setWidth] = useState(3);
  const [fill, setFill] = useState(false);
  const [history, setHistory] = useState([[], [{ tool: 'rectangle', x: 60, y: 158, x2: 521, y2: 345, color: '#e57162', width: 3 }]]);
  const [historyIndex, setHistoryIndex] = useState(1);
  const [draft, setDraft] = useState(null);
  const [selected, setSelected] = useState(null);
  const [moving, setMoving] = useState(null);
  const [textEntry, setTextEntry] = useState(null);
  const [showLoupe, setShowLoupe] = useState(false);
  const [pointer, setPointer] = useState({ x: 0, y: 0, color: '#f4f1e7' });
  const canvas = useRef(null);
  const content = useRef(null);
  const [boardWidth, setBoardWidth] = useState(undefined);
  useEffect(() => {
    const observer = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect;
      setBoardWidth(Math.min(800, width, Math.max(0, height - 2) * 1.6));
    });
    observer.observe(content.current);
    return () => observer.disconnect();
  }, []);
  const annotations = history[historyIndex];
  const display = draft ? moving ? annotations.map((item, index) => index === moving.index ? draft : item) : [...annotations, draft] : annotations;
  useEffect(() => {
    paintDemoImage(canvas.current, display);
    if (selected !== null && annotations[selected] && !draft) {
      const ctx = canvas.current.getContext('2d'); const item = annotations[selected];
      ctx.setLineDash([5, 5]); ctx.strokeStyle = '#467ee0'; ctx.lineWidth = 1;
      ctx.strokeRect(Math.min(item.x, item.x2 ?? item.x) - 8, Math.min(item.y, item.y2 ?? item.y) - 8, Math.max(16, Math.abs((item.x2 ?? item.x) - item.x)) + 16, Math.max(16, Math.abs((item.y2 ?? item.y) - item.y)) + 16);
    }
  }, [display, selected]);
  const commit = next => { setHistory(current => [...current.slice(0, historyIndex + 1), next]); setHistoryIndex(current => current + 1); setSelected(null); };
  const undo = () => { setHistoryIndex(current => Math.max(0, current - 1)); setSelected(null); };
  const redo = () => { setHistoryIndex(current => Math.min(history.length - 1, current + 1)); setSelected(null); };
  const position = event => { const rect = canvas.current.getBoundingClientRect(); return { x: Math.max(0, Math.min(imageWidth - 1, (event.clientX - rect.left) / rect.width * imageWidth)), y: Math.max(0, Math.min(imageHeight - 1, (event.clientY - rect.top) / rect.height * imageHeight)) }; };
  const down = event => {
    if (event.button !== 0 || textEntry) return;
    const point = position(event); canvas.current.focus();
    if (tool === 'text') { setTextEntry({ ...point, text: '' }); return; }
    if (tool === 'number') { commit([...annotations, { ...point, tool, width, color, number: annotations.filter(item => item.tool === 'number').length + 1 }]); return; }
    canvas.current.setPointerCapture(event.pointerId);
    if (tool === 'select') {
      let index = -1;
      for (let i = annotations.length - 1; i >= 0; i--) {
        const item = annotations[i];
        if (point.x >= Math.min(item.x, item.x2 ?? item.x) - 18 && point.x <= Math.max(item.x, item.x2 ?? item.x) + (item.tool === 'text' ? 180 : 18) && point.y >= Math.min(item.y, item.y2 ?? item.y) - 24 && point.y <= Math.max(item.y, item.y2 ?? item.y) + 24) { index = i; break; }
      }
      setSelected(index >= 0 ? index : null);
      if (index >= 0) { setMoving({ index, point, item: annotations[index] }); setDraft(annotations[index]); }
      return;
    }
    setSelected(null); setDraft({ tool, ...point, x2: point.x, y2: point.y, color, width, fill, points: [point] });
  };
  const move = event => {
    const point = position(event);
    if (showLoupe) { const pixel = canvas.current.getContext('2d').getImageData(Math.floor(point.x), Math.floor(point.y), 1, 1).data; setPointer({ ...point, color: '#' + [...pixel].slice(0, 3).map(value => value.toString(16).padStart(2, '0')).join('') }); }
    if (!draft) return;
    if (moving) { const dx = point.x - moving.point.x, dy = point.y - moving.point.y; const item = moving.item; setDraft({ ...item, x: item.x + dx, y: item.y + dy, x2: item.x2 === undefined ? undefined : item.x2 + dx, y2: item.y2 === undefined ? undefined : item.y2 + dy, points: item.points?.map(p => ({ x: p.x + dx, y: p.y + dy })) }); }
    else setDraft(current => ({ ...current, x2: point.x, y2: point.y, points: [...current.points, point] }));
  };
  const up = event => {
    if (canvas.current.hasPointerCapture(event.pointerId)) canvas.current.releasePointerCapture(event.pointerId);
    if (draft) {
      if (moving) commit(annotations.map((item, index) => index === moving.index ? draft : item));
      else if (Math.abs(draft.x2 - draft.x) + Math.abs(draft.y2 - draft.y) > 3 || draft.points?.length > 2) commit([...annotations, draft]);
    }
    setDraft(null); setMoving(null);
  };
  const commitText = () => { if (textEntry?.text.trim()) commit([...annotations, { tool: 'text', ...textEntry, color, width }]); setTextEntry(null); };
  const keyDown = event => {
    if (['INPUT', 'TEXTAREA', 'SELECT'].includes(event.target.tagName)) return;
    const mod = event.metaKey || event.ctrlKey;
    if (mod && event.key.toLowerCase() === 'z') { event.preventDefault(); event.shiftKey ? redo() : undo(); return; }
    if (mod && event.key.toLowerCase() === 's') { event.preventDefault(); downloadImage(annotations); return; }
    if (mod && event.key.toLowerCase() === 'c') { event.preventDefault(); copyImage(annotations); return; }
    if (mod && event.key.toLowerCase() === 't') { event.preventDefault(); openPin(annotations); return; }
    if (['Delete', 'Backspace'].includes(event.key) && selected !== null) { event.preventDefault(); commit(annotations.filter((_, index) => index !== selected)); return; }
    const chosen = tools.find(item => item.key === event.key.toUpperCase());
    if (chosen && !mod) { event.preventDefault(); setTool(chosen.id); }
  };
  return <div className="capture-stage" data-screen-label="截图标注工作区" onKeyDown={keyDown}><div className="capture-topbar"><span className="capture-title"><Icon name="scan" size={17}/><strong>屏幕捕捉</strong><span>已选区域</span></span><Badge>960 × 600 px</Badge><div className="spacer"/><span className="capture-top-hint">拖动以添加标注</span><IconButton icon="close" label="取消截图" onClick={() => setScene('workspace')}/></div>
    <div className="capture-content" ref={content}><div className="capture-board" style={{ width: boardWidth }}><span className="selection-size">960 × 600</span><canvas ref={canvas} width={imageWidth} height={imageHeight} tabIndex={0} aria-label="截图标注画布，拖动可添加标注" className={`capture-canvas tool-${tool}`} onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={() => { setDraft(null); setMoving(null); }}/><div className="selection-handles" aria-hidden="true">{Array.from({ length: 8 }, (_, index) => <i key={index}/>)}</div>{textEntry && <input className="canvas-text-input" aria-label="标注文字" autoFocus style={{ left: `${textEntry.x / imageWidth * 100}%`, top: `${textEntry.y / imageHeight * 100}%`, color }} value={textEntry.text} placeholder="输入文字…" onChange={event => setTextEntry(current => ({ ...current, text: event.target.value }))} onBlur={commitText} onKeyDown={event => { if (event.key === 'Enter') { event.preventDefault(); commitText(); } if (event.key === 'Escape') { event.stopPropagation(); setTextEntry(null); } }}/>}</div>
      {showLoupe && <div className="capture-loupe"><span style={{ background: pointer.color }}/><div><strong>{pointer.color.toUpperCase()}</strong><small>X {Math.round(pointer.x)} · Y {Math.round(pointer.y)}</small></div><IconButton icon="copy" label="复制像素颜色" onClick={() => { navigator.clipboard.writeText(pointer.color).then(() => notify('颜色值已复制'), () => notify('浏览器未允许复制')); }}/></div>}
    </div>
    <div className="capture-controls"><div className="annotation-options"><span className="current-tool"><Icon name={tools.find(item => item.id === tool)?.icon} size={14}/>{tools.find(item => item.id === tool)?.label}</span><span className="tool-divider"/><div className="annotation-colors">{['#e57162', '#e3b34d', '#6caa79', '#4e89df', '#ffffff'].map(value => <button type="button" key={value} aria-label={`标注颜色 ${value}`} className={value === color ? 'is-active' : ''} style={{ background: value }} onClick={() => setColor(value)}>{value === color && <Icon name="check" size={10}/>}</button>)}<label className="custom-color"><input type="color" aria-label="自定义标注颜色" value={color} onChange={event => setColor(event.target.value)}/><Icon name="plus" size={12}/></label></div><span className="tool-divider"/><RangeControl label="标注线宽" value={width} min={1} max={10} unit="px" onChange={setWidth}/>{['rectangle', 'ellipse'].includes(tool) && <label className="fill-option"><span>填充</span><Toggle label="填充形状" checked={fill} onChange={setFill}/></label>}</div>
      <div className="capture-toolbar"><div className="drawing-tools" role="toolbar" aria-label="标注工具">{tools.map(item => <IconButton key={item.id} icon={item.icon} label={`${item.label} (${item.key})`} active={item.id === tool} onClick={() => { setTool(item.id); setSelected(null); }}/>)}</div><span className="tool-divider"/><div className="history-tools"><IconButton icon="undo" label="撤销标注 (⌘ Z)" disabled={historyIndex === 0} onClick={undo}/><IconButton icon="redo" label="重做标注 (⌘ ⇧ Z)" disabled={historyIndex === history.length - 1} onClick={redo}/></div><span className="tool-divider"/><div className="output-tools"><IconButton icon="eye" label="显示像素取色器" active={showLoupe} onClick={() => setShowLoupe(value => !value)}/><IconButton icon="ocr" label="识别选区文字" onClick={() => setScene('recognize')}/><IconButton icon="bookmark" label="收藏截图" onClick={() => favoriteScreenshot(annotations)}/><IconButton icon="pin" label="贴图 (⌘ T)" onClick={() => openPin(annotations)}/><IconButton icon="download" label="保存截图 (⌘ S)" onClick={() => downloadImage(annotations)}/><Button tone="primary" small icon="copy" onClick={() => copyImage(annotations)}>复制</Button></div></div>
    </div><footer className="capture-help"><span><kbd>V</kbd>选择与移动<kbd>⌘ Z</kbd>撤销</span><span><Icon name="lock" size={12}/>演示画布 · 可实际标注与导出</span><span>Esc 退出截图</span></footer>
  </div>;
}

export function PinnedScene() {
  const { pinnedAnnotations, copyImage, downloadImage, setScene } = usePreview();
  const [zoom, setZoom] = useState(100);
  const [opacity, setOpacity] = useState(100);
  const [rotation, setRotation] = useState(0);
  const [locked, setLocked] = useState(false);
  const [top, setTop] = useState(true);
  const [position, setPosition] = useState({ x: 0, y: 0 });
  const drag = useRef(null);
  return <div className="pinned-stage" data-screen-label="贴图窗口"><div className="scene-caption"><span>把参考，留在手边</span><p>随意移动，随时对照。</p></div><div className="pinned-space"><div className="pinned-image-window" style={{ transform: `translate(${position.x}px, ${position.y}px) scale(${zoom / 100})` }}><header onPointerDown={event => { if (locked || event.target.closest('button')) return; drag.current = { x: event.clientX - position.x, y: event.clientY - position.y }; event.currentTarget.setPointerCapture(event.pointerId); }} onPointerMove={event => { if (drag.current) setPosition({ x: event.clientX - drag.current.x, y: event.clientY - drag.current.y }); }} onPointerUp={event => { drag.current = null; if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId); }}><Icon name="grip" size={16}/><span>阅读摘录</span><div className="spacer"/>{top && <Badge tone="accent">置顶</Badge>}<IconButton icon="close" label="关闭贴图" onClick={() => setScene('workspace')}/></header><div className="pinned-image-body" style={{ opacity: opacity / 100 }}><DemoCanvas annotations={pinnedAnnotations} style={{ transform: `rotate(${rotation * 90}deg) scale(${rotation % 2 === 1 ? 0.625 : 1})` }}/>{rotation > 0 && <div className="rotation-label"><Icon name="refresh" size={12}/>旋转 {rotation * 90}°</div>}</div><footer><span>960 × 600 px</span><span>{zoom}%<span className="status-separator">·</span>PNG</span></footer></div></div><div className="pinned-controls"><div className="pinned-toolbar"><IconButton icon="minus" label="缩小贴图" disabled={zoom <= 50} onClick={() => setZoom(value => Math.max(50, value - 10))}/><button type="button" className="zoom-value" title="恢复原始大小" onClick={() => { setZoom(100); setPosition({ x: 0, y: 0 }); }}>{zoom}%</button><IconButton icon="plus" label="放大贴图" disabled={zoom >= 150} onClick={() => setZoom(value => Math.min(150, value + 10))}/><span className="tool-divider"/><IconButton icon="pin" label={top ? '取消贴图置顶' : '贴图置顶'} active={top} onClick={() => setTop(value => !value)}/><IconButton icon="lock" label={locked ? '解锁贴图位置' : '锁定贴图位置'} active={locked} onClick={() => setLocked(value => !value)}/><IconButton icon="refresh" label="旋转贴图" onClick={() => setRotation(value => (value + 1) % 4)}/><span className="tool-divider"/><IconButton icon="copy" label="复制贴图" onClick={() => copyImage(pinnedAnnotations, rotation)}/><IconButton icon="download" label="保存贴图" onClick={() => downloadImage(pinnedAnnotations, rotation)}/></div><div className="pinned-opacity"><Icon name="eye" size={15}/><span>不透明度</span><RangeControl label="贴图不透明度" value={opacity} min={20} max={100} unit="%" onChange={setOpacity}/></div></div><div className="pinned-hint"><Icon name={locked ? 'lock' : 'grip'} size={13}/>{locked ? '位置已锁定，点击锁定按钮解除' : '拖动图片标题栏，放在顺手的位置'}</div></div>;
}
