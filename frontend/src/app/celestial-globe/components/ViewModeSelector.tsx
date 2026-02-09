// CelestialGlobe v2 — ViewModeSelector
// mindmap/outline/split 切替

'use client';

import React from 'react';
import { LayoutGrid, List, Columns } from 'lucide-react';
import type { ViewMode } from '../types';

interface ViewModeSelectorProps {
  mode: ViewMode;
  onChange: (mode: ViewMode) => void;
}

const MODES: { value: ViewMode; label: string; icon: React.ReactNode }[] = [
  { value: 'mindmap', label: 'Canvas', icon: <LayoutGrid size={14} /> },
  { value: 'outline', label: 'Outline', icon: <List size={14} /> },
  { value: 'split', label: 'Split', icon: <Columns size={14} /> },
];

export function ViewModeSelector({ mode, onChange }: ViewModeSelectorProps) {
  return (
    <div className="flex items-center gap-0.5 cg-glass-card p-1">
      {MODES.map(m => (
        <button
          key={m.value}
          onClick={() => onChange(m.value)}
          className={[
            'flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-colors',
            mode === m.value
              ? 'bg-blue-600/80 text-white'
              : 'text-gray-400 hover:bg-white/10 hover:text-gray-200',
          ].join(' ')}
        >
          {m.icon}
          {m.label}
        </button>
      ))}
    </div>
  );
}
