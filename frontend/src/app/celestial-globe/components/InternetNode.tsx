// CelestialGlobe v2 — InternetNode
// mobes2.0 準拠: gradient, border, icon, source handle only

'use client';

import React, { memo } from 'react';
import { Handle, Position } from 'reactflow';
import type { NodeProps } from 'reactflow';
import { Globe } from 'lucide-react';

interface InternetNodeData {
  node?: { label?: string; ip?: string };
  label?: string;
  ip?: string;
}

function InternetNodeInner({ data, selected }: NodeProps<InternetNodeData>) {
  const label = data.node?.label ?? data.label ?? 'Internet';

  return (
    <div
      className={[
        'relative rounded-xl p-4',
        'bg-gradient-to-br from-indigo-100 via-white to-indigo-50',
        'dark:from-indigo-950/60 dark:via-indigo-900/30 dark:to-indigo-800/40',
        'border-2 border-indigo-400 ring-1 ring-indigo-200/70',
        'shadow-lg shadow-indigo-500/10',
        selected ? 'ring-2 ring-blue-400' : '',
      ].join(' ')}
    >
      <div className="flex items-center gap-3">
        <div className="p-2 rounded-full bg-indigo-500/20">
          <Globe className="w-6 h-6 text-indigo-400" />
        </div>
        <div>
          <div className="font-bold text-gray-900 dark:text-gray-100">
            {label}
          </div>
        </div>
      </div>

      {/* Source handle only — Internet → children */}
      <Handle
        type="source"
        position={Position.Right}
        className="!w-3 !h-3 !bg-indigo-400 !border-2 !border-white dark:!border-zinc-800"
      />
    </div>
  );
}

export const InternetNode = memo(InternetNodeInner);
