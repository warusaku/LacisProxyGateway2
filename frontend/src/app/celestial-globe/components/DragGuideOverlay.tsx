// CelestialGlobe v2 — DragGuideOverlay
// mobes2.0 DragGuideOverlay.tsx (135行) 準拠
// ドラッグ中のコンテキストガイド表示

'use client';

import React from 'react';
import { useUIStateStore } from '../stores/useUIStateStore';

export function DragGuideOverlay() {
  const dragMode = useUIStateStore(s => s.dragMode);
  const dropParentNodeId = useUIStateStore(s => s.dropParentNodeId);
  const draggedNodeIds = useUIStateStore(s => s.draggedNodeIds);

  if (draggedNodeIds.length === 0) return null;

  let message: string;
  let bgColor: string;

  if (dropParentNodeId) {
    message = 'Drop to reparent under this device';
    bgColor = 'bg-emerald-600/90';
  } else if (dragMode === 'reparent') {
    message = 'Drag onto a device to change parent';
    bgColor = 'bg-blue-600/90';
  } else if (dragMode === 'free') {
    message = 'Free placement mode';
    bgColor = 'bg-purple-600/90';
  } else {
    message = 'Drag to move';
    bgColor = 'bg-gray-600/90';
  }

  return (
    <div className="absolute top-4 left-1/2 -translate-x-1/2 z-50 pointer-events-none">
      <div className={`animate-float-in ${bgColor} backdrop-blur-sm px-4 py-2 rounded-full shadow-lg`}>
        <span className="text-sm font-medium text-white">{message}</span>
      </div>
    </div>
  );
}
