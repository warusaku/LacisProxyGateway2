// CelestialGlobe v2 — Zustand Topology Store
// SSoT: Single store for all topology state management
// Layout is computed deterministically on the frontend from (parent_id, order).
// No position persistence or server-side layout computation.

import { create } from 'zustand';
import type {
  TopologyStoreState,
  TopologyNodeV2,
  TopologyEdgeV2,
  TopologyMetadataV2,
  ViewConfig,
  ViewMode,
  TopologyViewFilter,
  CreateLogicDeviceRequest,
  UpdateLogicDeviceRequest,
} from '../types';
import { topologyV2Api } from '@/lib/api';

// ============================================================================
// Store
// ============================================================================

export const useTopologyStore = create<TopologyStoreState>((set, get) => ({
  // Data
  nodes: [],
  edges: [],
  metadata: null,
  viewConfig: null,

  // UI state
  selectedNodeId: null,
  viewMode: 'split',
  viewFilter: 'full',
  siteFilter: null,
  loading: false,
  error: null,

  // Actions

  fetchTopology: async () => {
    const { viewFilter, siteFilter } = get();
    set({ loading: true, error: null });
    try {
      const data = await topologyV2Api.getTopology(viewFilter, siteFilter ?? undefined);
      set({
        nodes: data.nodes,
        edges: data.edges,
        metadata: data.metadata,
        viewConfig: data.view_config,
        loading: false,
      });
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : 'Failed to load topology',
        loading: false,
      });
    }
  },

  toggleCollapse: async (nodeId: string) => {
    const node = get().nodes.find(n => n.id === nodeId);
    if (!node) return;
    const newCollapsed = !node.collapsed;
    // Optimistic update
    set(state => ({
      nodes: state.nodes.map(n =>
        n.id === nodeId ? { ...n, collapsed: newCollapsed } : n
      ),
    }));
    try {
      await topologyV2Api.toggleCollapse(nodeId, newCollapsed);
      // Refetch to get proper visibility updates
      await get().fetchTopology();
    } catch (e) {
      console.error('Failed to toggle collapse:', e);
    }
  },

  updateParent: async (nodeId: string, newParentId: string) => {
    set({ loading: true, error: null });
    try {
      await topologyV2Api.updateParent(nodeId, newParentId);
      await get().fetchTopology();
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : 'Parent update failed',
        loading: false,
      });
    }
  },

  createLogicDevice: async (req: CreateLogicDeviceRequest) => {
    set({ loading: true, error: null });
    try {
      await topologyV2Api.createLogicDevice(req);
      await get().fetchTopology();
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : 'LogicDevice creation failed',
        loading: false,
      });
    }
  },

  updateLogicDevice: async (id: string, req: UpdateLogicDeviceRequest) => {
    set({ loading: true, error: null });
    try {
      await topologyV2Api.updateLogicDevice(id, req);
      await get().fetchTopology();
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : 'LogicDevice update failed',
        loading: false,
      });
    }
  },

  deleteLogicDevice: async (id: string) => {
    set({ loading: true, error: null });
    try {
      await topologyV2Api.deleteLogicDevice(id);
      await get().fetchTopology();
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : 'LogicDevice deletion failed',
        loading: false,
      });
    }
  },

  updateNodeLabel: async (nodeId: string, label: string) => {
    // Optimistic update
    set(state => ({
      nodes: state.nodes.map(n =>
        n.id === nodeId ? { ...n, label } : n
      ),
    }));
    try {
      await topologyV2Api.updateNodeLabel(nodeId, label);
    } catch (e) {
      console.error('Failed to update label:', e);
      // Revert by refetching
      await get().fetchTopology();
    }
  },

  setSelectedNodeId: (id: string | null) => {
    set({ selectedNodeId: id });
  },

  setViewMode: (mode: ViewMode) => {
    set({ viewMode: mode });
  },

  setViewFilter: (filter: TopologyViewFilter, siteFilter?: string) => {
    set({ viewFilter: filter, siteFilter: siteFilter ?? null });
    get().fetchTopology();
  },
}));
