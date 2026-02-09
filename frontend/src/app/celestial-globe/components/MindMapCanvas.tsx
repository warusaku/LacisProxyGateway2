'use client';

/**
 * MindMapCanvas Component
 *
 * ReactFlow ベースのトポロジーキャンバス
 * SSOT: mobes2.0 TopologyCanvas.tsx を LPG2 向けに完全移植
 *
 * レイアウト:
 *   位置 = f(parent_id, order) — フロントエンドDFS O(n) で決定論的に計算
 *   バックエンドは位置データを一切返さない/永続化しない
 *   ドラッグ無効 (draggable: false)
 *
 * mobes2.0 構造準拠:
 *   nodeTypes = { device: DeviceNode, internet: InternetNode }
 *     - InternetNode: source handle (right) のみ（逆進禁止を構造的に強制）
 *     - DeviceNode: target (left) + source (right) — 左→右ツリー
 *
 *   edgeTypes = { topology: TopologyEdge }
 *     - data.connectionType でスタイル決定
 *     - getSmoothStepPath + path 直接描画（直角折れ線）
 *
 *   defaultEdgeOptions = { type: 'topology' }
 *     - 全エッジにカスタム TopologyEdge を適用
 *
 *   エッジ方向ルール:
 *     source = 親ノード (Handle source = right)
 *     target = 子ノード (Handle target = left)
 *     → 親→子の左→右一方向フローをReactFlowの接続モデルで強制
 *
 *   Background: variant=Dots, gap=20, size=1
 *   MiniMap: nodeStrokeWidth=2, zoomable, pannable
 */

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

// mobes2.0 準拠: nodeTypes — device + internet の2種登録
const nodeTypes: NodeTypes = {
  device: DeviceNode,
  internet: InternetNode,
};

// mobes2.0 準拠: edgeTypes — カスタム TopologyEdge 登録
const edgeTypes: EdgeTypes = {
  topology: TopologyEdge,
};

// mobes2.0 準拠: 全エッジに type='topology' を適用
const defaultEdgeOptions = {
  type: 'topology',
  animated: false,
};

// ============================================================================
// DFS deterministic layout: position = f(parent_id, order)
// ============================================================================

const H_SPACING = 280;  // horizontal spacing per depth level (px)
const V_GAP = 16;       // minimum vertical gap between siblings (px)

/** Get node height based on node_type */
function getNodeHeight(nodeType?: string): number {
  switch (nodeType) {
    case 'internet': return 60;
    case 'gateway':
    case 'router':
    case 'switch':
    case 'ap':
    case 'external':
    case 'lpg_server':
    case 'logic_device':
      return 80;
    default: return 56; // client, wg_peer
  }
}

/**
 * Compute deterministic DFS layout from topology nodes.
 * Each node's position is determined solely by (parent_id, order).
 * O(n) time complexity — each node visited exactly once.
 */
function computeDfsLayout(nodes: TopologyNodeV2[]): Map<string, { x: number; y: number }> {
  // Build children map (parent_id → children sorted by order)
  const childrenMap = new Map<string, TopologyNodeV2[]>();
  const nodeById = new Map<string, TopologyNodeV2>();

  for (const n of nodes) {
    nodeById.set(n.id, n);
    const pid = n.parent_id || '__internet__';
    if (pid === n.id) continue; // Skip self-referencing (__internet__ node)
    if (!childrenMap.has(pid)) childrenMap.set(pid, []);
    childrenMap.get(pid)!.push(n);
  }

  // Sort children by order
  childrenMap.forEach((children) => {
    children.sort((a: TopologyNodeV2, b: TopologyNodeV2) => a.order - b.order);
  });

  const positions = new Map<string, { x: number; y: number }>();
  let cursorY = 0;

  // DFS: leaves get cursorY, parents get center of children
  function dfs(nodeId: string, depth: number): [number, number] {
    const children = childrenMap.get(nodeId) || [];
    const x = depth * H_SPACING;
    const node = nodeById.get(nodeId);
    const h = getNodeHeight(node?.node_type);

    if (children.length === 0) {
      // Leaf: place at current cursor
      positions.set(nodeId, { x, y: cursorY });
      const startY = cursorY;
      cursorY += h + V_GAP;
      return [startY, startY + h];
    }

    // Parent: recurse into children first, then center
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

  // Start from __internet__ (virtual root)
  dfs('__internet__', 0);

  return positions;
}

/**
 * トポロジーノード → ReactFlow ノード変換
 * Position is computed by DFS layout, not from backend data.
 */
function toFlowNodes(
  topoNodes: TopologyNodeV2[],
  positions: Map<string, { x: number; y: number }>,
  selectedId: string | null,
  onCollapse: (id: string) => void,
  onLabelEdit: (nodeId: string, newLabel: string) => void
): Node[] {
  return topoNodes.map(n => {
    const pos = positions.get(n.id) || { x: 0, y: 0 };

    // InternetNode: 独立コンポーネント（source-only handle で逆進禁止）
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

    // DeviceNode: target (left) + source (right) handle
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

/**
 * トポロジーエッジ → ReactFlow エッジ変換
 *
 * エッジ方向: source = 親ノード (from), target = 子ノード (to)
 * → ReactFlow の接続モデルで親→子の一方向フローを強制
 * → InternetNode は source handle のみなので、逆方向のエッジは接続不可能
 *
 * data.connectionType: TopologyEdge コンポーネントがスタイルを決定するために使用
 */
function toFlowEdges(topoEdges: TopologyEdgeV2[]): Edge[] {
  return topoEdges.map((e, i) => ({
    id: `edge-${i}-${e.from}-${e.to}`,
    source: e.from,   // 親ノード（Handle type="source" = right）
    target: e.to,     // 子ノード（Handle type="target" = left）
    type: 'topology',
    data: {
      connectionType: e.edge_type as EdgeType,
      label: e.label,
      animated: e.edge_type === 'wireless' || e.edge_type === 'route',
    } satisfies TopologyEdgeData,
  }));
}

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

  // Compute DFS layout from (parent_id, order)
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

  // mobes2.0 準拠: MiniMap ノード色関数
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
        {/* mobes2.0 準拠: gap=20, size=1 — ズーム時にグリッド粒度が変化 */}
        <Background color="#333" gap={20} size={1} variant={BackgroundVariant.Dots} />
        {/* mobes2.0 準拠: nodeStrokeWidth=2, zoomable, pannable */}
        <MiniMap
          nodeColor={getNodeColor}
          nodeStrokeWidth={2}
          zoomable
          pannable
          maskColor="rgba(0,0,0,0.7)"
          style={{ background: 'rgba(10,10,10,0.9)', border: '1px solid rgba(51,51,51,0.5)', borderRadius: 8 }}
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
