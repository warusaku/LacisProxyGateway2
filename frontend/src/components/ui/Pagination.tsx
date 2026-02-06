'use client';

interface PaginationProps {
  total: number;
  page: number;
  perPage: number;
  onPageChange: (page: number) => void;
}

export function Pagination({ total, page, perPage, onPageChange }: PaginationProps) {
  const totalPages = Math.ceil(total / perPage);

  if (totalPages <= 1) return null;

  const getPageNumbers = () => {
    const pages: (number | '...')[] = [];
    if (totalPages <= 7) {
      for (let i = 1; i <= totalPages; i++) pages.push(i);
    } else {
      pages.push(1);
      if (page > 3) pages.push('...');
      for (let i = Math.max(2, page - 1); i <= Math.min(totalPages - 1, page + 1); i++) {
        pages.push(i);
      }
      if (page < totalPages - 2) pages.push('...');
      pages.push(totalPages);
    }
    return pages;
  };

  return (
    <div className="flex items-center justify-between px-4 py-3">
      <div className="text-sm text-gray-400">
        {total.toLocaleString()} 件中 {((page - 1) * perPage) + 1}–{Math.min(page * perPage, total)} 件表示
      </div>
      <div className="flex items-center gap-1">
        <button
          onClick={() => onPageChange(page - 1)}
          disabled={page <= 1}
          className="px-2 py-1 text-sm rounded border border-border text-gray-400 hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed"
        >
          &lt;
        </button>
        {getPageNumbers().map((p, i) =>
          p === '...' ? (
            <span key={`ellipsis-${i}`} className="px-2 py-1 text-sm text-gray-500">...</span>
          ) : (
            <button
              key={p}
              onClick={() => onPageChange(p)}
              className={`px-3 py-1 text-sm rounded border ${
                p === page
                  ? 'bg-blue-600 border-blue-600 text-white'
                  : 'border-border text-gray-400 hover:bg-gray-800'
              }`}
            >
              {p}
            </button>
          )
        )}
        <button
          onClick={() => onPageChange(page + 1)}
          disabled={page >= totalPages}
          className="px-2 py-1 text-sm rounded border border-border text-gray-400 hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed"
        >
          &gt;
        </button>
      </div>
    </div>
  );
}
