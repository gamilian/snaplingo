import { useEffect, useRef, useState, type ButtonHTMLAttributes, type CSSProperties, type PointerEvent } from 'react';

interface SortSession {
  id: string;
  pointerId: number;
  startX: number;
  startY: number;
  x: number;
  y: number;
  listTop: number;
  scale: number;
  rows: { id: string; top: number; height: number }[];
  ids: string[];
  nextIds: string[];
  offset: number;
  moved: boolean;
  valid: boolean;
  settling: boolean;
}

export function useProviderSort({ ids, groupForId, onReorder }: {
  ids: string[];
  groupForId?: (id: string) => string;
  onReorder: (ids: string[], draggedId: string) => Promise<void>;
}) {
  const listRef = useRef<HTMLDivElement>(null);
  const session = useRef<SortSession | null>(null);
  const scrollFrame = useRef(0);
  const mounted = useRef(true);
  const [view, setView] = useState<SortSession | null>(null);
  const [announcement, setAnnouncement] = useState('');

  const stopScrolling = () => cancelAnimationFrame(scrollFrame.current);
  const clear = () => {
    stopScrolling();
    session.current = null;
    if (mounted.current) setView(null);
  };
  const settle = async (current: SortSession, save: boolean) => {
    stopScrolling();
    const duration = window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 240;
    current.settling = true;
    if (!save) current.nextIds = current.ids;
    current.offset = positions(current).get(current.id)! - current.rows.find((row) => row.id === current.id)!.top;
    setView({ ...current });
    try {
      await Promise.all([
        save ? onReorder(current.nextIds, current.id) : Promise.resolve(),
        new Promise((resolve) => setTimeout(resolve, duration)),
      ]);
      if (save && mounted.current) setAnnouncement(`已移至第 ${current.nextIds.indexOf(current.id) + 1} 项`);
    } catch (error) {
      current.nextIds = current.ids;
      current.offset = 0;
      if (mounted.current) setView({ ...current });
      await new Promise((resolve) => setTimeout(resolve, duration));
      if (mounted.current) alert(`重新排序失败: ${error}`);
    } finally {
      if (session.current === current) clear();
    }
  };

  useEffect(() => {
    mounted.current = true;
    const onKeyDown = (event: KeyboardEvent) => {
      const current = session.current;
      if (event.key === 'Escape' && current && !current.settling) {
        event.preventDefault();
        void settle(current, false);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      mounted.current = false;
      stopScrolling();
      session.current = null;
      window.removeEventListener('keydown', onKeyDown);
    };
  }, []);

  const update = (current: SortSession) => {
    const list = listRef.current!;
    const bounds = list.getBoundingClientRect();
    const source = current.rows.find((row) => row.id === current.id)!;
    const group = current.rows.filter((row) => !groupForId || groupForId(row.id) === groupForId(current.id));
    const first = group[0];
    const last = group[group.length - 1];
    const rawOffset = current.y - current.startY + current.listTop - bounds.top;
    current.offset = Math.max(first.top - source.top, Math.min(last.top + last.height - source.top - source.height, rawOffset));
    const pointerTop = current.y - bounds.top;
    current.valid = current.x >= bounds.left && current.x <= bounds.right && pointerTop >= first.top && pointerTop <= last.top + last.height;
    current.nextIds = [...current.ids];
    if (current.valid) {
      const center = source.top + current.offset + source.height / 2;
      const otherRows = group.filter((row) => row.id !== current.id);
      const target = otherRows.filter((row) => current.offset > 0 ? center >= row.top + row.height / 2 : center > row.top + row.height / 2).length;
      const nextGroup = otherRows.map((row) => row.id);
      nextGroup.splice(target, 0, current.id);
      current.nextIds.splice(current.ids.indexOf(first.id), group.length, ...nextGroup);
    }
    setView({ ...current });
  };

  const scroll = () => {
    const current = session.current;
    const area = listRef.current?.closest<HTMLElement>('.settings-scroll-area');
    if (!current || current.settling || !area) return;
    const bounds = area.getBoundingClientRect();
    const speed = current.y < bounds.top + 48
      ? -Math.min(8, (bounds.top + 48 - current.y) / 6)
      : current.y > bounds.bottom - 48 ? Math.min(8, (current.y - bounds.bottom + 48) / 6) : 0;
    if (speed && current.x >= bounds.left && current.x <= bounds.right) {
      const previous = area.scrollTop;
      area.scrollTop += speed;
      if (area.scrollTop !== previous) update(current);
    }
    scrollFrame.current = requestAnimationFrame(scroll);
  };

  const handleProps = (id: string): ButtonHTMLAttributes<HTMLButtonElement> => ({
    onPointerDown(event) {
      if (event.button !== 0 || session.current) return;
      event.preventDefault();
      event.currentTarget.setPointerCapture(event.pointerId);
      const bounds = listRef.current!.getBoundingClientRect();
      const rows = Array.from(listRef.current!.querySelectorAll<HTMLElement>('[data-provider-sort-id]'), (row) => {
        const rect = row.getBoundingClientRect();
        return { id: row.dataset.providerSortId!, top: rect.top - bounds.top, height: rect.height };
      });
      const scale = listRef.current!.offsetWidth ? bounds.width / listRef.current!.offsetWidth : 1;
      session.current = { id, pointerId: event.pointerId, startX: event.clientX, startY: event.clientY, x: event.clientX, y: event.clientY, listTop: bounds.top, scale, rows, ids: [...ids], nextIds: [...ids], offset: 0, moved: false, valid: false, settling: false };
    },
    onPointerMove(event) {
      const current = session.current;
      if (!current || current.settling || current.pointerId !== event.pointerId) return;
      current.x = event.clientX;
      current.y = event.clientY;
      if (!current.moved) {
        if (Math.hypot(current.x - current.startX, current.y - current.startY) < 4) return;
        current.moved = true;
        scrollFrame.current = requestAnimationFrame(scroll);
      }
      update(current);
    },
    onPointerUp(event) {
      const current = session.current;
      if (!current || current.settling || current.pointerId !== event.pointerId) return;
      if (!current.moved) { clear(); return; }
      current.x = event.clientX;
      current.y = event.clientY;
      update(current);
      void settle(current, current.valid && current.nextIds.some((item, index) => item !== current.ids[index]));
    },
    onPointerCancel: cancel,
    onLostPointerCapture: cancel,
    async onKeyDown(event) {
      if (session.current || !['ArrowUp', 'ArrowDown'].includes(event.key)) return;
      event.preventDefault();
      const from = ids.indexOf(id);
      const to = from + (event.key === 'ArrowUp' ? -1 : 1);
      if (to < 0 || to >= ids.length || (groupForId && groupForId(ids[to]) !== groupForId(id))) return;
      const next = [...ids];
      next.splice(from, 1);
      next.splice(to, 0, id);
      try {
        await onReorder(next, id);
        if (mounted.current) setAnnouncement(`已移至第 ${to + 1} 项`);
      } catch (error) { if (mounted.current) alert(`重新排序失败: ${error}`); }
    },
  });

  function cancel(event: PointerEvent<HTMLButtonElement>) {
    const current = session.current;
    if (current && !current.settling && current.pointerId === event.pointerId) void settle(current, false);
  }

  const previewPositions = view ? positions(view) : new Map<string, number>();
  const rowProps = (id: string) => {
    const row = view?.rows.find((item) => item.id === id);
    const active = view?.id === id;
    const offset = row ? active ? view!.offset : previewPositions.get(id)! - row.top : 0;
    return {
      'data-provider-sort-id': id,
      'data-sort-dragging': active && !view?.settling ? '' : undefined,
      'data-sort-settling': active && view?.settling ? '' : undefined,
      style: { transform: `translate3d(0, ${offset / (view?.scale ?? 1)}px, 0)`, transition: view ? undefined : 'none' } as CSSProperties,
    };
  };

  return {
    listRef,
    orderedIds: view?.ids ?? ids,
    handleProps,
    rowProps,
    announcement,
  };
}

function positions(session: SortSession) {
  const heights = new Map(session.rows.map((row) => [row.id, row.height]));
  const gap = session.rows.length > 1 ? session.rows[1].top - session.rows[0].top - session.rows[0].height : 0;
  let top = session.rows[0].top;
  return new Map(session.nextIds.map((id) => {
    const position = top;
    top += heights.get(id)! + gap;
    return [id, position];
  }));
}

export function ProviderDragHandle(props: ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button type="button" aria-label="拖动排序" title="拖动排序，也可用上下方向键调整" className="provider-drag-handle" {...props}>
    <svg width="6" height="14" viewBox="0 0 6 14" fill="currentColor" aria-hidden="true">
      <circle cx="1" cy="3" r="1" /><circle cx="5" cy="3" r="1" />
      <circle cx="1" cy="7" r="1" /><circle cx="5" cy="7" r="1" />
      <circle cx="1" cy="11" r="1" /><circle cx="5" cy="11" r="1" />
    </svg>
  </button>;
}
