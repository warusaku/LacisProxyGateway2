'use client';

import { useState, useMemo, useCallback } from 'react';
import { NetworkDeviceIcon } from './icons';
import { useTopologyStore } from '../stores/useTopologyStore';
import type { TopologyNodeV2 } from '../types';
import { STATUS_COLORS } from '../constants';

interface TreeNode {
  node: TopologyNodeV2;
  children: TreeNode[];
}

function buildTree(nodes: TopologyNodeV2[]): TreeNode[] {
  const nodeMap = new Map<string, TreeNode>();
  for (const n of nodes) {
    nodeMap.set(n.id, { node: n, children: [] });
  }

  const roots: TreeNode[] = [];
  for (const n of nodes) {
    const treeNode = nodeMap.get(n.id)!;
    if (n.parent_id && nodeMap.has(n.parent_id)) {
      nodeMap.get(n.parent_id)!.children.push(treeNode);
    } else {
      roots.push(treeNode);
    }
  }
  return roots;
}

function filterTree(tree: TreeNode[], query: string): TreeNode[] {
  if (!query) return tree;
  const q = query.toLowerCase();

  function matches(tn: TreeNode): boolean {
    const n = tn.node;
    if (n.label.toLowerCase().includes(q)) return true;
    if (n.ip?.toLowerCase().includes(q)) return true;
    if (n.mac?.toLowerCase().includes(q)) return true;
    return tn.children.some(c => matches(c));
  }

  return tree
    .filter(tn => matches(tn))
    .map(tn => ({
      ...tn,
      children: filterTree(tn.children, query),
    }));
}

export function OutlineView() {
  const nodes = useTopologyStore(s => s.nodes);
  const selectedNodeId = useTopologyStore(s => s.selectedNodeId);
  const setSelectedNodeId = useTopologyStore(s => s.setSelectedNodeId);
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());

  const tree = useMemo(() => buildTree(nodes), [nodes]);
  const filteredTree = useMemo(() => filterTree(tree, searchQuery), [tree, searchQuery]);

  const stats = useMemo(() => {
    const online = nodes.filter(n => n.status === 'online' || n.status === 'active').length;
    const offline = nodes.filter(n => n.status === 'offline' || n.status === 'inactive').length;
    const total = nodes.length;
    return { total, online, offline };
  }, [nodes]);

  useMemo(() => {
    if (searchQuery) {
      const allIds = new Set(nodes.map(n => n.id));
      setExpandedIds(allIds);
    }
  }, [searchQuery, nodes]);

  const toggleExpand = useCallback((id: string) => {
    setExpandedIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  return (
    <div className="cg-glass-card w-full h-full flex flex-col overflow-hidden">
      {/* Search */}
      <div className="px-2.5 py-2 border-b border-dark-300/50">
        <div className="relative">
          <svg className="absolute left-2 top-1.5 w-3.5 h-3.5 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <input
            type="text"
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            placeholder="Search nodes..."
            className="w-full py-1.5 pl-7 pr-2 text-xs bg-white/5 border border-dark-300/50 rounded-md text-gray-200 outline-none focus:border-primary-500/50"
          />
        </div>
      </div>

      {/* Stats bar */}
      <div className="flex gap-2 px-3 py-1.5 border-b border-dark-300/50 text-[10px] text-gray-500">
        <span>{stats.total} nodes</span>
        <span className="text-emerald-500">{stats.online} online</span>
        <span className="text-red-500">{stats.offline} offline</span>
      </div>

      {/* Tree */}
      <div className="flex-1 overflow-auto px-1 py-1">
        {filteredTree.map(tn => (
          <TreeItem
            key={tn.node.id}
            treeNode={tn}
            depth={0}
            expandedIds={expandedIds}
            selectedId={selectedNodeId}
            onToggleExpand={toggleExpand}
            onSelect={setSelectedNodeId}
          />
        ))}
      </div>
    </div>
  );
}

function TreeItem({
  treeNode,
  depth,
  expandedIds,
  selectedId,
  onToggleExpand,
  onSelect,
}: {
  treeNode: TreeNode;
  depth: number;
  expandedIds: Set<string>;
  selectedId: string | null;
  onToggleExpand: (id: string) => void;
  onSelect: (id: string) => void;
}) {
  const { node, children } = treeNode;
  const hasChildren = children.length > 0;
  const isExpanded = expandedIds.has(node.id);
  const isSelected = node.id === selectedId;
  const isOrphan = depth === 0 && !!node.parent_id && node.node_type !== 'internet';

  const statusColor = STATUS_COLORS[node.state_type] || STATUS_COLORS.unknown || '#9CA3AF';

  return (
    <div>
      <div
        className={`cg-outline-item ${isSelected ? 'cg-outline-item--selected' : ''}`}
        style={{ paddingLeft: 8 + depth * 16 }}
        onClick={() => onSelect(node.id)}
      >
        {hasChildren ? (
          <button
            onClick={e => { e.stopPropagation(); onToggleExpand(node.id); }}
            className="border-none bg-none p-0 cursor-pointer text-gray-500 flex"
          >
            <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              {isExpanded ? (
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
              ) : (
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
              )}
            </svg>
          </button>
        ) : (
          <span className="w-3" />
        )}
        <NetworkDeviceIcon nodeType={node.node_type} className="w-3 h-3 text-gray-400 flex-shrink-0" />
        <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
          {node.label}
        </span>
        {isOrphan && (
          <span title="Orphan node (parent not visible)" className="flex flex-shrink-0">
            <svg className="w-2.5 h-2.5 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" />
            </svg>
          </span>
        )}
        {/* Status dot with computed color */}
        <span
          className="w-2 h-2 rounded-full inline-block flex-shrink-0"
          style={{ backgroundColor: statusColor }}
        />
      </div>
      {hasChildren && isExpanded && (
        <div>
          {children.map(child => (
            <TreeItem
              key={child.node.id}
              treeNode={child}
              depth={depth + 1}
              expandedIds={expandedIds}
              selectedId={selectedId}
              onToggleExpand={onToggleExpand}
              onSelect={onSelect}
            />
          ))}
        </div>
      )}
    </div>
  );
}
