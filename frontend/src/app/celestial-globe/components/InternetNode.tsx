'use client';

/**
 * InternetNode Component — mobes2.0 準拠 Tailwind CSS 化
 *
 * mobes2.0 仕様:
 *   - Cloud SVGアイコン、中央配置
 *   - bg-gradient-to-br from-indigo-100 ... dark:from-indigo-950/60
 *   - border-2 border-indigo-400 ring-1 ring-indigo-200/70
 *   - Handle source (Position.Right) のみ — 逆進禁止
 */

import { memo } from 'react';
import { Handle, Position, type NodeProps } from 'reactflow';
import { NetworkDeviceIcon } from './icons';
import type { InternetNodeData } from '../types';

function InternetNodeComponent({ data, selected }: NodeProps<InternetNodeData>) {
  const selectionClass = selected
    ? 'mindmap-selection-pulse-strong border-2 border-primary-500 ring-2 ring-primary-200 dark:ring-primary-900'
    : 'border-2 border-indigo-400 ring-1 ring-indigo-200/70 shadow-[0_0_18px_rgba(99,102,241,0.35)]';

  return (
    <div
      className={`
        relative rounded-lg p-4 min-w-[120px] text-center transition-all cursor-pointer
        bg-gradient-to-br from-indigo-100 via-white to-indigo-50
        dark:from-indigo-950/60 dark:via-indigo-900/30 dark:to-indigo-800/40
        ${selectionClass}
      `}
    >
      {/* Cloud Icon */}
      <div className="flex items-center justify-center mb-2">
        <NetworkDeviceIcon type="internet" className="w-10 h-10 text-blue-500" />
      </div>

      {/* Label */}
      <div className="font-semibold text-sm text-gray-900 dark:text-gray-100">
        {data.label || 'Internet'}
      </div>

      {/* IP */}
      {data.ip && (
        <div className="font-mono text-xs text-gray-500 dark:text-gray-400 mt-1">
          {data.ip}
        </div>
      )}

      {/* Source handle ONLY (right) — target handle は意図的に無い */}
      <Handle
        type="source"
        position={Position.Right}
        className="!w-2.5 !h-2.5 !bg-blue-500 !border-2 !border-white dark:!border-dark-100"
      />
    </div>
  );
}

export const InternetNode = memo(InternetNodeComponent);
