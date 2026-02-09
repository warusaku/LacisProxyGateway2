// CelestialGlobe v2 — UI State Store
// mobes2.0 useUIStateStore.ts (~400行) 準拠
// SSoT: 選択、コンテキストメニュー、ドラッグ、ハイライト、レイアウト状態

import { create } from 'zustand';

// ============================================================================
// Types
// ============================================================================

export interface ContextMenuState {
  isOpen: boolean;
  x: number;
  y: number;
  nodeId?: string;
  edgeId?: string;
}

export type DragMode = 'reparent' | 'free' | null;

export interface UIState {
  // 選択
  selectedNodeId: string | null;
  selectedNodeIds: string[];
  selectOnly: (ids: string[]) => void;
  toggleSelection: (ids: string[]) => void;
  clearSelection: () => void;

  // コンテキストメニュー
  contextMenu: ContextMenuState;
  openContextMenu: (x: number, y: number, nodeId?: string, edgeId?: string) => void;
  closeContextMenu: () => void;

  // ドラッグ状態
  draggedNodeIds: string[];
  dropParentNodeId: string | null;
  dragMode: DragMode;
  setDraggingState: (nodeIds: string[], mode: 'reparent' | 'free') => void;
  setDropTarget: (nodeId: string | null) => void;
  clearDraggingState: () => void;

  // ハイライト
  highlightedNodeIds: string[];
  highlightedEdgeIds: string[];
  setHighlights: (nodeIds: string[], edgeIds: string[]) => void;
  clearHighlights: () => void;

  // レイアウト
  isLayouting: boolean;
  setIsLayouting: (v: boolean) => void;
}

// ============================================================================
// Store
// ============================================================================

export const useUIStateStore = create<UIState>((set) => ({
  // 選択
  selectedNodeId: null,
  selectedNodeIds: [],

  selectOnly: (ids: string[]) =>
    set({
      selectedNodeIds: ids,
      selectedNodeId: ids.length === 1 ? ids[0] : ids.length > 0 ? ids[0] : null,
    }),

  toggleSelection: (ids: string[]) =>
    set((state) => {
      const current = new Set(state.selectedNodeIds);
      for (const id of ids) {
        if (current.has(id)) {
          current.delete(id);
        } else {
          current.add(id);
        }
      }
      const arr = Array.from(current);
      return {
        selectedNodeIds: arr,
        selectedNodeId: arr.length === 1 ? arr[0] : arr.length > 0 ? arr[0] : null,
      };
    }),

  clearSelection: () =>
    set({
      selectedNodeIds: [],
      selectedNodeId: null,
    }),

  // コンテキストメニュー
  contextMenu: { isOpen: false, x: 0, y: 0 },

  openContextMenu: (x: number, y: number, nodeId?: string, edgeId?: string) =>
    set({
      contextMenu: { isOpen: true, x, y, nodeId, edgeId },
    }),

  closeContextMenu: () =>
    set({
      contextMenu: { isOpen: false, x: 0, y: 0 },
    }),

  // ドラッグ状態
  draggedNodeIds: [],
  dropParentNodeId: null,
  dragMode: null,

  setDraggingState: (nodeIds: string[], mode: 'reparent' | 'free') =>
    set({
      draggedNodeIds: nodeIds,
      dragMode: mode,
    }),

  setDropTarget: (nodeId: string | null) =>
    set({ dropParentNodeId: nodeId }),

  clearDraggingState: () =>
    set({
      draggedNodeIds: [],
      dropParentNodeId: null,
      dragMode: null,
    }),

  // ハイライト
  highlightedNodeIds: [],
  highlightedEdgeIds: [],

  setHighlights: (nodeIds: string[], edgeIds: string[]) =>
    set({
      highlightedNodeIds: nodeIds,
      highlightedEdgeIds: edgeIds,
    }),

  clearHighlights: () =>
    set({
      highlightedNodeIds: [],
      highlightedEdgeIds: [],
    }),

  // レイアウト
  isLayouting: false,

  setIsLayouting: (v: boolean) =>
    set({ isLayouting: v }),
}));
