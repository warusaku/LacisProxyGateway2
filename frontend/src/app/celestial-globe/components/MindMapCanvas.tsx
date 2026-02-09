'use client';

/**
 * MindMapCanvas Component
 *
 * ReactFlow ベースのトポロジーキャンバス
 * SSOT: mobes2.0 TopologyCanvas.tsx を LPG2 向けに完全移植
 *
 * mobes2.0 構造準拠:
 *   nodeTypes = { device: DeviceNode, internet: InternetNode }
 *     - InternetNode: source handle のみ（逆進禁止を構造的に強制）
 *     - DeviceNode: target (top) + source (bottom)
 *
 *   edgeTypes = { topology: TopologyEdge }
 *     - data.connectionType でスタイル決定
 *     - getBezierPath + path 直接描画
 *
 *   defaultEdgeOptions = { type: 'topology' }
 *     - 全エッジにカスタム TopologyEdge を適用
 *
 *   エッジ方向ルール:
 *     source = 親ノード (Handle source = bottom)
 *     target = 子ノード (Handle target = top)
 *     → 親→子の一方向フローをReactFlowの接続モデルで強制
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
  type NodeDragHandler,
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

/**
 * トポロジーノード → ReactFlow ノード変換
 *
 * internet ノードは InternetNode コンポーネント（source handle のみ）に分離
 * それ以外は DeviceNode コンポーネント（target + source handle）
 */
function toFlowNodes(
  topoNodes: TopologyNodeV2[],
  selectedId: string | null,
  onCollapse: (id: string) => void,
  onLabelEdit: (nodeId: string, newLabel: string) => void
): Node[] {
  return topoNodes.map(n => {
    // InternetNode: 独立コンポーネント（source-only handle で逆進禁止）
    if (n.node_type === 'internet') {
      return {
        id: n.id,
        type: 'internet',
        position: { x: n.position.x, y: n.position.y },
        data: {
          label: n.label,
          ip: n.ip,
        } satisfies InternetNodeData,
        draggable: true,
      };
    }

    // DeviceNode: target (top) + source (bottom) handle
    return {
      id: n.id,
      type: 'device',
      position: { x: n.position.x, y: n.position.y },
      data: {
        node: n,
        selected: n.id === selectedId,
        onCollapse,
        onLabelEdit,
      } satisfies DeviceNodeData,
      draggable: true,
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
    source: e.from,   // 親ノード（Handle type="source" = bottom）
    target: e.to,     // 子ノード（Handle type="target" = top）
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
  const updateNodePosition = useTopologyStore(s => s.updateNodePosition);
  const toggleCollapse = useTopologyStore(s => s.toggleCollapse);
  const updateNodeLabel = useTopologyStore(s => s.updateNodeLabel);

  const onCollapse = useCallback((id: string) => {
    toggleCollapse(id);
  }, [toggleCollapse]);

  const onLabelEdit = useCallback((nodeId: string, newLabel: string) => {
    updateNodeLabel(nodeId, newLabel);
  }, [updateNodeLabel]);

  const flowNodes = useMemo(
    () => toFlowNodes(topoNodes, selectedNodeId, onCollapse, onLabelEdit),
    [topoNodes, selectedNodeId, onCollapse, onLabelEdit]
  );
  const flowEdges = useMemo(() => toFlowEdges(topoEdges), [topoEdges]);

  const [nodes, setNodes, onNodesChange] = useNodesState(flowNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(flowEdges);

  // Sync when store data changes
  useMemo(() => { setNodes(flowNodes); }, [flowNodes, setNodes]);
  useMemo(() => { setEdges(flowEdges); }, [flowEdges, setEdges]);

  const onNodeDragStop: NodeDragHandler = useCallback((_event, node) => {
    updateNodePosition(node.id, node.position.x, node.position.y);
  }, [updateNodePosition]);

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
    const status = data.node.status;
    if (status === 'online' || status === 'active') return '#10B981';
    if (status === 'offline' || status === 'inactive') return '#EF4444';
    if (status === 'warning') return '#F59E0B';
    return '#9CA3AF';
  }, []);

  return (
    <div className="cg-canvas" style={{ width: '100%', height: '100%', position: 'relative' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeDragStop={onNodeDragStop}
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
