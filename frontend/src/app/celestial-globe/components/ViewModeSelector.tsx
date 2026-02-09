'use client';

import type { ViewMode, TopologyViewFilter } from '../types';
import { useTopologyStore } from '../stores/useTopologyStore';

export function ViewModeSelector() {
  const viewMode = useTopologyStore(s => s.viewMode);
  const setViewMode = useTopologyStore(s => s.setViewMode);
  const viewFilter = useTopologyStore(s => s.viewFilter);
  const setViewFilter = useTopologyStore(s => s.setViewFilter);

  const modes: { key: ViewMode; label: string }[] = [
    { key: 'mindmap', label: 'MindMap' },
    { key: 'outline', label: 'Outline' },
    { key: 'split', label: 'Split' },
  ];

  const filters: { key: TopologyViewFilter; label: string }[] = [
    { key: 'full', label: 'All' },
    { key: 'routes', label: 'Routes' },
  ];

  return (
    <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
      <div className="cg-view-mode">
        {modes.map(m => (
          <button
            key={m.key}
            className={`cg-view-mode-btn ${viewMode === m.key ? 'cg-view-mode-btn--active' : ''}`}
            onClick={() => setViewMode(m.key)}
          >
            {m.label}
          </button>
        ))}
      </div>
      <div className="cg-view-mode">
        {filters.map(f => (
          <button
            key={f.key}
            className={`cg-view-mode-btn ${viewFilter === f.key ? 'cg-view-mode-btn--active' : ''}`}
            onClick={() => setViewFilter(f.key)}
          >
            {f.label}
          </button>
        ))}
      </div>
    </div>
  );
}
