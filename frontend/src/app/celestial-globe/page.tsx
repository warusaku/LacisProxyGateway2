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
// Error Boundary — クラッシュ時に具体的なエラーメッセージを表示
// ============================================================================

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

class CelestialGlobeErrorBoundary extends React.Component<
  { children: React.ReactNode },
  ErrorBoundaryState
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error('[CelestialGlobe] Render error:', error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col items-center justify-center h-full bg-[#020202] text-gray-200 p-8">
          <div className="max-w-lg text-center">
            <h2 className="text-lg font-bold text-red-400 mb-2">CelestialGlobe Error</h2>
            <p className="text-sm text-gray-400 mb-4">
              {this.state.error?.message ?? 'Unknown error'}
            </p>
            <pre className="text-xs text-gray-500 bg-gray-900 p-4 rounded-lg overflow-auto max-h-48 mb-4 text-left">
              {this.state.error?.stack ?? ''}
            </pre>
            <button
              onClick={() => this.setState({ hasError: false, error: null })}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-sm rounded-md"
            >
              Retry
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

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
    <CelestialGlobeErrorBoundary>
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
    </CelestialGlobeErrorBoundary>
  );
}
