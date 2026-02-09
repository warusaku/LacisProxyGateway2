'use client';

import { useEffect, useState, useCallback } from 'react';
import { useTopologyStore } from './stores/useTopologyStore';
import { MindMapCanvas } from './components/MindMapCanvas';
import { OutlineView } from './components/OutlineView';
import { PropertyPanel } from './components/PropertyPanel';
import { ViewModeSelector } from './components/ViewModeSelector';
import { LogicDeviceDialog } from './components/LogicDeviceDialog';
import { LAYOUT } from './constants';
import './styles.css';

export default function CelestialGlobePage() {
  const fetchTopology = useTopologyStore(s => s.fetchTopology);
  const loading = useTopologyStore(s => s.loading);
  const error = useTopologyStore(s => s.error);
  const viewMode = useTopologyStore(s => s.viewMode);
  const metadata = useTopologyStore(s => s.metadata);
  const selectedNodeId = useTopologyStore(s => s.selectedNodeId);

  const [logicDialogOpen, setLogicDialogOpen] = useState(false);

  useEffect(() => {
    fetchTopology();
  }, [fetchTopology]);

  const openLogicDialog = useCallback(() => setLogicDialogOpen(true), []);
  const closeLogicDialog = useCallback(() => setLogicDialogOpen(false), []);

  if (loading && !metadata) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        Loading topology...
      </div>
    );
  }

  if (error && !metadata) {
    return (
      <div className="p-5">
        <div className="cg-glass-card p-5">
          <div className="text-red-500 mb-2">Error: {error}</div>
          <button
            onClick={() => fetchTopology()}
            className="text-blue-400 bg-transparent border-none cursor-pointer underline"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex justify-between items-center pb-3 flex-shrink-0">
        <div className="flex items-center gap-4">
          <h2 className="text-xl font-bold m-0 text-gray-200">CelestialGlobe</h2>
          {metadata && (
            <div className="flex gap-3 text-[11px] text-gray-500">
              <span><strong className="text-blue-400">{metadata.total_devices}</strong> Devices</span>
              <span><strong className="text-gray-400">{metadata.total_clients}</strong> Clients</span>
              <span><strong className="text-indigo-400">{metadata.controllers}</strong> Controllers</span>
              <span><strong className="text-emerald-400">{metadata.routers}</strong> Routers</span>
              {metadata.logic_devices > 0 && (
                <span><strong className="text-gray-400">{metadata.logic_devices}</strong> Logic</span>
              )}
            </div>
          )}
        </div>
        <ViewModeSelector />
      </div>

      {/* Main content area */}
      <div className="flex-1 flex gap-0 overflow-hidden min-h-0">
        {/* Outline panel (left) */}
        {(viewMode === 'outline' || viewMode === 'split') && (
          <div
            className="flex-shrink-0 overflow-hidden"
            style={{
              width: viewMode === 'outline' ? '100%' : LAYOUT.OUTLINE_WIDTH,
              borderRight: viewMode === 'split' ? '1px solid rgba(51,51,51,0.5)' : undefined,
            }}
          >
            <OutlineView />
          </div>
        )}

        {/* Canvas (center) */}
        {(viewMode === 'mindmap' || viewMode === 'split') && (
          <div className="flex-1 min-w-0 overflow-hidden">
            <MindMapCanvas onAddLogicDevice={openLogicDialog} />
          </div>
        )}

        {/* Property panel (right) */}
        {selectedNodeId && viewMode !== 'outline' && (
          <div
            className="flex-shrink-0 overflow-hidden border-l border-dark-300/50"
            style={{ width: LAYOUT.PROPERTY_WIDTH }}
          >
            <PropertyPanel />
          </div>
        )}
      </div>

      {/* LogicDevice dialog */}
      <LogicDeviceDialog open={logicDialogOpen} onClose={closeLogicDialog} />
    </div>
  );
}
