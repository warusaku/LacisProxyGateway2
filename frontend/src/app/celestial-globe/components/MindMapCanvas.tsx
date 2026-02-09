'use client';

import { useCallback, useMemo, useState } from 'react';
import ReactFlow, {
  Background,
  MiniMap,
  ReactFlowProvider,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
  type NodeTypes,
  type NodeDragHandler,
  type NodeMouseHandler,
  BackgroundVariant,
} from 'reactflow';
import 'reactflow/dist/style.css';

import { DeviceNode } from './DeviceNode';
import { Toolbar } from './Toolbar';
import { useTopologyStore } from '../stores/useTopologyStore';
import { EDGE_STYLES } from '../constants';
import type { TopologyNodeV2, TopologyEdgeV2, DeviceNodeData, EdgeType } from '../types';

const nodeTypes: NodeTypes = {
  device: DeviceNode,
};

function toFlowNodes(
  topoNodes: TopologyNodeV2[],
  selectedId: string | null,
  onCollapse: (id: string) => void
): Node<DeviceNodeData>[] {
  return topoNodes.map(n => ({
    id: n.id,
    type: 'device',
    position: { x: n.position.x, y: n.position.y },
    data: {
      node: n,
      selected: n.id === selectedId,
      onCollapse,
    },
    draggable: true,
  }));
}

function toFlowEdges(topoEdges: TopologyEdgeV2[]): Edge[] {
  return topoEdges.map((e, i) => {
    const style = EDGE_STYLES[e.edge_type as EdgeType] || EDGE_STYLES.wired;
    return {
      id: `edge-${i}-${e.from}-${e.to}`,
      source: e.from,
      target: e.to,
      type: 'default',
      label: e.label || undefined,
      style: {
        stroke: style.color,
        strokeWidth: style.strokeWidth,
        strokeDasharray: style.strokeDasharray,
      },
      animated: style.animated,
      labelStyle: { fill: '#6B7280', fontSize: 9 },
      labelBgStyle: { fill: '#0a0a0a', fillOpacity: 0.8 },
    };
  });
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

  const onCollapse = useCallback((id: string) => {
    toggleCollapse(id);
  }, [toggleCollapse]);

  const flowNodes = useMemo(
    () => toFlowNodes(topoNodes, selectedNodeId, onCollapse),
    [topoNodes, selectedNodeId, onCollapse]
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
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.1}
        maxZoom={2}
        proOptions={{ hideAttribution: true }}
      >
        <Background color="#1a1a1a" gap={40} variant={BackgroundVariant.Dots} />
        <MiniMap
          nodeColor={(n) => {
            const data = n.data as DeviceNodeData | undefined;
            if (!data) return '#333';
            const status = data.node.status;
            if (status === 'online' || status === 'active') return '#10B981';
            if (status === 'offline' || status === 'inactive') return '#6B7280';
            return '#F59E0B';
          }}
          maskColor="rgba(0,0,0,0.7)"
          style={{ background: 'rgba(10,10,10,0.9)', border: '1px solid rgba(51,51,51,0.5)', borderRadius: 8 }}
        />
      </ReactFlow>
      <Toolbar onAddLogicDevice={onAddLogicDevice} />
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
