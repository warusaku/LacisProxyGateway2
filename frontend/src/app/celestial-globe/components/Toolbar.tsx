'use client';

import { Maximize, RefreshCw, Plus } from 'lucide-react';
import { useReactFlow } from 'reactflow';
import { useTopologyStore } from '../stores/useTopologyStore';

interface ToolbarProps {
  onAddLogicDevice: () => void;
}

export function Toolbar({ onAddLogicDevice }: ToolbarProps) {
  const { fitView } = useReactFlow();
  const loading = useTopologyStore(s => s.loading);
  const fetchTopology = useTopologyStore(s => s.fetchTopology);

  return (
    <div className="cg-toolbar" style={{ position: 'absolute', top: 12, right: 12, zIndex: 10 }}>
      <button
        className="cg-toolbar-btn"
        onClick={() => fetchTopology()}
        title="Refresh"
        disabled={loading}
      >
        <RefreshCw size={16} className={loading ? 'animate-spin' : ''} />
      </button>
      <button className="cg-toolbar-btn" onClick={() => fitView({ padding: 0.2 })} title="Fit to view">
        <Maximize size={16} />
      </button>
      <button className="cg-toolbar-btn" onClick={onAddLogicDevice} title="Add LogicDevice">
        <Plus size={16} />
      </button>
    </div>
  );
}
