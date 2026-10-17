interface HistoryPaginationProps {
  page: number;
  pageSize: number;
  total: number;
  onPageChange(page: number): void;
}

export function HistoryPagination({
  page,
  pageSize,
  total,
  onPageChange,
}: HistoryPaginationProps) {
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  if (total <= pageSize) return null;

  return (
    <div className="flex items-center justify-between pt-4 border-t border-line">
      <span className="text-sm text-muted">
        共 {total} 条，第 {page + 1}/{pageCount} 页
      </span>
      <div className="flex gap-2">
        <button
          type="button"
          disabled={page === 0}
          onClick={() => onPageChange(page - 1)}
          className="px-3 py-1.5 text-sm rounded-lg border border-line disabled:opacity-40 hover:bg-sunken"
        >
          上一页
        </button>
        <button
          type="button"
          disabled={page + 1 >= pageCount}
          onClick={() => onPageChange(page + 1)}
          className="px-3 py-1.5 text-sm rounded-lg border border-line disabled:opacity-40 hover:bg-sunken"
        >
          下一页
        </button>
      </div>
    </div>
  );
}
