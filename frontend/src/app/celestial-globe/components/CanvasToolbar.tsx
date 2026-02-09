// CelestialGlobe v2 — CanvasToolbar
// mobes2.0 CanvasToolbar.tsx (313行) 準拠
// デバイスパレット + ガラスモーフィズム

'use client';

import React, { useCallback } from 'react';
import { NetworkDeviceIcon } from './icons';

interface ToolbarItem {
  type: string;
  label: string;
}

const PALETTE_ITEMS: ToolbarItem[] = [
  { type: 'router',       label: 'Router' },
  { type: 'switch',       label: 'Switch' },
  { type: 'ap',           label: 'AP' },
  { type: 'client',       label: 'Client' },
  { type: 'server',       label: 'Server' },
  { type: 'logic_device', label: 'Logic Device' },
];

interface CanvasToolbarProps {
  onAddDevice?: (type: string) => void;
}

export function CanvasToolbar({ onAddDevice }: CanvasToolbarProps) {
  const handleClick = useCallback((type: string) => {
    onAddDevice?.(type);
  }, [onAddDevice]);

  return (
    <div className="absolute top-4 left-4 z-10 pointer-events-none">
      <div className="cg-glass-card p-2 pointer-events-auto">
        <div className="text-xs font-medium text-gray-400 px-2 mb-1">
          Add Device
        </div>
        <div className="flex flex-col gap-1">
          {PALETTE_ITEMS.map((item) => (
            <button
              key={item.type}
              onClick={() => handleClick(item.type)}
              className="flex items-center gap-2 px-3 py-1.5 rounded-md text-sm text-gray-300 hover:bg-white/10 transition-colors"
              title={`Add ${item.label}`}
            >
              <NetworkDeviceIcon type={item.type} size={16} className="text-gray-400" />
              <span>{item.label}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
