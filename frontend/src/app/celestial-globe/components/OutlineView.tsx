// CelestialGlobe v2 — OutlineView
// mobes2.0 OutlineView.tsx (298行) 準拠
// ツリー構造表示、展開/折りたたみ、選択同期

'use client';

import React, { useState, useCallback, useMemo } from 'react';
import { ChevronRight, ChevronDown } from 'lucide-react';
import type { TopologyNodeV2 } from '../types';
import { useTopologyStore } from '../stores/useTopologyStore';
import { useUIStateStore } from '../stores/useUIStateStore';
import { NetworkDeviceIcon } from './icons';
import { getStatusColor, resolveComputedStatus } from './deviceNode/helpers';

// ============================================================================
// Tree Node Type
// ============================================================================

interface TreeItem {
  node: TopologyNodeV2;
  children: TreeItem[];
  depth: number;
}

// ============================================================================
// Build tree from flat nodes
// ============================================================================

function buildOutlineTree(nodes: TopologyNodeV2[]): TreeItem[] {
  const nodeMap = new Map<string, TreeItem>();

  for (const n of nodes) {
    nodeMap.set(n.id, { node: n, children: [], depth: 0 });
  }

  const roots: TreeItem[] = [];

  for (const n of nodes) {
    const item = nodeMap.get(n.id)!;
    if (n.parent_id && nodeMap.has(n.parent_id)) {
      const parent = nodeMap.get(n.parent_id)!;
      parent.children.push(item);
      item.depth = parent.depth + 1;
    } else {
      roots.push(item);
    }
  }

  // Sort children by order
  const sortRecursive = (items: TreeItem[]) => {
    items.sort((a, b) => a.node.order - b.node.order);
    for (const item of items) {
      sortRecursive(item.children);
    }
  };
  sortRecursive(roots);

  // Compute depths
  const computeDepth = (items: TreeItem[], depth: number) => {
    for (const item of items) {
      item.depth = depth;
      computeDepth(item.children, depth + 1);
    }
  };
  computeDepth(roots, 0);

  return roots;
}

// ============================================================================
// Outline Tree Item
// ============================================================================

interface OutlineItemProps {
  item: TreeItem;
  selectedNodeId: string | null;
  expandedIds: Set<string>;
  onToggleExpand: (id: string) => void;
  onSelect: (id: string) => void;
}

function OutlineItem({ item, selectedNodeId, expandedIds, onToggleExpand, onSelect }: OutlineItemProps) {
  const { node, children, depth } = item;
  const isExpanded = expandedIds.has(node.id);
  const isSelected = selectedNodeId === node.id;
  const hasChildren = children.length > 0;
  const computedStatus = resolveComputedStatus(node.state_type, node.status);
  const statusColor = getStatusColor(computedStatus);
  const isOnline = computedStatus.toLowerCase().includes('online');

  return (
    <div>
      <div
        className={[
          'flex items-center gap-1.5 px-2 py-1 rounded-md cursor-pointer transition-colors text-sm',
          isSelected
            ? 'bg-blue-500/20 text-blue-300'
            : 'text-gray-300 hover:bg-white/5',
        ].join(' ')}
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
        onClick={() => onSelect(node.id)}
      >
        {/* Expand/collapse toggle */}
        <button
          onClick={(e) => {
            e.stopPropagation();
            if (hasChildren) onToggleExpand(node.id);
          }}
          className="w-4 h-4 flex items-center justify-center flex-shrink-0"
        >
          {hasChildren ? (
            isExpanded ? (
              <ChevronDown size={12} className="text-gray-500" />
            ) : (
              <ChevronRight size={12} className="text-gray-500" />
            )
          ) : (
            <span className="w-1 h-1 rounded-full bg-gray-600" />
          )}
        </button>

        {/* Status dot */}
        <div
          className={`w-2 h-2 rounded-full flex-shrink-0 ${isOnline ? 'animate-pulse' : ''}`}
          style={{ backgroundColor: statusColor }}
        />

        {/* Icon */}
        <NetworkDeviceIcon type={node.node_type} size={14} className="text-gray-400 flex-shrink-0" />

        {/* Label */}
        <span className="truncate">{node.label}</span>

        {/* Type badge */}
        <span className="text-[10px] text-gray-600 ml-auto flex-shrink-0">
          {node.node_type}
        </span>
      </div>

      {/* Children */}
      {isExpanded && hasChildren && (
        <div>
          {children.map(child => (
            <OutlineItem
              key={child.node.id}
              item={child}
              selectedNodeId={selectedNodeId}
              expandedIds={expandedIds}
              onToggleExpand={onToggleExpand}
              onSelect={onSelect}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// OutlineView
// ============================================================================

export function OutlineView() {
  const nodes = useTopologyStore(s => s.nodes);
  const selectedNodeId = useUIStateStore(s => s.selectedNodeId);
  const selectOnly = useUIStateStore(s => s.selectOnly);

  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());

  const tree = useMemo(() => buildOutlineTree(nodes), [nodes]);

  // Auto-expand all on first render
  React.useEffect(() => {
    const allIds = new Set<string>();
    const walk = (items: TreeItem[]) => {
      for (const item of items) {
        if (item.children.length > 0) allIds.add(item.node.id);
        walk(item.children);
      }
    };
    walk(tree);
    setExpandedIds(allIds);
  }, [tree]);

  const handleToggleExpand = useCallback((id: string) => {
    setExpandedIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const handleSelect = useCallback((id: string) => {
    selectOnly([id]);
  }, [selectOnly]);

  return (
    <div className="h-full cg-glass-panel flex flex-col">
      {/* Header */}
      <div className="p-3 border-b border-white/10">
        <h2 className="text-sm font-semibold text-gray-300">Outline</h2>
        <div className="text-xs text-gray-500 mt-0.5">
          {nodes.length} devices
        </div>
      </div>

      {/* Tree */}
      <div className="flex-1 overflow-y-auto cg-scrollbar py-1">
        {tree.map(item => (
          <OutlineItem
            key={item.node.id}
            item={item}
            selectedNodeId={selectedNodeId}
            expandedIds={expandedIds}
            onToggleExpand={handleToggleExpand}
            onSelect={handleSelect}
          />
        ))}
      </div>
    </div>
  );
}
