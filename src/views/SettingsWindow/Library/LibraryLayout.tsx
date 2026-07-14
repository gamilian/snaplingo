import type { ReactNode } from 'react';

export type LibraryFilter<Key extends string> = {
  key: Key;
  label: string;
};

export function LibraryLayout<Key extends string>({
  title,
  total,
  search,
  searchPlaceholder,
  onSearchChange,
  filters,
  activeFilter,
  onFilterChange,
  list,
  detail,
  page,
  pageSize,
  onPageChange,
  footerAction,
}: {
  title: string;
  total: number;
  search: string;
  searchPlaceholder: string;
  onSearchChange: (value: string) => void;
  filters: LibraryFilter<Key>[];
  activeFilter: Key;
  onFilterChange: (filter: Key) => void;
  list: ReactNode;
  detail: ReactNode;
  page: number;
  pageSize: number;
  onPageChange: (page: number) => void;
  footerAction?: ReactNode;
}) {
  const start = total === 0 ? 0 : page * pageSize + 1;
  const end = Math.min(total, (page + 1) * pageSize);
  const hasPrevious = page > 0;
  const hasNext = end < total;

  return (
    <div className="grid h-full min-h-0 min-w-0 flex-1 grid-cols-[410px_minmax(0,1fr)] bg-white">
      <section className="flex min-h-0 min-w-0 flex-col border-r border-gray-200 bg-gray-50/60">
        <header className="space-y-3 border-b border-gray-200 bg-white px-4 pb-3 pt-5">
          <div className="flex items-end justify-between gap-3">
            <h1 className="text-xl font-bold tracking-[-0.035em] text-gray-900">
              {title}
            </h1>
            <span className="pb-0.5 text-[11px] tabular-nums text-gray-400">
              {total.toLocaleString()} 项
            </span>
          </div>
          <label className="relative block">
            <span className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-sm text-gray-400">
              ⌕
            </span>
            <input
              type="search"
              value={search}
              onChange={(event) => onSearchChange(event.target.value)}
              placeholder={searchPlaceholder}
              className="h-9 w-full rounded-lg border border-gray-200 bg-gray-50 pl-8 pr-3 text-xs text-gray-800 outline-none transition focus:border-primary-300 focus:bg-white focus:ring-2 focus:ring-primary-100"
            />
          </label>
          <div className="flex gap-1.5 overflow-x-auto">
            {filters.map((filter) => (
              <button
                key={filter.key}
                type="button"
                onClick={() => onFilterChange(filter.key)}
                className={`h-7 shrink-0 rounded-md border px-2.5 text-[11px] font-medium transition-colors ${
                  filter.key === activeFilter
                    ? 'border-primary-200 bg-primary-50 text-primary-700'
                    : 'border-gray-200 bg-white text-gray-500 hover:border-gray-300 hover:text-gray-800'
                }`}
              >
                {filter.label}
              </button>
            ))}
          </div>
        </header>

        <div className="min-h-0 flex-1 overflow-y-auto">{list}</div>

        <footer className="flex h-11 shrink-0 items-center justify-between gap-3 border-t border-gray-200 bg-white px-4 text-[10px] text-gray-500">
          <span className="tabular-nums">
            {start}–{end} / {total.toLocaleString()}
          </span>
          <div className="flex items-center gap-3">
            {footerAction}
            <div className="flex overflow-hidden rounded-md border border-gray-200">
              <button
                type="button"
                disabled={!hasPrevious}
                onClick={() => onPageChange(page - 1)}
                className="grid h-7 w-8 place-items-center border-r border-gray-200 bg-white text-gray-500 hover:bg-gray-50 disabled:cursor-default disabled:text-gray-200"
                aria-label="上一页"
              >
                ‹
              </button>
              <button
                type="button"
                disabled={!hasNext}
                onClick={() => onPageChange(page + 1)}
                className="grid h-7 w-8 place-items-center bg-white text-gray-500 hover:bg-gray-50 disabled:cursor-default disabled:text-gray-200"
                aria-label="下一页"
              >
                ›
              </button>
            </div>
          </div>
        </footer>
      </section>

      <section className="min-h-0 min-w-0 overflow-y-auto bg-[#f5f6f8] px-7 py-7">
        {detail}
      </section>
    </div>
  );
}

export function LibraryListItem({
  active,
  kind,
  kindTone,
  time,
  title,
  preview,
  onClick,
}: {
  active: boolean;
  kind: string;
  kindTone: 'blue' | 'purple' | 'green';
  time: string;
  title: string;
  preview: string;
  onClick: () => void;
}) {
  const dotClassName = {
    blue: 'bg-blue-500',
    purple: 'bg-violet-500',
    green: 'bg-emerald-500',
  }[kindTone];

  return (
    <button
      type="button"
      onClick={onClick}
      className={`relative block min-h-[72px] w-full border-b border-gray-200 px-4 py-2.5 text-left transition-colors ${
        active ? 'bg-primary-50' : 'bg-transparent hover:bg-gray-100/80'
      }`}
    >
      {active && (
        <span className="absolute inset-y-2 left-0 w-[3px] rounded-r bg-primary-600" />
      )}
      <span className="mb-1 flex items-center justify-between gap-3">
        <span className="flex items-center gap-1.5 text-[10px] font-bold text-gray-500">
          <span className={`h-1.5 w-1.5 rounded-full ${dotClassName}`} />
          {kind}
        </span>
        <span className="text-[9px] text-gray-400">{time}</span>
      </span>
      <span className="block truncate text-xs font-semibold text-gray-900">
        {title}
      </span>
      <span className="mt-1 block truncate text-[10px] text-gray-500">
        {preview}
      </span>
    </button>
  );
}

export function LibraryEmptyState({ children }: { children: ReactNode }) {
  return (
    <div className="grid min-h-[360px] place-items-center px-8 text-center text-xs text-gray-400">
      {children}
    </div>
  );
}

export function DetailHeader({
  title,
  subtitle,
  actions,
}: {
  title: string;
  subtitle: string;
  actions: ReactNode;
}) {
  return (
    <header className="mb-5 flex items-start justify-between gap-5">
      <div className="min-w-0">
        <h2 className="truncate text-lg font-bold tracking-[-0.02em] text-gray-900">
          {title}
        </h2>
        <p className="mt-1 text-[11px] text-gray-500">{subtitle}</p>
      </div>
      <div className="flex shrink-0 items-center gap-1">{actions}</div>
    </header>
  );
}

export function DetailCard({
  label,
  meta,
  children,
  actions,
}: {
  label: string;
  meta?: string;
  children: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <section className="mb-3 rounded-[10px] border border-gray-200 bg-white p-4 shadow-sm shadow-gray-200/30">
      <div className="mb-3 flex items-center justify-between gap-3 text-[10px] font-bold tracking-[0.04em] text-gray-400">
        <span>{label}</span>
        {meta && <span className="font-medium tracking-normal">{meta}</span>}
      </div>
      {children}
      {actions && (
        <div className="mt-4 flex items-center gap-1.5 border-t border-gray-100 pt-3">
          {actions}
        </div>
      )}
    </section>
  );
}

export function DetailActionButton({
  children,
  onClick,
  tone = 'default',
  title,
  disabled = false,
}: {
  children: ReactNode;
  onClick: () => void;
  tone?: 'default' | 'danger' | 'favorite';
  title: string;
  disabled?: boolean;
}) {
  const toneClassName = {
    default: 'text-gray-500 hover:border-gray-300 hover:text-gray-800',
    danger: 'text-gray-400 hover:border-red-200 hover:bg-red-50 hover:text-red-600',
    favorite: 'text-amber-500 hover:border-amber-200 hover:bg-amber-50',
  }[tone];

  return (
    <button
      type="button"
      title={title}
      aria-label={title}
      disabled={disabled}
      onClick={onClick}
      className={`grid h-8 min-w-8 place-items-center rounded-lg border border-transparent px-2 text-xs transition-colors disabled:cursor-default disabled:opacity-60 ${toneClassName}`}
    >
      {children}
    </button>
  );
}

export function SmallActionButton({
  children,
  onClick,
}: {
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="h-7 rounded-md border border-gray-200 bg-white px-2.5 text-[10px] font-medium text-gray-500 hover:border-gray-300 hover:text-gray-800"
    >
      {children}
    </button>
  );
}
