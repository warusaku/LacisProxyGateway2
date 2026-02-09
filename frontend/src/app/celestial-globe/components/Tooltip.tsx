'use client';

/**
 * Tooltip Component
 * SSoT: mobes2.0 Tooltip.tsx 完全移植
 * ズーム補正: scale(1/zoom) で常に100%サイズ表示
 */

import React, { useId } from 'react';

interface TooltipProps {
  label: React.ReactNode;
  children: React.ReactNode;
  position?: 'top' | 'bottom' | 'left' | 'right';
  className?: string;
  zoom?: number;
  disabled?: boolean;
}

const POSITION_CLASSES: Record<NonNullable<TooltipProps['position']>, string> = {
  top: 'bottom-full left-1/2',
  bottom: 'top-full left-1/2',
  left: 'left-0 top-1/2 -ml-2',
  right: 'right-0 top-1/2 ml-2',
};

const getTransform = (position: NonNullable<TooltipProps['position']>, zoom: number) => {
  const inverseScale = zoom > 0 ? 1 / zoom : 1;

  switch (position) {
    case 'top':
      return `translate(-50%, -0.5rem) scale(${inverseScale})`;
    case 'bottom':
      return `translate(-50%, 0.5rem) scale(${inverseScale})`;
    case 'left':
      return `translate(-100%, -50%) scale(${inverseScale})`;
    case 'right':
      return `translate(100%, -50%) scale(${inverseScale})`;
  }
};

const Tooltip: React.FC<TooltipProps> = ({ label, children, position = 'top', className, zoom = 1.0, disabled = false }) => {
  const positionClass = POSITION_CLASSES[position];
  const tooltipId = useId();

  const tooltipStyle = {
    transform: getTransform(position, zoom),
    transformOrigin: position === 'top' ? 'bottom center' :
                      position === 'bottom' ? 'top center' :
                      position === 'left' ? 'right center' : 'left center',
  };

  const wrappedChild = React.isValidElement(children)
    ? React.cloneElement(children, {
        ...(children.props as Record<string, unknown>),
        'aria-describedby': [
          (children.props as Record<string, unknown>)?.['aria-describedby'],
          tooltipId,
        ]
          .filter(Boolean)
          .join(' ') || undefined,
      } as Record<string, unknown>)
    : children;

  return (
    <div className={`relative inline-flex group ${className ?? ''}`}>
      {wrappedChild}
      {!disabled && (
        <div
          role="tooltip"
          id={tooltipId}
          className={`pointer-events-none absolute z-[9999] max-w-xs min-w-[180px] whitespace-normal break-words rounded-md bg-gray-900 px-3 py-2 text-[10px] font-medium text-white opacity-0 shadow-lg transition-opacity duration-150 group-hover:opacity-100 group-focus-within:opacity-100 ${positionClass}`}
          style={tooltipStyle}
        >
          {label}
        </div>
      )}
    </div>
  );
};

export default Tooltip;
