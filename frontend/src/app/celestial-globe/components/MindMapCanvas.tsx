/**
 * MindMapCanvas Component — DFSレイアウト完全修正版
 *
 * 前回の問題: getNodeHeight() の値が実際の CSS 描画サイズと乖離
 * 今回の修正:
 *   - p-3 = 12px * 2 = 24px padding
 *   - border = 1px or 2px (logic_device) → 2-4px total
 *   - 各要素の実際の行高を正確に積算
 *   - V_GAP = 28px (badge が -top-2 で上にはみ出すため余裕確保)
 *   - H_SPACING = 260px
 *
 * ReactFlow設定 — mobes2.0 MindMapCanvas.tsx L744-813 準拠:
 *   - minZoom=0.1, maxZoom=2
 *   - fitView, fitViewOptions: { padding: 0.2 }
 *   - Background: Dots, gap=24, size=1, color="#334155"
 *   - MiniMap: pannable, zoomable
 */

'use client';

import { useCallback, useMemo } from 'react';
import ReactFlow, {
  Background,
  MiniMap,
  ReactFlowProvider,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
  type NodeTypes,
  type EdgeTypes,
  type NodeMouseHandler,
  BackgroundVariant,
} from 'reactflow';
import 'reactflow/dist/style.css';

import { DeviceNode } from './DeviceNode';
import { InternetNode } from './InternetNode';
import { TopologyEdge } from './TopologyEdge';
import { Toolbar } from './Toolbar';
import { Legend } from './Legend';
import { useTopologyStore } from '../stores/useTopologyStore';
import type {
  TopologyNodeV2,
  TopologyEdgeV2,
  DeviceNodeData,
  InternetNodeData,
  EdgeType,
  TopologyEdgeData,
} from '../types';

// nodeTypes: device + internet
const nodeTypes: NodeTypes = {
  device: DeviceNode,
  internet: InternetNode,
};

// edgeTypes: custom TopologyEdge
const edgeTypes: EdgeTypes = {
  topology: TopologyEdge,
};

const defaultEdgeOptions = {
  type: 'topology',
  animated: false,
};

// ============================================================================
// DFS deterministic layout — p-3 ベース正確計算
// ============================================================================

const H_SPACING = 260;  // horizontal spacing per depth level
const V_GAP = 28;       // vertical gap between siblings (badge はみ出し考慮)

/**
 * Get node height based on node_type — p-3 ベース正確計算
 *
 * p-3 = 12px * 2 = 24px padding
 * border-1 = 1px * 2 = 2px (normal) or border-2 = 2px * 2 = 4px (logic_device)
 *
 * Infrastructure (gateway/router/switch/ap/external/lpg_server):
 *   dot(16) + label(20) + IP(18) + MAC(18) + padding(24) + border(2) = ~98 → 100
 * Client/WG_peer:
 *   dot(16) + label(20) + IP(18) + padding(24) + border(2) = ~80 → 82
 * Internet:
 *   icon(24) + label(20) + padding(24) + border(4) = ~72
 * Logic device:
 *   same as infrastructure + border-2 = 100
 */
function getNodeHeight(nodeType?: string): number {
  if (nodeType === 'internet') return 72;
  if (nodeType === 'client' || nodeType === 'wg_peer') return 82;
  return 100; // gateway, router, switch, ap, external, lpg_server, logic_device, controller
}

/**
 * Compute deterministic DFS layout from topology nodes.
 * Each node's position = f(parent_id, order). O(n).
 */
function computeDfsLayout(nodes: TopologyNodeV2[]): Map<string, { x: number; y: number }> {
  const childrenMap = new Map<string, TopologyNodeV2[]>();
  const nodeById = new Map<string, TopologyNodeV2>();

  for (const n of nodes) {
    nodeById.set(n.id, n);
    const pid = n.parent_id || '__internet__';
    if (pid === n.id) continue; // Skip self-referencing
    if (!childrenMap.has(pid)) childrenMap.set(pid, []);
    childrenMap.get(pid)!.push(n);
  }

  // Sort children by order
  childrenMap.forEach((children) => {
    children.sort((a, b) => a.order - b.order);
  });

  const positions = new Map<string, { x: number; y: number }>();
  let cursorY = 0;

  function dfs(nodeId: string, depth: number): [number, number] {
    const children = childrenMap.get(nodeId) || [];
    const x = depth * H_SPACING;
    const node = nodeById.get(nodeId);
    const h = getNodeHeight(node?.node_type);

    if (children.length === 0) {
      positions.set(nodeId, { x, y: cursorY });
      const startY = cursorY;
      cursorY += h + V_GAP;
      return [startY, startY + h];
    }

    let firstStart = Infinity;
    let lastEnd = 0;
    for (const child of children) {
      const [s, e] = dfs(child.id, depth + 1);
      if (s < firstStart) firstStart = s;
      if (e > lastEnd) lastEnd = e;
    }

    positions.set(nodeId, { x, y: (firstStart + lastEnd) / 2 - h / 2 });
    return [firstStart, lastEnd];
  }

  dfs('__internet__', 0);

  return positions;
}

/** topology nodes → ReactFlow nodes */
function toFlowNodes(
  topoNodes: TopologyNodeV2[],
  positions: Map<string, { x: number; y: number }>,
  selectedId: string | null,
  onCollapse: (id: string) => void,
  onLabelEdit: (nodeId: string, newLabel: string) => void
): Node[] {
  return topoNodes.map(n => {
    const pos = positions.get(n.id) || { x: 0, y: 0 };

    if (n.node_type === 'internet') {
      return {
        id: n.id,
        type: 'internet',
        position: pos,
        data: {
          label: n.label,
          ip: n.ip,
        } satisfies InternetNodeData,
        draggable: false,
      };
    }

    return {
      id: n.id,
      type: 'device',
      position: pos,
      data: {
        node: n,
        selected: n.id === selectedId,
        onCollapse,
        onLabelEdit,
      } satisfies DeviceNodeData,
      draggable: false,
    };
  });
}

/** topology edges → ReactFlow edges */
function toFlowEdges(topoEdges: TopologyEdgeV2[]): Edge[] {
  return topoEdges.map((e, i) => ({
    id: `edge-${i}-${e.from}-${e.to}`,
    source: e.from,
    target: e.to,
    type: 'topology',
    data: {
      connectionType: e.edge_type as EdgeType,
      label: e.label,
      animated: e.edge_type === 'wireless' || e.edge_type === 'route',
    } satisfies TopologyEdgeData,
  }));
}

// ============================================================================
// Canvas component
// ============================================================================

interface MindMapCanvasInnerProps {
  onAddLogicDevice: () => void;
}

function MindMapCanvasInner({ onAddLogicDevice }: MindMapCanvasInnerProps) {
  const topoNodes = useTopologyStore(s => s.nodes);
  const topoEdges = useTopologyStore(s => s.edges);
  const selectedNodeId = useTopologyStore(s => s.selectedNodeId);
  const setSelectedNodeId = useTopologyStore(s => s.setSelectedNodeId);
  const toggleCollapse = useTopologyStore(s => s.toggleCollapse);
  const updateNodeLabel = useTopologyStore(s => s.updateNodeLabel);

  const onCollapse = useCallback((id: string) => {
    toggleCollapse(id);
  }, [toggleCollapse]);

  const onLabelEdit = useCallback((nodeId: string, newLabel: string) => {
    updateNodeLabel(nodeId, newLabel);
  }, [updateNodeLabel]);

  const positions = useMemo(() => computeDfsLayout(topoNodes), [topoNodes]);

  const flowNodes = useMemo(
    () => toFlowNodes(topoNodes, positions, selectedNodeId, onCollapse, onLabelEdit),
    [topoNodes, positions, selectedNodeId, onCollapse, onLabelEdit]
  );
  const flowEdges = useMemo(() => toFlowEdges(topoEdges), [topoEdges]);

  const [nodes, setNodes, onNodesChange] = useNodesState(flowNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(flowEdges);

  // Sync when store data changes
  useMemo(() => { setNodes(flowNodes); }, [flowNodes, setNodes]);
  useMemo(() => { setEdges(flowEdges); }, [flowEdges, setEdges]);

  const onNodeClick: NodeMouseHandler = useCallback((_event, node) => {
    setSelectedNodeId(node.id);
  }, [setSelectedNodeId]);

  const onPaneClick = useCallback(() => {
    setSelectedNodeId(null);
  }, [setSelectedNodeId]);

  // MiniMap node color
  const getNodeColor = useCallback((n: Node): string => {
    if (n.type === 'internet') return '#3B82F6';
    const data = n.data as DeviceNodeData | undefined;
    if (!data?.node) return '#333';
    const st = data.node.state_type;
    if (st === 'online' || st === 'StaticOnline') return '#10B981';
    if (st === 'offline') return '#EF4444';
    if (st === 'StaticOffline') return '#9CA3AF';
    return '#9CA3AF';
  }, []);

  return (
    <div className="cg-canvas" style={{ width: '100%', height: '100%', position: 'relative' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={onNodeClick}
        onPaneClick={onPaneClick}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        defaultEdgeOptions={defaultEdgeOptions}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.1}
        maxZoom={2}
        proOptions={{ hideAttribution: true }}
      >
        {/* mobes2.0 準拠: Dots background */}
        <Background color="#334155" gap={24} size={1} variant={BackgroundVariant.Dots} />
        {/* mobes2.0 準拠: MiniMap */}
        <MiniMap
          nodeColor={getNodeColor}
          nodeStrokeWidth={2}
          zoomable
          pannable
          maskColor="rgba(0,0,0,0.7)"
          style={{
            background: 'rgba(10,10,10,0.9)',
            border: '1px solid rgba(51,51,51,0.5)',
            borderRadius: 8,
          }}
        />
      </ReactFlow>
      <Toolbar onAddLogicDevice={onAddLogicDevice} />
      <Legend />
    </div>
  );
}

interface MindMapCanvasProps {
  onAddLogicDevice: () => void;
}

export function MindMapCanvas({ onAddLogicDevice }: MindMapCanvasProps) {
  return (
    <ReactFlowProvider>
      <MindMapCanvasInner onAddLogicDevice={onAddLogicDevice} />
    </ReactFlowProvider>
  );
}
