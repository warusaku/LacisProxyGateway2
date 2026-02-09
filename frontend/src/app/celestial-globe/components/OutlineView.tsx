'use client';

import { useState, useMemo, useCallback } from 'react';
import {
  Cloud, Globe, GitBranch, Wifi, Monitor, Shield, Box, HardDrive, Server,
  ChevronRight, ChevronDown, Search, AlertTriangle,
  type LucideIcon,
} from 'lucide-react';
import { useTopologyStore } from '../stores/useTopologyStore';
import type { TopologyNodeV2, NodeType } from '../types';

const ICON_MAP: Record<string, LucideIcon> = {
  Cloud, Globe, GitBranch, Wifi, Monitor, Shield, Box, HardDrive, Server,
};

const ICON_FOR_TYPE: Record<NodeType, string> = {
  internet: 'Cloud',
  controller: 'Globe',
  gateway: 'Globe',
  router: 'Globe',
  switch: 'GitBranch',
  ap: 'Wifi',
  client: 'Monitor',
  wg_peer: 'Shield',
  logic_device: 'Box',
  external: 'HardDrive',
  lpg_server: 'Server',
};

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
  const toggleCollapse = useTopologyStore(s => s.toggleCollapse);
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());

  const tree = useMemo(() => buildTree(nodes), [nodes]);
  const filteredTree = useMemo(() => filterTree(tree, searchQuery), [tree, searchQuery]);

  // Statistics
  const stats = useMemo(() => {
    const online = nodes.filter(n => n.status === 'online' || n.status === 'active').length;
    const offline = nodes.filter(n => n.status === 'offline' || n.status === 'inactive').length;
    const total = nodes.length;
    return { total, online, offline };
  }, [nodes]);

  // Auto-expand all on search
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
    <div className="cg-glass-card" style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      {/* Search */}
      <div style={{ padding: '8px 10px', borderBottom: '1px solid rgba(51,51,51,0.5)' }}>
        <div style={{ position: 'relative' }}>
          <Search size={14} style={{ position: 'absolute', left: 8, top: 7, color: '#6B7280' }} />
          <input
            type="text"
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            placeholder="Search nodes..."
            style={{
              width: '100%',
              padding: '5px 8px 5px 28px',
              fontSize: 12,
              background: 'rgba(255,255,255,0.05)',
              border: '1px solid rgba(51,51,51,0.5)',
              borderRadius: 6,
              color: '#E5E7EB',
              outline: 'none',
            }}
          />
        </div>
      </div>

      {/* Stats bar */}
      <div style={{
        display: 'flex',
        gap: 8,
        padding: '6px 12px',
        borderBottom: '1px solid rgba(51,51,51,0.5)',
        fontSize: 10,
        color: '#6B7280',
      }}>
        <span>{stats.total} nodes</span>
        <span style={{ color: '#10B981' }}>{stats.online} online</span>
        <span style={{ color: '#EF4444' }}>{stats.offline} offline</span>
      </div>

      {/* Tree */}
      <div style={{ flex: 1, overflow: 'auto', padding: '4px 4px' }}>
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
  // Orphan: has parent_id but parent not visible (rendered as root)
  const isOrphan = depth === 0 && !!node.parent_id && node.node_type !== 'internet';

  const nodeType = node.node_type as NodeType;
  const iconName = ICON_FOR_TYPE[nodeType] || 'Monitor';
  const IconComponent = ICON_MAP[iconName] || Monitor;

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
            style={{ border: 'none', background: 'none', padding: 0, cursor: 'pointer', color: '#6B7280', display: 'flex' }}
          >
            {isExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
          </button>
        ) : (
          <span style={{ width: 12 }} />
        )}
        <IconComponent size={12} style={{ color: '#9CA3AF', flexShrink: 0 }} />
        <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {node.label}
        </span>
        {isOrphan && (
          <span title="Orphan node (parent not visible)" style={{ display: 'flex', flexShrink: 0 }}>
            <AlertTriangle size={10} style={{ color: '#F59E0B' }} />
          </span>
        )}
        <span className={`cg-status-dot cg-status-dot--${node.status}`} />
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
