/**
 * Tooltip Component — mobes2.0 Tooltip.tsx 忠実移植
 *
 * zoom補正: scale(1/zoom) で常に同じサイズで表示
 * group-hover:opacity-100 でホバー時のみ表示
 */

'use client';

import { type ReactNode } from 'react';

interface TooltipProps {
  children: ReactNode;
  zoom?: number;
}

export function Tooltip({ children, zoom = 1 }: TooltipProps) {
  const scale = Math.min(Math.max(1 / zoom, 0.5), 3);

  return (
    <div
      className="
        absolute left-1/2 -bottom-2 translate-y-full -translate-x-1/2
        opacity-0 group-hover:opacity-100 transition-opacity duration-150
        pointer-events-none z-50
      "
      style={{ transform: `translateX(-50%) translateY(100%) scale(${scale})` }}
    >
      <div
        className="
          bg-dark-900/95 backdrop-blur-sm border border-dark-700
          rounded-lg shadow-xl px-3 py-2 text-xs text-dark-200
          whitespace-nowrap
        "
      >
        {children}
      </div>
    </div>
  );
}
