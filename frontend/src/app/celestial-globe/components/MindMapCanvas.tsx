'use client';

/**
 * MindMapCanvas Component
 *
 * ReactFlow ベースのトポロジーキャンバス
 * mobes2.0 TopologyCanvas.tsx 準拠:
 *   - nodeTypes = { device: DeviceNode }
 *   - edgeTypes = { topology: TopologyEdge }  ← CRITICAL: カスタムエッジ登録
 *   - defaultEdgeOptions = { type: 'topology' }
 *   - Background: variant=Dots, gap=20, size=1
 *   - MiniMap: nodeStrokeWidth=2, zoomable, pannable
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
import { TopologyEdge } from './TopologyEdge';
import { Toolbar } from './Toolbar';
import { Legend } from './Legend';
import { useTopologyStore } from '../stores/useTopologyStore';
import type { TopologyNodeV2, TopologyEdgeV2, DeviceNodeData, EdgeType, TopologyEdgeData } from '../types';

// mobes2.0 準拠: nodeTypes + edgeTypes 登録
const nodeTypes: NodeTypes = {
  device: DeviceNode,
};

const edgeTypes: EdgeTypes = {
  topology: TopologyEdge,
};

// mobes2.0 準拠: defaultEdgeOptions — 全エッジに type='topology' を適用
const defaultEdgeOptions = {
  type: 'topology',
  animated: false,
};

function toFlowNodes(
  topoNodes: TopologyNodeV2[],
  selectedId: string | null,
  onCollapse: (id: string) => void,
  onLabelEdit: (nodeId: string, newLabel: string) => void
): Node<DeviceNodeData>[] {
  return topoNodes.map(n => ({
    id: n.id,
    type: 'device',
    position: { x: n.position.x, y: n.position.y },
    data: {
      node: n,
      selected: n.id === selectedId,
      onCollapse,
      onLabelEdit,
    },
    draggable: true,
  }));
}

// mobes2.0 準拠: エッジに data.connectionType を付与（TopologyEdge コンポーネントが参照）
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
  useMemo(() => {
    setNodes(flowNodes);
  }, [flowNodes, setNodes]);

  useMemo(() => {
    setEdges(flowEdges);
  }, [flowEdges, setEdges]);

  const onNodeDragStop: NodeDragHandler = useCallback((_event, node) => {
    updateNodePosition(node.id, node.position.x, node.position.y);
  }, [updateNodePosition]);

  const onNodeClick: NodeMouseHandler = useCallback((_event, node) => {
    setSelectedNodeId(node.id);
  }, [setSelectedNodeId]);

  const onPaneClick = useCallback(() => {
    setSelectedNodeId(null);
  }, [setSelectedNodeId]);

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
        {/* mobes2.0 準拠: gap=20, size=1 (ズーム時にグリッド粒度が変化) */}
        <Background color="#333" gap={20} size={1} variant={BackgroundVariant.Dots} />
        {/* mobes2.0 準拠: nodeStrokeWidth=2, zoomable, pannable */}
        <MiniMap
          nodeColor={(n) => {
            const data = n.data as DeviceNodeData | undefined;
            if (!data) return '#333';
            const status = data.node.status;
            if (status === 'online' || status === 'active') return '#10B981';
            if (status === 'offline' || status === 'inactive') return '#EF4444';
            if (status === 'warning') return '#F59E0B';
            return '#9CA3AF';
          }}
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
