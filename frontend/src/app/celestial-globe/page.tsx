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
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: '#6B7280' }}>
        Loading topology...
      </div>
    );
  }

  if (error && !metadata) {
    return (
      <div style={{ padding: 20 }}>
        <div className="cg-glass-card" style={{ padding: 20 }}>
          <div style={{ color: '#EF4444', marginBottom: 8 }}>Error: {error}</div>
          <button
            onClick={() => fetchTopology()}
            style={{ color: '#60A5FA', background: 'none', border: 'none', cursor: 'pointer', textDecoration: 'underline' }}
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '0 0 12px 0', flexShrink: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <h2 style={{ fontSize: 20, fontWeight: 700, margin: 0, color: '#E5E7EB' }}>CelestialGlobe</h2>
          {metadata && (
            <div style={{ display: 'flex', gap: 12, fontSize: 11, color: '#6B7280' }}>
              <span><strong style={{ color: '#60A5FA' }}>{metadata.total_devices}</strong> Devices</span>
              <span><strong style={{ color: '#9CA3AF' }}>{metadata.total_clients}</strong> Clients</span>
              <span><strong style={{ color: '#818CF8' }}>{metadata.controllers}</strong> Controllers</span>
              <span><strong style={{ color: '#34D399' }}>{metadata.routers}</strong> Routers</span>
              {metadata.logic_devices > 0 && (
                <span><strong style={{ color: '#9CA3AF' }}>{metadata.logic_devices}</strong> Logic</span>
              )}
            </div>
          )}
        </div>
        <ViewModeSelector />
      </div>

      {/* Main content area */}
      <div style={{ flex: 1, display: 'flex', gap: 0, overflow: 'hidden', minHeight: 0 }}>
        {/* Outline panel (left) */}
        {(viewMode === 'outline' || viewMode === 'split') && (
          <div style={{
            width: viewMode === 'outline' ? '100%' : LAYOUT.OUTLINE_WIDTH,
            flexShrink: 0,
            overflow: 'hidden',
            borderRight: viewMode === 'split' ? '1px solid rgba(51,51,51,0.5)' : undefined,
          }}>
            <OutlineView />
          </div>
        )}

        {/* Canvas (center) */}
        {(viewMode === 'mindmap' || viewMode === 'split') && (
          <div style={{ flex: 1, minWidth: 0, overflow: 'hidden' }}>
            <MindMapCanvas onAddLogicDevice={openLogicDialog} />
          </div>
        )}

        {/* Property panel (right) */}
        {selectedNodeId && viewMode !== 'outline' && (
          <div style={{
            width: LAYOUT.PROPERTY_WIDTH,
            flexShrink: 0,
            overflow: 'hidden',
            borderLeft: '1px solid rgba(51,51,51,0.5)',
          }}>
            <PropertyPanel />
          </div>
        )}
      </div>

      {/* LogicDevice dialog */}
      <LogicDeviceDialog open={logicDialogOpen} onClose={closeLogicDialog} />
    </div>
  );
}
