// CelestialGlobe v2 — Tooltip
// mobes2.0 Tooltip.tsx (81行) 準拠
// zoom補正付き。ドラッグ中非表示対応。

'use client';

import React, { useState, useId, useCallback } from 'react';
import { useZoom } from './deviceNode/hooks';

interface TooltipProps {
  children: React.ReactNode;
  content: React.ReactNode;
  disabled?: boolean;
}

export function Tooltip({ children, content, disabled }: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const tooltipId = useId();
  const zoom = useZoom();

  const show = useCallback(() => {
    if (!disabled) setVisible(true);
  }, [disabled]);

  const hide = useCallback(() => {
    setVisible(false);
  }, []);

  if (disabled || !content) {
    return <>{children}</>;
  }

  return (
    <div
      className="relative inline-flex"
      onMouseEnter={show}
      onMouseLeave={hide}
      aria-describedby={visible ? tooltipId : undefined}
    >
      {children}
      {visible && (
        <div
          id={tooltipId}
          role="tooltip"
          className="absolute z-50 pointer-events-none bottom-full left-1/2 mb-2"
          style={{
            transform: `translateX(-50%) scale(${1 / zoom})`,
            transformOrigin: 'bottom center',
          }}
        >
          <div className="cg-glass-card px-3 py-2 text-xs text-gray-200 whitespace-nowrap shadow-lg">
            {content}
          </div>
        </div>
      )}
    </div>
  );
}
