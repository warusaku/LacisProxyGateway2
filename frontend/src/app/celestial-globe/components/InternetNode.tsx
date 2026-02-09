/**
 * InternetNode Component — mobes2.0 L195-196, L688-714 ベース
 *
 * Internet 専用描画:
 *   - indigo gradient background
 *   - border-2 border-indigo-400 ring-1 ring-indigo-200/70
 *   - Source handle (Position.Right) のみ → 逆進禁止を構造的に強制
 */

'use client';

import { memo } from 'react';
import { Handle, Position } from 'reactflow';
import type { NodeProps } from 'reactflow';
import type { InternetNodeData } from '../types';
import { useZoom } from './deviceNode/hooks';

export const InternetNode = memo(({ data, selected }: NodeProps<InternetNodeData>) => {
  const zoom = useZoom();
  const isMinimalZoom = zoom < 0.4;

  return (
    <>
      <div
        className={`
          mindmap-node group relative rounded-lg shadow-lg p-3 transition-all
          bg-gradient-to-br from-indigo-100 via-white to-indigo-50
          dark:from-indigo-950/60 dark:via-indigo-900/30 dark:to-indigo-800/40
          border-2 border-indigo-400 ring-1 ring-indigo-200/70
          dark:border-indigo-500/60 dark:ring-indigo-500/20
          ${selected ? 'mindmap-selection-pulse-strong ring-2 ring-primary-400/60' : ''}
        `}
        style={{ minWidth: isMinimalZoom ? 40 : 120 }}
      >
        {isMinimalZoom ? (
          /* Minimal: just a globe icon */
          <div className="flex items-center justify-center p-1">
            <svg
              className="w-5 h-5 text-indigo-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
            >
              <path d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z" />
            </svg>
          </div>
        ) : (
          /* Normal: icon + label */
          <div className="flex items-center gap-2">
            <svg
              className="w-6 h-6 text-indigo-400 dark:text-indigo-300 shrink-0"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
            >
              <path d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z" />
            </svg>
            <div>
              <div className="text-sm font-semibold text-indigo-700 dark:text-indigo-200">
                {data.label}
              </div>
              {data.ip && (
                <div className="text-xs text-indigo-500 dark:text-indigo-400">
                  {data.ip}
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Source handle only (right) — no target handle = structurally prevents reverse edges */}
      <Handle
        type="source"
        position={Position.Right}
        className="!w-3 !h-3 !bg-indigo-400 dark:!bg-indigo-500 !border-2 !border-white dark:!border-dark-100"
      />
    </>
  );
});

InternetNode.displayName = 'InternetNode';
