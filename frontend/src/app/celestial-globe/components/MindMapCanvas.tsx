// CelestialGlobe v2 — MindMapCanvas (Phase1最大ファイル)
// mobes2.0 MindMapCanvas.tsx (1107行) 準拠
// ReactFlow キャンバス、レイアウト、ドラッグ&ドロップ、LOD、イベントハンドラ

'use client';

import React, { useCallback, useEffect, useRef, useMemo, useState } from 'react';
import ReactFlow, {
  Background,
  Controls,
  MiniMap,
  ConnectionMode,
  useNodesState,
  useEdgesState,
  useReactFlow,
} from 'reactflow';
import type {
  Node,
  Edge,
  NodeMouseHandler,
  OnSelectionChangeFunc,
  NodeDragHandler,
} from 'reactflow';
import 'reactflow/dist/style.css';

import type { TopologyNodeV2, TopologyEdgeV2, EdgeType } from '../types';
import { STATUS_COLORS, EDGE_STYLES } from '../constants';
import { layoutTree } from '../lib/layoutTree';
import { bindLodSwitch } from '../lib/lodSwitch';
import { useTopologyStore } from '../stores/useTopologyStore';
import { useUIStateStore } from '../stores/useUIStateStore';
import { DeviceNode } from './DeviceNode';
import { InternetNode } from './InternetNode';
import { TopologyEdge } from './TopologyEdge';
import { DragGuideOverlay } from './DragGuideOverlay';
import { ContextMenu } from './ContextMenu';
import { CanvasToolbar } from './CanvasToolbar';

// ============================================================================
// Node/Edge type registrations
// ============================================================================

const nodeTypes = {
  device: DeviceNode,
  internet: InternetNode,
};

const edgeTypes = {
  topology: TopologyEdge,
};

// ============================================================================
// Convert topology data to ReactFlow nodes/edges
// ============================================================================

function buildFlowEdges(edges: TopologyEdgeV2[]): Edge[] {
  return edges.map((e) => ({
    id: `${e.from}-${e.to}`,
    source: e.from,
    target: e.to,
    type: 'topology',
    data: {
      connectionType: e.edge_type,
      label: e.label,
    },
  }));
}

// ============================================================================
// MindMapCanvas Component
// ============================================================================

export function MindMapCanvas() {
  const canvasRef = useRef<HTMLDivElement>(null);
  const reactFlowInstance = useReactFlow();

  // Topology store
  const topoNodes = useTopologyStore(s => s.nodes);
  const topoEdges = useTopologyStore(s => s.edges);
  const loading = useTopologyStore(s => s.loading);
  const fetchTopology = useTopologyStore(s => s.fetchTopology);
  const updateParent = useTopologyStore(s => s.updateParent);
  const updateNodeLabel = useTopologyStore(s => s.updateNodeLabel);
  const toggleCollapse = useTopologyStore(s => s.toggleCollapse);

  // UI state
  const selectOnly = useUIStateStore(s => s.selectOnly);
  const clearSelection = useUIStateStore(s => s.clearSelection);
  const openContextMenu = useUIStateStore(s => s.openContextMenu);
  const setDraggingState = useUIStateStore(s => s.setDraggingState);
  const setDropTarget = useUIStateStore(s => s.setDropTarget);
  const clearDraggingState = useUIStateStore(s => s.clearDraggingState);

  // ReactFlow state
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);

  // ============================================================================
  // Layout computation — layoutTree
  // ============================================================================

  useEffect(() => {
    if (topoNodes.length === 0) return;

    const { nodes: layoutNodes } = layoutTree(topoNodes);
    const flowEdges = buildFlowEdges(topoEdges);

    setNodes(layoutNodes);
    setEdges(flowEdges);
  }, [topoNodes, topoEdges, setNodes, setEdges]);

  // ============================================================================
  // Initial fetch + 30秒ポーリング
  // ============================================================================

  useEffect(() => {
    fetchTopology();
    const interval = setInterval(fetchTopology, 30_000);
    return () => clearInterval(interval);
  }, [fetchTopology]);

  // ============================================================================
  // LOD binding
  // ============================================================================

  useEffect(() => {
    const el = canvasRef.current?.querySelector('.react-flow__viewport') as HTMLElement | null;
    if (!el) return;

    const cleanup = bindLodSwitch(el, {
      getZoom: () => reactFlowInstance.getZoom(),
    });
    return cleanup;
  }, [reactFlowInstance]);

  // ============================================================================
  // Event handlers: Label editing via CustomEvent
  // ============================================================================

  useEffect(() => {
    const handleLabelEdit = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail?.nodeId && detail?.label) {
        updateNodeLabel(detail.nodeId, detail.label);
      }
    };
    document.addEventListener('cg:label-edit', handleLabelEdit);
    return () => document.removeEventListener('cg:label-edit', handleLabelEdit);
  }, [updateNodeLabel]);

  // ============================================================================
  // Node click → selection
  // ============================================================================

  const onNodeClick: NodeMouseHandler = useCallback((_event, node) => {
    selectOnly([node.id]);
  }, [selectOnly]);

  // ============================================================================
  // Pane click → deselect
  // ============================================================================

  const onPaneClick = useCallback(() => {
    clearSelection();
  }, [clearSelection]);

  // ============================================================================
  // Node context menu
  // ============================================================================

  const onNodeContextMenu: NodeMouseHandler = useCallback((event, node) => {
    event.preventDefault();
    openContextMenu(event.clientX, event.clientY, node.id);
  }, [openContextMenu]);

  // ============================================================================
  // Pane context menu
  // ============================================================================

  const onPaneContextMenu = useCallback((event: React.MouseEvent) => {
    event.preventDefault();
    openContextMenu(event.clientX, event.clientY);
  }, [openContextMenu]);

  // ============================================================================
  // Drag & Drop (reparent)
  // ============================================================================

  const dragStartPosRef = useRef<{ x: number; y: number } | null>(null);

  const onNodeDragStart: NodeDragHandler = useCallback((_event, node) => {
    dragStartPosRef.current = { x: node.position.x, y: node.position.y };
    setDraggingState([node.id], 'reparent');
  }, [setDraggingState]);

  const onNodeDrag: NodeDragHandler = useCallback((_event, node) => {
    // Find nearest potential parent
    const currentNodes = reactFlowInstance.getNodes();
    let nearestId: string | null = null;
    let nearestDist = 120; // drop hit radius

    for (const n of currentNodes) {
      if (n.id === node.id) continue;
      if (n.type === 'internet') continue;

      const dx = n.position.x - node.position.x;
      const dy = n.position.y - node.position.y;
      const dist = Math.sqrt(dx * dx + dy * dy);

      if (dist < nearestDist) {
        nearestDist = dist;
        nearestId = n.id;
      }
    }

    setDropTarget(nearestId);
  }, [reactFlowInstance, setDropTarget]);

  const onNodeDragStop: NodeDragHandler = useCallback((_event, node) => {
    const dropTarget = useUIStateStore.getState().dropParentNodeId;
    // Capture position BEFORE any state updates — React 18 batching may
    // defer setNodes updater execution, by which time the ref would be null.
    const originalPos = dragStartPosRef.current;
    dragStartPosRef.current = null;

    if (dropTarget && dropTarget !== node.id) {
      // Reparent
      const topoNode = topoNodes.find(n => n.id === node.id);
      if (topoNode && topoNode.parent_id !== dropTarget) {
        updateParent(node.id, dropTarget);
      }
    } else if (originalPos) {
      // Return to original position
      setNodes(nds =>
        nds.map(n =>
          n.id === node.id
            ? { ...n, position: originalPos }
            : n
        )
      );
    }

    clearDraggingState();
  }, [topoNodes, updateParent, clearDraggingState, setNodes]);

  // ============================================================================
  // Selection change sync
  // ============================================================================

  const onSelectionChange: OnSelectionChangeFunc = useCallback(({ nodes: selectedNodes }) => {
    if (selectedNodes.length > 0) {
      const newIds = selectedNodes.map(n => n.id);
      const current = useUIStateStore.getState().selectedNodeIds;
      // Avoid re-setting same selection (prevents unnecessary re-render cascade)
      if (
        newIds.length !== current.length ||
        newIds.some((id, i) => id !== current[i])
      ) {
        selectOnly(newIds);
      }
    }
  }, [selectOnly]);

  // ============================================================================
  // MiniMap node color
  // ============================================================================

  const miniMapNodeColor = useCallback((node: Node) => {
    const topoNode = topoNodes.find(n => n.id === node.id);
    if (!topoNode) return '#6B7280';
    const status = topoNode.state_type || topoNode.status;
    return STATUS_COLORS[status] ?? '#6B7280';
  }, [topoNodes]);

  // ============================================================================
  // Add device handler
  // ============================================================================

  const handleAddDevice = useCallback((type: string) => {
    const event = new CustomEvent('cg:add-device-type', {
      detail: { type },
      bubbles: true,
    });
    document.dispatchEvent(event);
  }, []);

  // ============================================================================
  // Render
  // ============================================================================

  return (
    <div ref={canvasRef} className="relative w-full h-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodeClick={onNodeClick}
        onNodeContextMenu={onNodeContextMenu}
        onNodeDragStart={onNodeDragStart}
        onNodeDrag={onNodeDrag}
        onNodeDragStop={onNodeDragStop}
        onPaneClick={onPaneClick}
        onPaneContextMenu={onPaneContextMenu}
        onSelectionChange={onSelectionChange}
        minZoom={0.1}
        maxZoom={2}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        connectionMode={ConnectionMode.Loose}
        panOnDrag={[0]}
        snapToGrid
        snapGrid={[16, 16]}
        proOptions={{ hideAttribution: true }}
      >
        <Background gap={24} size={1} color="#334155" />
        <Controls position="bottom-left" />
        <MiniMap
          pannable
          zoomable
          position="bottom-right"
          nodeColor={miniMapNodeColor}
          maskColor="rgba(0, 0, 0, 0.7)"
          className="!bg-transparent"
        />
      </ReactFlow>

      {/* Overlays */}
      <DragGuideOverlay />
      <ContextMenu />
      <CanvasToolbar onAddDevice={handleAddDevice} />

      {/* Loading overlay */}
      {loading && nodes.length === 0 && (
        <div className="absolute inset-0 flex items-center justify-center bg-black/40 z-50">
          <div className="flex flex-col items-center gap-3">
            <div className="w-8 h-8 border-2 border-blue-400 border-t-transparent rounded-full animate-spin" />
            <span className="text-sm text-gray-400">Loading topology...</span>
          </div>
        </div>
      )}
    </div>
  );
}
