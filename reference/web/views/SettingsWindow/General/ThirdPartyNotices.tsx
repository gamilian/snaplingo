import { useEffect, useRef, useState } from 'react';
import inventory from '../../../../../assets/licenses/thirdPartyNotices.json';
import { useSettingsRuntime } from '../runtimeContext';

export default function ThirdPartyNotices({ onClose }: { onClose(): void }) {
  const runtime = useSettingsRuntime();
  const dialog = useRef<HTMLDialogElement>(null);
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState<(typeof inventory.components)[number] | null>(null);
  useEffect(() => { dialog.current?.showModal(); return () => dialog.current?.close(); }, []);
  const components = inventory.components.filter((component) => `${component.name} ${component.license}`.toLowerCase().includes(query.toLowerCase()));
  return <dialog ref={dialog} aria-labelledby="notices-title" onCancel={onClose} className="third-party-notices m-auto overflow-hidden rounded-xl border border-line bg-surface p-0 text-ink shadow-e3 backdrop:bg-black/35">
    <header className="flex shrink-0 items-center justify-between gap-4 border-b border-line px-6 py-4">
      <div><h2 id="notices-title" className="text-base font-semibold">第三方组件与许可</h2><p className="mt-1 text-xs text-muted">查看组件来源和随附许可声明。</p></div>
      <button type="button" onClick={onClose} className="rounded-lg px-3 py-2 text-xs text-muted hover:bg-sunken">关闭</button>
    </header>
    <div className="flex min-h-0 flex-1 flex-col p-5">
      <input autoFocus type="search" aria-label="搜索第三方组件" value={query} onChange={(event) => { setQuery(event.currentTarget.value); setSelected(null); }} placeholder="搜索组件或许可证…" className="mb-4 h-9 w-full shrink-0 rounded-lg border border-line bg-canvas px-3 text-xs outline-none focus:border-accent" />
      {selected ? <div className="min-h-0 flex-1 overflow-auto">
        <button type="button" onClick={() => setSelected(null)} className="mb-3 text-xs text-accent-text">← 返回组件列表</button>
        <h3 className="text-sm font-semibold">{selected.name} <span className="font-normal text-muted">{selected.version}</span></h3>
        <button type="button" onClick={() => void runtime.window.openExternal(selected.source)} className="my-2 text-xs text-accent-text hover:underline">查看来源 · {selected.license}</button>
        {selected.notices.length ? selected.notices.map((id) => <pre key={id} className="mt-3 whitespace-pre-wrap break-words rounded-lg bg-sunken p-4 font-mono text-xs leading-relaxed text-muted">{(inventory.texts as Record<string, string>)[id]}</pre>) : <p className="mt-4 text-xs text-muted">该依赖的许可标识为 {selected.license}，完整许可请查看来源仓库。</p>}
      </div> : <div className="min-h-0 flex-1 overflow-auto rounded-lg border border-line">
        <table className="w-full text-left text-xs"><thead className="sticky top-0 bg-sunken text-muted"><tr><th className="px-4 py-3 font-medium">组件</th><th className="px-4 py-3 font-medium">版本</th><th className="px-4 py-3 font-medium">许可</th></tr></thead>
          <tbody className="divide-y divide-line">{components.map((component) => <tr key={`${component.category}/${component.name}/${component.version}`} className="hover:bg-sunken"><td className="px-4 py-3"><button type="button" onClick={() => setSelected(component)} className="text-accent-text hover:underline">{component.name}</button></td><td className="px-4 py-3 text-muted">{component.version}</td><td className="px-4 py-3 text-muted">{component.license}</td></tr>)}</tbody>
        </table>{!components.length && <p className="p-5 text-center text-xs text-muted">没有匹配的组件</p>}
      </div>}
    </div>
  </dialog>;
}
