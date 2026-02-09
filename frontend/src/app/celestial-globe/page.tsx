// CelestialGlobe v2 — Page
// mobes2.0 MindMapPage.tsx (243行) 準拠
// ReactFlowProvider でラップ、ViewMode切替、ローディング、エラー

'use client';

import React from 'react';
import { ReactFlowProvider } from 'reactflow';
import { useTopologyStore } from './stores/useTopologyStore';
import { useUIStateStore } from './stores/useUIStateStore';
import { MindMapCanvas } from './components/MindMapCanvas';
import { OutlineView } from './components/OutlineView';
import { PropertyPanel } from './components/PropertyPanel';
import { ViewModeSelector } from './components/ViewModeSelector';
import { LAYOUT } from './constants';
import type { ViewMode } from './types';
import './styles.css';

// ============================================================================
// Page Component
// ============================================================================

export default function CelestialGlobePage() {
  const viewMode = useTopologyStore(s => s.viewMode);
  const setViewMode = useTopologyStore(s => s.setViewMode);
  const error = useTopologyStore(s => s.error);
  const selectedNodeId = useUIStateStore(s => s.selectedNodeId);

  const handleViewModeChange = (mode: ViewMode) => {
    setViewMode(mode);
  };

  return (
    <ReactFlowProvider>
      <div className="flex flex-col h-full bg-[#020202] text-gray-200">
        {/* Top Bar */}
        <div className="flex items-center justify-between px-4 py-2 border-b border-white/10">
          <div className="flex items-center gap-3">
            <h1 className="text-sm font-bold text-gray-200">CelestialGlobe</h1>
          </div>
          <ViewModeSelector mode={viewMode} onChange={handleViewModeChange} />
        </div>

        {/* Error bar */}
        {error && (
          <div className="px-4 py-2 bg-red-900/50 border-b border-red-700/50">
            <span className="text-xs text-red-300">{error}</span>
          </div>
        )}

        {/* Main content area */}
        <div className="flex-1 flex overflow-hidden">
          {/* Outline panel (left) */}
          {(viewMode === 'outline' || viewMode === 'split') && (
            <div
              className="flex-shrink-0 border-r border-white/10"
              style={{ width: LAYOUT.OUTLINE_WIDTH }}
            >
              <OutlineView />
            </div>
          )}

          {/* Canvas (center) */}
          {(viewMode === 'mindmap' || viewMode === 'split') && (
            <div className="flex-1 relative">
              <MindMapCanvas />
            </div>
          )}

          {/* Outline only mode — full width outline */}
          {viewMode === 'outline' && (
            <div className="flex-1" />
          )}

          {/* Property panel (right) */}
          {selectedNodeId && (
            <div className="flex-shrink-0 border-l border-white/10">
              <PropertyPanel />
            </div>
          )}
        </div>
      </div>
    </ReactFlowProvider>
  );
}
